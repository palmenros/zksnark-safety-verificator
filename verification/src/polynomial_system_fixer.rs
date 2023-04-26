// TODO: Create a function that takes a polynomial system and the signals to be fixed, simplify
//  it using Gauss-Jordan and finally generate Cocoa5 code for solving the Groebner Basis

use crate::input_data::SignalIndex;
use crate::verifier::PolynomialSystemFixedSignal;
use crate::InputDataContextView;
use circom_algebra::algebra::{ArithmeticExpression, Constraint};
use indoc::formatdoc;
use itertools::Itertools;
use num_bigint_dig::BigInt;
use num_traits::One;
use std::collections::{BTreeSet, HashMap};
use std::iter;

// This enum controls how each signal should be displayed: either as its name (which is human
//  readable but may cause problems with Computer Algebra Systems), or as a signal index (which is
//  not easily human readable but can be safely used by Computer Algebra Systems).
#[derive(Copy, Clone)]
enum SignalDisplayKind {
    Name,
    Index,
}

pub fn print_polynomial_system(
    pol_system: &PolynomialSystemFixedSignal,
    context: &InputDataContextView,
) {
    let display_kind = SignalDisplayKind::Name;

    println!("\nConstraints: ");

    for constraint in &pol_system.constraints {
        println!(
            "{} = 0",
            get_constraint_polynomial(constraint, context, display_kind)
        );
    }

    let signals_to_fix_name_vec: Vec<String> = pol_system
        .signals_to_fix
        .iter()
        .map(|idx| -> String { context.signal_name_map[idx].clone() })
        .collect();

    println!("Signals to fix: {:?}", signals_to_fix_name_vec);
    println!("Prohibition constraint: ");
    println!(
        "{} = 0",
        get_prohibition_witness_polynomial(&pol_system.signals_to_fix, context, display_kind)
    );

    println!("Cocoa Script: ");
    println!("{}", get_cocoa_subscript(pol_system, context));
}

// Returns a String containing a subscript in the Cocoa5 CAS system for proving that the
//  signals are fixed by the given constraints
fn get_cocoa_subscript(
    pol_system: &PolynomialSystemFixedSignal,
    context: &InputDataContextView,
) -> String {
    let mut used_signal_indices = BTreeSet::new();

    for constraint in &pol_system.constraints {
        used_signal_indices.append(&mut constraint.take_cloned_signals_ordered());
    }

    let prohibition_vars = (0..pol_system.signals_to_fix.len()).map(|i| {
        format!("u_{}", i)
    });

    let vars: String = Itertools::intersperse(used_signal_indices.iter().map(|i| {
        format!("x_{}", i)
    }).chain(prohibition_vars), ", ".to_string()).collect();

    let pols: String = Itertools::intersperse(
        pol_system
            .constraints
            .iter()
            .map(|c| -> String { get_constraint_polynomial(c, context, SignalDisplayKind::Index) })
            .chain(iter::once(get_prohibition_witness_polynomial(
                &pol_system.signals_to_fix,
                context,
                SignalDisplayKind::Index,
            ))),
        ",\n".to_string(),
    )
        .collect();

    let s = formatdoc! {"
        use R ::= F[{vars}];

        I := ideal({pols});

        println \"Is pol_system safe: \", 1 IsIn I;
    "};

    s
}

fn get_prohibition_witness_polynomial(
    signals_to_fix: &BTreeSet<SignalIndex>,
    context: &InputDataContextView,
    display_kind: SignalDisplayKind,
) -> String {
    let str: String = Itertools::intersperse(
        signals_to_fix
            .iter()
            .enumerate()
            .map(|(i, signal_idx)| -> String {
                let indexed_signal_kind = format!("x_{}", signal_idx);
                let signal_name = match display_kind {
                    SignalDisplayKind::Name => &context.signal_name_map[signal_idx],
                    SignalDisplayKind::Index => &indexed_signal_kind,
                };
                let witness_value = &context.witness[signal_idx];

                format!("(({} - {})*u_{} - 1)", signal_name, witness_value, i)
            }),
        " * ".to_string(),
    )
        .collect();

    str
}

fn get_constraint_polynomial(
    constraint: &Constraint<usize>,
    context: &InputDataContextView,
    display_kind: SignalDisplayKind,
) -> String {
    // We assume that the constraints are fixed (called Constraint::fix_constraint)
    // TODO: Check that fix_constraint is called

    let a = constraint.a();
    let b = constraint.b();
    let c = constraint.c();

    if a.is_empty() || b.is_empty() {
        //  Only linear constraint c
        linear_term_to_string(c, context, false, display_kind)
    } else {
        // TODO: Do not print the + symbol if
        let a_str = linear_term_to_string(a, context, true, display_kind);
        let b_str = linear_term_to_string(b, context, true, display_kind);
        let c_str = linear_term_to_string(c, context, false, display_kind);

        if c_str.starts_with('-') {
            format!(
                "{} * {} - {}",
                a_str,
                b_str,
                c_str.chars().skip(1).collect::<String>()
            )
        } else if c.is_empty() {
            format!("{} * {}", a_str, b_str)
        } else {
            format!("{} * {} + {}", a_str, b_str, c_str)
        }
    }
}

// Will surround with parenthesis if there is more than one summation term and surround_with_parenthesis is true
fn linear_term_to_string(
    linear_term: &HashMap<usize, BigInt>,
    context: &InputDataContextView,
    surround_with_parenthesis: bool,
    display_kind: SignalDisplayKind,
) -> String {
    if linear_term.is_empty() {
        return "0".to_string();
    }

    let prime = &context.field;

    let s: String = linear_term
        .iter()
        .sorted_by_key(|(&idx, _)| idx)
        .map(|(&signal_idx, coeff)| -> String {
            if signal_idx == ArithmeticExpression::<usize>::constant_coefficient() {
                coefficient_to_string(coeff, prime)
            } else {
                let indexed_signal_name = format!("x_{}", signal_idx);
                let signal_name = match display_kind {
                    SignalDisplayKind::Name => &context.signal_name_map[&signal_idx],
                    SignalDisplayKind::Index => &indexed_signal_name,
                };

                if coeff.is_one() {
                    signal_name.clone()
                } else if coeff.eq(&(prime - &BigInt::one())) {
                    format!("-{}", signal_name)
                } else {
                    format!("{}*{}", coefficient_to_string(coeff, prime), signal_name)
                }
            }
        })
        .fold("".to_string(), |curr, next| -> String {
            if curr.is_empty() {
                next
            } else if next.starts_with('-') {
                format!("{} - {}", curr, next.chars().skip(1).collect::<String>())
            } else {
                format!("{} + {}", curr, next)
            }
        });

    if surround_with_parenthesis && linear_term.len() > 1 {
        format!("({})", s)
    } else {
        s
    }
}

// Returns a prettified string of the given coefficient
fn coefficient_to_string(coeff: &BigInt, prime_field: &BigInt) -> String {
    if coeff > &(prime_field / 2) {
        format!("-{}", (prime_field - coeff))
    } else {
        coeff.to_string()
    }
}
