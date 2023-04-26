// TODO: Create a function that takes a polynomial system and the signals to be fixed, simplify
//  it using Gauss-Jordan and finally generate Cocoa5 code for solving the Groebner Basis

use crate::input_data::SignalIndex;
use crate::verifier::PolynomialSystemFixedSignal;
use crate::InputDataContextView;
use circom_algebra::algebra::{ArithmeticExpression, Constraint};
use itertools::Itertools;
use num_bigint_dig::BigInt;
use num_traits::One;
use std::collections::{BTreeSet, HashMap};

pub fn print_polynomial_system(
    pol_system: &PolynomialSystemFixedSignal,
    context: &InputDataContextView,
) {
    println!("\nConstraints: ");
    for constraint in &pol_system.constraints {
        println!("{} = 0", get_constraint_polynomial(constraint, context));
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
        get_prohibition_witness_polynomial(&pol_system.signals_to_fix, context)
    );
}

fn get_prohibition_witness_polynomial(
    signals_to_fix: &BTreeSet<SignalIndex>,
    context: &InputDataContextView,
) -> String {
    let str: String = Itertools::intersperse(
        signals_to_fix
            .iter()
            .enumerate()
            .map(|(i, signal_idx)| -> String {
                let signal_name = &context.signal_name_map[signal_idx];
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
) -> String {
    // We assume that the constraints are fixed (called Constraint::fix_constraint)
    // TODO: Check that fix_constraint is called

    let a = constraint.a();
    let b = constraint.b();
    let c = constraint.c();

    if a.is_empty() || b.is_empty() {
        //  Only linear constraint c
        linear_term_to_string(c, context, false)
    } else {
        // TODO: Do not print the + symbol if
        let a_str = linear_term_to_string(a, context, true);
        let b_str = linear_term_to_string(b, context, true);
        let c_str = linear_term_to_string(c, context, false);

        if c_str.starts_with('-') {
            format!(
                "{} * {} - {}",
                a_str,
                b_str,
                c_str.chars().skip(1).collect::<String>()
            )
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
) -> String {
    if linear_term.is_empty() {
        return "0".to_string();
    }

    let prime = &context.field;

    let s: String = linear_term
        .iter()
        .sorted_by_key(|(&idx, _)| idx)
        .map(|(&idx, coeff)| -> String {
            if idx == ArithmeticExpression::<usize>::constant_coefficient() {
                coefficient_to_string(coeff, prime)
            } else {
                let signal_name = context.signal_name_map.get(&idx).unwrap();
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
