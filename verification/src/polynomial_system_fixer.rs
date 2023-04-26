// TODO: Create a function that takes a polynomial system and the signals to be fixed, simplify
//  it using Gauss-Jordan and finally generate Cocoa5 code for solving the Groebner Basis

use crate::verifier::PolynomialSystemFixedSignal;
use crate::InputDataContextView;
use circom_algebra::algebra::{ArithmeticExpression, Constraint};
use itertools::Itertools;
use num_bigint_dig::BigInt;
use std::collections::HashMap;
use num_traits::One;

pub fn print_polynomial_system(
    pol_system: &PolynomialSystemFixedSignal,
    context: &InputDataContextView,
) {
    for constraint in &pol_system.constraints {
        println!("{}", constraint_to_string(constraint, context));
    }

    println!();
}

fn constraint_to_string(constraint: &Constraint<usize>, context: &InputDataContextView) -> String {
    // We assume that the constraints are fixed (called Constraint::fix_constraint)
    // TODO: Check that fix_constraint is called

    let a = constraint.a();
    let b = constraint.b();
    let c = constraint.c();

    if a.is_empty() || b.is_empty() {
        //  Only linear constraint c
        format!(
            "{} = 0",
            linear_term_to_string(c, context, false)
        )
    } else {
        format!(
            "{} * {} + {} = 0",
            linear_term_to_string(a, context, true),
            linear_term_to_string(b, context, true),
            linear_term_to_string(c, context, false)
        )
    }
}

// Will surround with parenthesis if there is more than one summation term and surround_with_parenthesis is true
fn linear_term_to_string(
    linear_term: &HashMap<usize, BigInt>,
    context: &InputDataContextView,
    surround_with_parenthesis: bool,
) -> String {
    // TODO: Maybe return Option<String> and None if there is nothing to print

    if linear_term.is_empty() {
        return String::from("0");
    }

    let prime = &context.field;

    let s: String = itertools::Itertools::intersperse(
        linear_term
            .iter()
            .sorted_by_key(|(&idx, _)| idx)
            .map(|(&idx, coeff)| -> String {
                if idx == ArithmeticExpression::<usize>::constant_coefficient() {
                    coeff.to_string()
                } else {
                    let signal_name = context.signal_name_map.get(&idx).unwrap();
                    if coeff.is_one() {
                        signal_name.clone()
                    } else if coeff.eq(&(prime - &BigInt::one())) {
                        format!("-{}", signal_name)
                    } else if coeff > &(prime / 2) {
                        // Handle negative numbers more gracefully
                        format!("-{}*{}", prime - coeff, signal_name)
                    } else {
                        format!("{}*{}", coeff, signal_name)
                    }
                }
            }),
        String::from("+"),
    )
        .collect();

    if surround_with_parenthesis && linear_term.len() > 1 {
        format!("({})", s)
    } else {
        s
    }
}
