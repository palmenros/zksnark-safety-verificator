mod input_data;
mod tree_constraint_graph_printer;
mod verification_graph;

use input_data::*;
use tree_constraint_graph_printer::*;

use std::error::Error;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    let base_path = Path::new(r"C:\Users\pedro\Documents\dev\CircomVerification\test-artifacts\binsubtest");

    // print_constraint_storage(&storage);
    // print_witness(&witness);
    // print_signal_name_map(&signal_name_map);
    // print_tree_constraints(&tree_constraints);
    let context = InputDataContext::parse_from_files(base_path)?;

    // let subcomponent = tree_constraints.subcomponents.into_iter().nth(2).unwrap();
    print_tree_constraint_graph(&context.tree_constraints, &context.signal_name_map,
                                &context.constraint_storage, base_path.join("components.svg").as_path())?;

    return Ok(());
}
