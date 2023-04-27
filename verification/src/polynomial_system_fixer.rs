// TODO: Create a function that takes a polynomial system and the signals to be fixed, simplify
//  it using Gauss-Jordan and finally generate Cocoa5 code for solving the Groebner Basis

use crate::input_data::SignalIndex;
use crate::verifier::PolynomialSystemFixedSignal;
use crate::InputDataContextView;
use circom_algebra::algebra::{ArithmeticExpression, Constraint};
use colored::Colorize;
use indoc::formatdoc;
use itertools::Itertools;
use num_bigint_dig::BigInt;
use num_traits::One;
use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::iter;
use std::path::Path;
use std::process::{Command, Stdio};
use which::which;

// This enum controls how each signal should be displayed: either as its name (which is human
//  readable but may cause problems with Computer Algebra Systems), or as a signal index (which is
//  not easily human readable but can be safely used by Computer Algebra Systems).
#[derive(Copy, Clone)]
enum SignalDisplayKind {
    Name,
    Index,
}

pub type PolSystemIndex = usize;

// Verifies a polynomial system generating a Cocoa5 file and executing it. Returns true if
//  verification succeeded and false otherwise.
pub fn verify_pol_systems(
    pol_systems: &[PolynomialSystemFixedSignal],
    context: &InputDataContextView,
) -> Result<bool, Box<dyn Error>> {
    assert!(!pol_systems.is_empty());

    // TODO: Add support for specifying in a command line argument the CoCoA PATH
    let maybe_cocoa_path = which("CoCoAInterpreter");
    if let Err(e) = maybe_cocoa_path {
        let error_msg = format!("Couldn't find CocoA 5 interpreter in PATH: {}", e);
        println!("{}", error_msg.red());
        return Ok(false);
    }

    let cocoa_path = maybe_cocoa_path.unwrap();
    let cocoa_base_folder = cocoa_path.parent().unwrap();
    println!("Found CoCoA at {}", cocoa_path.to_str().unwrap());

    let cocoa_file_path = Path::new(context.base_path).join("groebner.cocoa5");

    {
        // Write Cocoa file
        let mut cocoa_file = File::create(cocoa_file_path.as_path())?;
        cocoa_file.write_all(generate_cocoa_script(pol_systems, context).as_bytes())?;
        cocoa_file.flush()?;
    }

    println!("{}", cocoa_file_path.display());

    let mut child = Command::new(cocoa_path.as_path())
        .arg("--no-preamble")
        .arg(cocoa_file_path)
        .current_dir(cocoa_base_folder)
        .stdout(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().unwrap();
    let pol_systems_len = pol_systems.len();

    display_ith_pol_system_progress(pol_systems, 0, context);

    for maybe_line in BufReader::new(stdout).lines() {
        let line = maybe_line?;
        if let Some(num_str) = line.strip_prefix("OK: ") {
            let num: usize = num_str.parse()?;
            println!(
                "\n{}",
                format!(
                    "Polynomial system {}/{} has only one solution!",
                    num + 1,
                    pol_systems_len
                )
                .green()
            );

            if num + 1 < pol_systems_len {
                display_ith_pol_system_progress(pol_systems, num + 1, context);
            }
        } else if let Some(num_str) = line.strip_prefix("ERROR: ") {
            let num: usize = num_str.parse()?;
            child.kill()?;

            println!(
                "{}",
                format!(
                    "Polynomial system number {} possibly has many solutions! Aborting...",
                    num + 1
                )
                .red()
            );
            return Ok(false);
        } else if line.eq("ALL OK") {
            return Ok(true);
        } else {
            unreachable!();
        }
    }

    // TODO: Somewhere remove 0=0 constraints from the polynomial system
    // for pol_system in &pol_systems {
    //     display_polynomial_system_readable(pol_system, context);
    // }

    unreachable!()
}

fn display_ith_pol_system_progress(
    pol_systems: &[PolynomialSystemFixedSignal],
    index: usize,
    context: &InputDataContextView,
) {
    println!(
        "\n{}",
        format!(
            "Fixing polynomial system {}/{}",
            index + 1,
            pol_systems.len()
        )
        .blue()
    );
    display_polynomial_system_readable(&pol_systems[index], context);
}

pub fn generate_cocoa_script(
    pol_systems: &[PolynomialSystemFixedSignal],
    context: &InputDataContextView,
) -> String {
    let pol_systems_str: String = Itertools::intersperse(
        pol_systems
            .iter()
            .enumerate()
            .map(|(idx, pol_system)| -> String { get_cocoa_subscript(pol_system, context, idx) }),
        "\n".to_string(),
    )
    .collect();

    let field_prime = context.field.to_string();

    let s: String = formatdoc! {"
        p := {field_prime};
        use F ::= ZZ/(p);

        {pol_systems_str}

        println \"ALL OK\";
    "};

    s
}

pub fn display_polynomial_system_readable(
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
}

// Returns a String containing a subscript in the Cocoa5 CAS system for proving that the
//  signals are fixed by the given constraints
fn get_cocoa_subscript(
    pol_system: &PolynomialSystemFixedSignal,
    context: &InputDataContextView,
    pol_system_idx: PolSystemIndex,
) -> String {
    let mut used_signal_indices = BTreeSet::new();

    for constraint in &pol_system.constraints {
        used_signal_indices.append(&mut constraint.take_cloned_signals_ordered());
    }

    let prohibition_vars = (0..pol_system.signals_to_fix.len()).map(|i| format!("u_{}", i));

    let vars: String = Itertools::intersperse(
        used_signal_indices
            .iter()
            .map(|i| format!("x_{}", i))
            .chain(prohibition_vars),
        ", ".to_string(),
    )
    .collect();

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

    // TODO: Remove SleepFor from Groebner file
    let s = formatdoc! {"
        use R ::= F[{vars}];

        I := ideal({pols});

        If not(1 IsIn I) Then
            println \"ERROR: {pol_system_idx}\";
            exit;
        Else;
            println \"OK: {pol_system_idx}\";
        EndIf;
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
