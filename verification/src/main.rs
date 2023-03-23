mod input_data;
mod tree_constraint_graph_printer;

use input_data::*;
use tree_constraint_graph_printer::*;

use std::error::Error;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    let base_path = Path::new(r"C:\Users\pedro\Documents\dev\CircomVerification\test-artifacts\binsubtest");
    let storage = parse_constraint_list(base_path.join("circuit_constraints.json").as_path())?;
    let witness = parse_witness(base_path.join("witness.json").as_path())?;
    let signal_name_map = parse_signal_name_map(base_path.join("circuit_signals.sym").as_path())?;
    let tree_constraints = parse_tree_constraints(base_path.join("circuit_treeconstraints.json").as_path())?;

    // print_constraint_storage(&storage);
    // print_witness(&witness);
    // print_signal_name_map(&signal_name_map);
    // print_tree_constraints(&tree_constraints);
    print_tree_constraint_graph(&tree_constraints, &signal_name_map, base_path.join("components.svg").as_path())?;

    return Ok(());
}
