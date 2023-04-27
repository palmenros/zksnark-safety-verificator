use crate::input_data::{InputDataContextView, SignalIndex};
use crate::polynomial_system_fixer::verify_pol_systems;
use crate::verification_graph::VerificationGraph;
use crate::verifier::ModuleUnsafeReason::UnfixedOutputsAfterPropagation;
use crate::verifier::SubComponentVerificationResultKind::{
    Exception, ModuleConditionallySafe, ModuleUnsafe,
};
use crate::verifier::VerificationException::NoUnsafeConstraintConnectedComponentWithoutCycles;
use circom_algebra::algebra::Constraint;
use circom_algebra::constraint_storage::ConstraintStorage;
use colored::Colorize;
use itertools::Itertools;
use std::collections::BTreeSet;
use std::error::Error;

// This structure represents a polynomial system of constraints that should have their output fixed
#[derive(Clone)]
pub struct PolynomialSystemFixedSignal {
    pub constraints: Vec<Constraint<usize>>,

    // Signals to fix from the constraints given above
    pub signals_to_fix: BTreeSet<SignalIndex>,
}

// Conditions that must be satisfied for this module to be considered safe
pub struct SafetyConditions {
    // Subcomponents that must also be verified for this module to be safe
    pub subcomponents: Vec<SubComponentVerificationResult>,

    // Polynomial systems to be fixed using Groebner Basis for this module to be safe
    pub pol_systems: Vec<PolynomialSystemFixedSignal>,
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
    pub kind: SubComponentVerificationResultKind,
    pub subcomponent_name: String,
}

impl SubComponentVerificationResult {
    // If this SubComponentVerificationResult is an error or exception, returns a string message
    //  describing the error. If not, returns none. Does not recurse to subcomponents.
    fn get_error_string(&self) -> Option<String> {
        match &self.kind {
            ModuleConditionallySafe(_) => None,
            ModuleUnsafe(unsafe_reason) => match unsafe_reason {
                UnfixedOutputsAfterPropagation(unfixed_outputs) => {
                    if unfixed_outputs.len() == 1 {
                        Some(format!(
                            "[Unsafe] Component '{}' is unsafe. Output '{}' is not fixed by inputs",
                            self.subcomponent_name, unfixed_outputs[0]
                        ))
                    } else {
                        Some(format!(
                            "[Unsafe] Component '{}' is unsafe. Outputs {} are not fixed by inputs",
                            self.subcomponent_name,
                            unfixed_outputs
                                .iter()
                                .map(|s| { format!("'{}'", s) })
                                .join(", ")
                        ))
                    }
                }
            },
            Exception(exception) => match exception {
                NoUnsafeConstraintConnectedComponentWithoutCycles => {
                    Some(format!(
                        // TODO: Change this message, as it may not always be a cyclic dependency (see is-zero-1)
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

pub fn verify(
    context: &InputDataContextView,
    constraint_storage: &mut ConstraintStorage,
) -> Result<bool, Box<dyn Error>> {
    let mut verification_graph = VerificationGraph::new(context, constraint_storage);
    let res = verification_graph.verify_subcomponents(context, constraint_storage);

    let maybe_pol_systems = flatten_verification_result_and_report_errors(&res);
    if let Some(pol_systems) = maybe_pol_systems {
        if pol_systems.is_empty() {
            // We don't have any polynomial systems to fix using Groebner Basis, finished.
            println!(
                "{}",
                "No polynomial systems to fix. Finished. Module is safe!".green()
            );
            return Ok(true);
        } else {
            println!(
                "{}",
                "No exceptions or errors reported when traversing tree. Fixing polynomial systems...\n".green()
            );

            let res = verify_pol_systems(&pol_systems, context)?;

            if res {
                println!(
                    "{}",
                    "\nMODULE SAFE: all polynomials systems have been fixed".green()
                );
            } else {
                println!(
                    "{}",
                    "\nCouldn't fix a polynomial system. Aborting verification...".red()
                );
            }

            return Ok(res);
        }
    }

    Ok(false)
}

// Returns true if any error or exception was found. False otherwise
fn flatten_verification_result_and_report_errors(
    verification_result: &SubComponentVerificationResult,
) -> Option<Vec<PolynomialSystemFixedSignal>> {
    let mut num_unsafe_found = 0;
    let mut num_exceptions_found = 0;

    let mut polynomial_systems_to_prove = vec![];

    verification_result.apply(&mut |res| {
        if let Some(s) = res.get_error_string() {
            println!("{}", s.red());
        }

        match &res.kind {
            ModuleUnsafe(_) => {
                num_unsafe_found += 1;
            }
            ModuleConditionallySafe(safety_conditions) => {
                // Add polynomial systems to a vector to further verify
                polynomial_systems_to_prove.append(&mut safety_conditions.pol_systems.clone())
            }
            Exception(_) => {
                num_exceptions_found += 1;
            }
        }
    });

    // TODO: If there are no errors / exceptions found, give a different message to the user

    if num_unsafe_found + num_exceptions_found > 0 {
        println!(
            "{}",
            format!(
                "{} unsafe modules found, {} exceptions found on verification graph traversal. Aborting safety verification...",
                num_unsafe_found, num_exceptions_found).red()
        );
    }

    if num_unsafe_found + num_exceptions_found == 0 {
        Some(polynomial_systems_to_prove)
    } else {
        None
    }
}

#[test]
fn test_verification_result_error_printing() {
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

    flatten_verification_result_and_report_errors(&a);
}
