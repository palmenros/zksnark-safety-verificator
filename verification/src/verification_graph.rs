use std::collections::{BTreeMap, BTreeSet, HashSet, LinkedList};
use num_traits::Zero;
use circom_algebra::algebra::{Constraint, Substitution};
use crate::{ComponentIndex, ConstraintIndex, InputDataContextView, SignalIndex};

#[allow(clippy::enum_variant_names)]
pub enum Node {
    InputSignal,
    OutputSignal,
    IntermediateSignal,

    SubComponentInputSignal(ComponentIndex),
    SubComponentOutputSignal(ComponentIndex),
}

#[derive(Clone)]
pub struct SafeAssignment {
    // Signal index of the signal appearing in the LHS of the '<==' assignment
    pub lhs_signal: SignalIndex,

    // Signal indices of the signals appearing in the RHS of the '==>' assignment
    pub rhs_signals: BTreeSet<SignalIndex>,

    // Constraint index of the constraint associated to the safe assignment
    pub associated_constraint: ConstraintIndex,
}

// A constraint of the type '===' that has not been generated by a safe assignment '<=='
//  TODO: Look for a better name?

#[derive(Clone)]
pub struct UnsafeConstraint {
    // List of *all* participating signals in this constraint, including the key of edge_constraints
    pub signals: BTreeSet<SignalIndex>,

    // Constraint index
    pub associated_constraint: ConstraintIndex,
}

// A subcomponent, which has input_signals and output_signals
pub struct SubComponent {
    pub input_signals: BTreeSet<SignalIndex>,
    pub output_signals: BTreeSet<SignalIndex>,

    pub not_yet_fixed_inputs: BTreeSet<SignalIndex>,
}

pub type SafeAssignmentIndex = usize;
pub type UnsafeConstraintIndex = usize;


// NOTE: For reproducibility, I have declared the HashMaps as BTreeMap, so they are ordered.
//      Explore whether it's a good idea to change them to HashMap
pub struct VerificationGraph {
    // List of nodes in the graph. Each signal has a node, and it can be of multiple types
    pub nodes: BTreeMap<SignalIndex, Node>,

    // Given a node, it returns the list of safe assignments '<==' in which this signal is part of
    //    the LHS of the assignment
    pub incoming_safe_assignments: BTreeMap<SignalIndex, SafeAssignmentIndex>,

    // Given a node, it returns the list of safe assignments '<==' in which this signal is part of
    //    the RHS of the assignment
    pub outgoing_safe_assignments: BTreeMap<SignalIndex, BTreeSet<SafeAssignmentIndex>>,

    // Given a node, it returns the list of all constraints '===' that are not a result of safe
    //    assignments '<==' in which this signal appears
    pub edge_constraints: BTreeMap<SignalIndex, BTreeSet<UnsafeConstraintIndex>>,

    // Given a component index, it returns the SubComponent struct
    pub subcomponents: BTreeMap<ComponentIndex, SubComponent>,

    // List of all safe_assignments (<==). Edges only have indices into this vector.
    // Elements in this vector should not be removed, because the indices would be invalidated.
    pub safe_assignments: Vec<SafeAssignment>,

    // List of all unsafe_constraints (===). Edges only have indices into this vector.
    // Elements in this vector should not be removed, because the indices would be invalidated.
    pub unsafe_constraints: Vec<UnsafeConstraint>,

    pub substitutions: LinkedList<Substitution<usize>>,

    //  List of nodes that have been fixed (proved to be unique) but not yet removed from the graph
    pub fixed_nodes: BTreeSet<SignalIndex>,
}


impl VerificationGraph {
    pub fn new(
        context: &InputDataContextView,
    ) -> VerificationGraph {
        let tree_constraints = context.tree_constraints;

        let mut nodes = BTreeMap::<SignalIndex, Node>::new();
        let mut subcomponents = BTreeMap::<ComponentIndex, SubComponent>::new();

        // Outputs
        for idx in 0..tree_constraints.number_outputs {
            let s = idx + tree_constraints.initial_signal;
            nodes.insert(
                s, Node::OutputSignal,
            );
        }

        let mut input_signals = BTreeSet::new();

        // Inputs
        for idx in 0..tree_constraints.number_inputs {
            let s = idx + tree_constraints.number_outputs + tree_constraints.initial_signal;
            nodes.insert(
                s, Node::InputSignal,
            );
            input_signals.insert(s);
        }

        // Intermediates
        let number_intermediates = tree_constraints.number_signals - tree_constraints.number_outputs
            - tree_constraints.number_inputs;

        for idx in 0..number_intermediates {
            let s = idx
                + tree_constraints.number_outputs
                + tree_constraints.number_inputs
                + tree_constraints.initial_signal;

            nodes.insert(s, Node::IntermediateSignal);
        }

        // Components

        // TODO: We should make a difference between safe and unsafe subcomponents. By default, we will
        //  assume that all subcomponents ought to be safe (that is, their output must remain fixed
        //  if the input is fixed). We should allow "unsafe" subcomponents (such as Inverse), which might
        //  not fully determine their outputs when their inputs are fixed. In that case, we should "extract"
        //  the subcomponent signals, constraints and subcomponents into the parent component, so
        //  we can perform the algorithm taking the relationships into account, not as a black box.
        //      This information about unsafe components should be passed by an input .json

        for (cmp_index, c) in tree_constraints.subcomponents.iter().enumerate() {
            let mut subcomponent_inputs = BTreeSet::new();
            let mut subcomponent_outputs = BTreeSet::new();
            let component_index = c.node_id;

            // Subcomponent inputs
            for idx in 0..c.number_inputs {
                let s = idx + c.number_outputs + c.initial_signal;
                subcomponent_inputs.insert(s);
                nodes.insert(
                    s, Node::SubComponentInputSignal(component_index),
                );
            }

            // Subcomponent outputs
            for idx in 0..c.number_outputs {
                let s = idx + c.initial_signal;
                subcomponent_outputs.insert(s);

                nodes.insert(
                    s, Node::SubComponentOutputSignal(cmp_index),
                );
            }

            subcomponents.insert(
                cmp_index,
                SubComponent {
                    input_signals: subcomponent_inputs.clone(),
                    output_signals: subcomponent_outputs,
                    not_yet_fixed_inputs: subcomponent_inputs,
                },
            );
        }

        let mut incoming_safe_assignments = BTreeMap::<SignalIndex, SafeAssignmentIndex>::new();
        let mut outgoing_safe_assignments = BTreeMap::<SignalIndex, BTreeSet<SafeAssignmentIndex>>::new();
        let mut safe_assignments = vec![];

        let mut is_constraint_double_arrow = HashSet::new();

        // Add safe assignment edges
        for (constraint, lhs_signal) in &tree_constraints.are_double_arrow {
            is_constraint_double_arrow.insert(*constraint);

            let mut signals: BTreeSet<SignalIndex> = context.constraint_storage.read_constraint(*constraint).unwrap().take_cloned_signals_ordered();
            signals.remove(lhs_signal);

            let safe_assignment = SafeAssignment {
                lhs_signal: *lhs_signal,
                rhs_signals: signals,
                associated_constraint: *constraint,
            };

            let safe_assignment_idx = safe_assignments.len();
            safe_assignments.push(safe_assignment);

            incoming_safe_assignments.insert(*lhs_signal, safe_assignment_idx);

            // Outgoings
            for rhs_signal in context.constraint_storage.read_constraint(*constraint).unwrap().take_signals() {
                if rhs_signal != lhs_signal {
                    outgoing_safe_assignments.entry(*rhs_signal).or_insert(BTreeSet::new()).insert(safe_assignment_idx);
                }
            }
        }

        let mut edge_constraints: BTreeMap<SignalIndex, BTreeSet<UnsafeConstraintIndex>> = BTreeMap::new();
        let mut unsafe_constraints: Vec<UnsafeConstraint> = vec![];

        // Add unsafe edges
        let constraints_range = tree_constraints.initial_constraint..(tree_constraints.initial_constraint + tree_constraints.no_constraints);
        for (constraint_index, c) in constraints_range.filter(|idx| !is_constraint_double_arrow.contains(idx))
            .map(|x| (x, context.constraint_storage.read_constraint(x).unwrap())) {
            let signals = c.take_cloned_signals_ordered();

            let unsafe_constraint_index = unsafe_constraints.len();

            for &signal in &signals {
                // let vector: BTreeSet<SignalIndex> = signals.iter().filter(|x| **x != signal).copied().collect();
                edge_constraints.entry(signal).or_insert(BTreeSet::new()).insert(unsafe_constraint_index);
            }

            unsafe_constraints.push(UnsafeConstraint {
                signals,
                associated_constraint: constraint_index,
            });
        }

        // Compute fixed_nodes, which should include the inputs, safe assignments of only constants
        //  (for example, i <== 2) and linear constraints with only one appearing signal and non-zero coefficient
        //  (for example, 3*s===1).
        // TODO: Maybe there are more fixed_nodes situation to take into account?

        // Input signals
        let mut fixed_nodes = BTreeSet::new();
        fixed_nodes.append(&mut input_signals);

        // Safe assignments of only constants
        for ass in &safe_assignments {
            propagate_fixed_node_in_safe_assignment(&mut fixed_nodes, ass);
        }

        let substitutions = LinkedList::new();

        // Unsafe constraints ===
        for unsafe_constraint in &unsafe_constraints {
            propagate_fixed_node_in_unsafe_constraint(context, &mut fixed_nodes, &substitutions, unsafe_constraint);
        }

        VerificationGraph {
            nodes,
            incoming_safe_assignments,
            outgoing_safe_assignments,
            edge_constraints,
            subcomponents,
            safe_assignments,
            unsafe_constraints,
            substitutions,
            fixed_nodes,
        }
    }

    // pub fn get_unsafe_constraints(&self) -> impl Iterator<Item=&UnsafeConstraint> {
    //     self.unsafe_constraints.iter()
    // }
}

// This function checks a safe assignment. If all RHS values have been fixed, the LHS will
// also be fixed. Called both on creation of the VerificationGraph and on fixed node propagation
fn propagate_fixed_node_in_safe_assignment(fixed_nodes: &mut BTreeSet<SignalIndex>, assignment: &SafeAssignment) {
    // Fix the LHS of a '<==' assignment if the RHS does not have any signals (are constants)
    // TODO: Is this condition correct and complete for safe assignments?
    if assignment.rhs_signals.is_empty() {
        fixed_nodes.insert(assignment.lhs_signal);
    }
}

// This function checks an unsafe constraint. If it only contains one unfixed signal, the constraint
// is linear and its coefficient is non-zero, that signal will also be marked fixed.
fn propagate_fixed_node_in_unsafe_constraint(context: &InputDataContextView, fixed_nodes: &mut BTreeSet<SignalIndex>,
                                             substitutions: &LinkedList<Substitution<usize>>,
                                             unsafe_constraint: &UnsafeConstraint) {
    // Fix the only signal of a === constraint if it is the only signal, the constraint is
    // linear, and its coefficient is non-zero
    // TODO: Is this condition correct and complete for unsafe assignments?

    if unsafe_constraint.signals.len() == 1 {
        let signal = unsafe_constraint.signals.last().unwrap();
        let constraint = context.constraint_storage.
            read_constraint(unsafe_constraint.associated_constraint).unwrap();

        // TODO: Study when we need to apply this substitution. If we want to prove weak safety (only
        //  for one input) we could probably apply the substitutions one by one.
        //  We need to study the case of strong safety (for all inputs)
        let substituted_constraint = apply_fixed_nodes_substitution(constraint, substitutions, context);

        // TODO: Check if this is the correct form to compute whether the constraint is linear
        if Constraint::<usize>::is_linear(&substituted_constraint) {
            let coefficient = substituted_constraint.c().get(signal).unwrap();
            if !coefficient.is_zero() {
                fixed_nodes.insert(*signal);
            }
        }
    }
}

// TODO: Study when to apply substitutions. If we want to prove weak safety (only
//  for one input) we could probably apply the substitutions one by one when fixing signals.
//  We need to study the case of strong safety (for all inputs)
fn apply_fixed_nodes_substitution(mut constraint: Constraint<usize>, substitutions: &LinkedList<Substitution<usize>>, context: &InputDataContextView) -> Constraint<usize> {
    // TODO: Check if this implementation is correct

    for substitution in substitutions {
        Constraint::apply_substitution(&mut constraint, substitution, &context.field);
    }

    constraint
}