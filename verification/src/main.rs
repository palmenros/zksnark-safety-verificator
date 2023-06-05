#![allow(dead_code)]

mod cli;
mod input_data;
mod polynomial_system_fixer;
mod tree_constraint_graph_printer;
mod verification_graph;
mod verifier;

use input_data::*;
use tree_constraint_graph_printer::*;

use crate::cli::parse_command_line_arguments;
use std::error::Error;
use std::path::Path;

// TODO: We should add an option for the user to prove strong safety for all inputs for a module
//  that has === constraints (and therefore only handle the rest of the modules using our local
//  algorithm)

// TODO: We should apply some heuristics for quickly verifying modules without === constraints,
//  such as the one published in Circom paper

// TODO: When outputting constraints for Cocoa, try to simplify them using Gauss-Jordan first

// TODO: When outputting constraints for Cocoa, first do a reachability analysis and remove all
//  constraints not reachable by the outputs to fix

fn main() -> Result<(), Box<dyn Error>> {
    let (maybe_base_path, options) = parse_command_line_arguments();

    let base_path = maybe_base_path.unwrap_or_else(|| {
        // Hardcoded path for testing purposes if that flag was passed
        let test_artifacts_path =
            Path::new(r"C:\Users\pedro\Documents\dev\CircomVerification\test-artifacts\");
        let folder_name = r"binsubtest8bit";

        test_artifacts_path.join(folder_name)
    });

    // print_constraint_storage(&storage);
    // print_witness(&witness);
    // print_signal_name_map(&signal_name_map);
    // print_tree_constraints(&tree_constraints);
    let (context, mut constraint_storage) =
        InputDataContext::parse_from_files(&base_path, options)?;
    let global_context_view = context.get_context_view();

    let context_view = global_context_view;
    // let context_view = global_context_view.get_subcomponent_context_view(2);

    verifier::verify(&context_view, &mut constraint_storage)?;

    Ok(())
}
