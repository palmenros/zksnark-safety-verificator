#![allow(dead_code)]

mod input_data;
mod tree_constraint_graph_printer;
mod verification_graph;

use input_data::*;
use tree_constraint_graph_printer::*;

use std::error::Error;
use std::path::Path;
use crate::verification_graph::VerificationGraph;

fn main() -> Result<(), Box<dyn Error>> {
    let base_path = Path::new(r"C:\Users\pedro\Documents\dev\CircomVerification\test-artifacts\binsubtest");

    // print_constraint_storage(&storage);
    // print_witness(&witness);
    // print_signal_name_map(&signal_name_map);
    // print_tree_constraints(&tree_constraints);
    let context = InputDataContext::parse_from_files(base_path)?;
    let global_context_view = context.get_context_view();

    let context_view = global_context_view;
    // let context_view = global_context_view.get_subcomponent_context_view(3);

    let verification_graph = VerificationGraph::new(&context_view);
    print_verification_graph(&verification_graph, &context_view, base_path.join("components.svg").as_path())?;

    Ok(())
}
