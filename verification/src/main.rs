#![allow(dead_code)]

mod input_data;
mod tree_constraint_graph_printer;
mod verification_graph;

use input_data::*;
use tree_constraint_graph_printer::*;

use crate::verification_graph::VerificationGraph;
use std::error::Error;
use std::fs;
use std::path::Path;

// TODO: We should add an option for the user to prove strong safety for all inputs for a module
//  that has === constraints (and therefore only handle the rest of the modules using our local
//  algorithm)

// TODO: We should apply some heuristics for quickly verifying modules without === constraints,
//  such as the one published in Circom paper

// TODO: When outputing constraints for Cocoa, remember to print the 0==0 constraint if all hash
//  maps are empty

// TODO: When outputing constraints for Cocoa, first do a reachability analysis and remove all
//  constraints not reachable by the outputs to fix

fn delete_all_svg_files(base_path: &Path) {
    if base_path.join("svg").is_dir() {
        fs::remove_dir_all(base_path.join("svg")).unwrap();
    }
    fs::create_dir(base_path.join("svg")).unwrap();
}

fn main() -> Result<(), Box<dyn Error>> {
    let test_artifacts_path =
        Path::new(r"C:\Users\pedro\Documents\dev\CircomVerification\test-artifacts\");
    let folder_name = "different_connected_components";

    let base_path = Path::join(test_artifacts_path, folder_name);

    // print_constraint_storage(&storage);
    // print_witness(&witness);
    // print_signal_name_map(&signal_name_map);
    // print_tree_constraints(&tree_constraints);
    let (context, mut constraint_storage) = InputDataContext::parse_from_files(&base_path)?;
    let global_context_view = context.get_context_view();

    let context_view = global_context_view;
    // let context_view = global_context_view.get_subcomponent_context_view(2);

    delete_all_svg_files(&base_path);

    let mut verification_graph = VerificationGraph::new(&context_view, &constraint_storage);
    print_verification_graph(
        &verification_graph,
        &context_view,
        base_path.join("svg/components.svg").as_path(),
    )?;

    verification_graph.verify(&context_view, &mut constraint_storage);

    Ok(())
}
