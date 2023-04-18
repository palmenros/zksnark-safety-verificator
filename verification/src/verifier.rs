use crate::input_data::SignalIndex;
use crate::verifier::ModuleUnsafeReason::UnfixedOutputsAfterPropagation;
use crate::verifier::SubComponentVerificationResultKind::{
    Exception, ModuleConditionallySafe, ModuleUnsafe,
};
use crate::verifier::VerificationException::NoUnsafeConstraintConnectedComponentWithoutCycles;
use circom_algebra::algebra::Constraint;
use itertools::Itertools;
use std::collections::BTreeSet;
use colored::Colorize;

// This structure represents a polynomial system of constraints that should have their output fixed
pub struct PolynomialSystemFixedSignal {
    pub constraints: Vec<Constraint<usize>>,

    // Signals to fix from the constraints given above
    pub signals_to_fix: BTreeSet<SignalIndex>,
}

// Conditions that must be satisfied for this module to be considered safe
pub struct SafetyConditions {
    // Subcomponents that must also be verified for this module to be safe
    subcomponents: Vec<SubComponentVerificationResult>,

    // Polynomial systems to be fixed using Groebner Basis for this module to be safe
    pol_systems: Vec<PolynomialSystemFixedSignal>,
}

pub enum VerificationException {
    NoUnsafeConstraintConnectedComponentWithoutCycles,
}

pub enum ModuleUnsafeReason {
    // A vector of signal names have not been fixed after finishing all possible propagation
    //  and no === remaining
    UnfixedOutputsAfterPropagation(Vec<String>),
}

pub enum SubComponentVerificationResultKind {
    // This module does not fix all its outputs, for example, when we directly compute from the
    //  verification graph that the outputs are not assigned by any fixed node
    ModuleUnsafe(ModuleUnsafeReason),

    ModuleConditionallySafe(SafetyConditions),

    Exception(VerificationException),
}

pub struct SubComponentVerificationResult {
    kind: SubComponentVerificationResultKind,
    subcomponent_name: String,
}

impl SubComponentVerificationResult {
    // If this SubComponentVerificationResult is an error or exception, returns a string message
    //  describing the error. If not, returns none. Does not recurse to subcomponents.
    fn get_error_string(&self) -> Option<String> {
        match &self.kind {
            ModuleConditionallySafe(_) => None,
            ModuleUnsafe(unsafe_reason) => match unsafe_reason {
                UnfixedOutputsAfterPropagation(unfixed_outputs) => {
                    Some(format!(
                        "[Unsafe] Component '{}' is unsafe. Outputs {} are not fixed by inputs",
                        self.subcomponent_name,
                        unfixed_outputs.iter().map(|s| { format!("'{}'", s) }).join(", ")
                    ))
                }
            },
            Exception(exception) => match exception {
                NoUnsafeConstraintConnectedComponentWithoutCycles => {
                    Some(format!(
                        "[Exception] Cyclic dependencies between === constraints connected component in component '{}', cannot determine safety",
                        self.subcomponent_name
                    ))
                }
            },
        }
    }
}

impl SubComponentVerificationResult {
    pub fn apply<F>(&self, f: &mut F)
        where
            F: FnMut(&SubComponentVerificationResult),
    {
        f(self);

        if let ModuleConditionallySafe(safety_conditions) = &self.kind {
            for sub_component in &safety_conditions.subcomponents {
                sub_component.apply(f);
            }
        }
    }
}

pub fn verify() {
    let a = SubComponentVerificationResult {
        kind: ModuleConditionallySafe(SafetyConditions {
            subcomponents: vec![
                SubComponentVerificationResult {
                    kind: Exception(NoUnsafeConstraintConnectedComponentWithoutCycles),
                    subcomponent_name: "main.first".to_string(),
                },
                SubComponentVerificationResult {
                    kind: ModuleUnsafe(UnfixedOutputsAfterPropagation(vec![
                        "out1".to_string(),
                        "out2".to_string(),
                    ])),
                    subcomponent_name: "main.second".to_string(),
                },
                SubComponentVerificationResult {
                    kind: ModuleConditionallySafe(SafetyConditions {
                        subcomponents: vec![SubComponentVerificationResult {
                            kind: Exception(NoUnsafeConstraintConnectedComponentWithoutCycles),
                            subcomponent_name: "main.third.one".to_string(),
                        }],
                        pol_systems: vec![],
                    }),
                    subcomponent_name: "main.third".to_string(),
                },
            ],
            pol_systems: vec![],
        }),
        subcomponent_name: "main".to_string(),
    };

    let mut num_unsafe_found = 0;
    let mut num_exceptions_found = 0;

    a.apply(&mut |res| {
        if let Some(s) = res.get_error_string() {
            println!("{}", s.red());
        }

        match res.kind {
            ModuleUnsafe(_) => { num_unsafe_found += 1; }
            ModuleConditionallySafe(_) => {
                // TODO: Add polynomial systems to a vector to further verify
            }
            Exception(_) => { num_exceptions_found += 1; }
        }
    });

    println!("{} unsafe modules found, {} exceptions found", num_unsafe_found, num_exceptions_found);
}
