#![allow(dead_code)]

mod input_data;
mod tree_constraint_graph_printer;
mod verification_graph;

use input_data::*;
use tree_constraint_graph_printer::*;

use crate::verification_graph::VerificationGraph;
use std::error::Error;
use std::path::Path;

// TODO: We should add an option for the user to prove strong safety for all inputs for a module
//  that has === constraints (and therefore only handle the rest of the modules using our local
//  algorithm)

// TODO: We should apply some heuristics for quickly verifying modules without === constraints,
//  such as the one published in Circom paper

fn main() -> Result<(), Box<dyn Error>> {
    let base_path =
        Path::new(r"C:\Users\pedro\Documents\dev\CircomVerification\test-artifacts\binsubtest");

    // print_constraint_storage(&storage);
    // print_witness(&witness);
    // print_signal_name_map(&signal_name_map);
    // print_tree_constraints(&tree_constraints);
    let (context, mut constraint_storage) = InputDataContext::parse_from_files(base_path)?;
    let global_context_view = context.get_context_view();

    let context_view = global_context_view;
    // let context_view = global_context_view.get_subcomponent_context_view(3);

    let mut verification_graph = VerificationGraph::new(&context_view, &constraint_storage);
    print_verification_graph(
        &verification_graph,
        &context_view,
        base_path.join("svg/components.svg").as_path(),
    )?;

    verification_graph.propagate_fixed_nodes(&context_view, &mut constraint_storage);

    Ok(())
}
