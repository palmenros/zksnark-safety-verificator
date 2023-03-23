use std::path::Path;
use graphviz_rust::dot_generator::*;
use graphviz_rust::dot_structures::*;
use graphviz_rust::printer::{DotPrinter, PrinterContext};
use std::error::Error;
use std::fs::File;
use std::io::Write;
use graphviz_rust::attributes::{color_name, NodeAttributes, shape};
use graphviz_rust::cmd::Format;
use graphviz_rust::{exec, print};
use crate::{SignalNameMap, TreeConstraints};

fn construct_graph_from_tree_constraint(tree_constraints: &TreeConstraints, signal_name_map: &SignalNameMap) -> Graph {
    let mut g = graph!(id!("id"));

    // Outputs
    for idx in 0..tree_constraints.number_outputs {
        let s = idx + tree_constraints.initial_signal;
        g.add_stmt(Stmt::Node(node!(s.to_string(); attr!("label", esc signal_name_map.get(&s).unwrap()))));
    }

    // Inputs
    for idx in 0..tree_constraints.number_inputs {
        let s = idx + tree_constraints.number_outputs +  tree_constraints.initial_signal;
        g.add_stmt(Stmt::Node(node!(s.to_string(); attr!("label", esc signal_name_map.get(&s).unwrap()))));
    }

    // Intermediates
    for idx in 0..(tree_constraints.number_signals - tree_constraints.number_outputs - tree_constraints.number_inputs) {
        let s = idx + tree_constraints.number_outputs + tree_constraints.number_inputs +  tree_constraints.initial_signal;
        g.add_stmt(Stmt::Node(node!(s.to_string(); attr!("label", esc signal_name_map.get(&s).unwrap()))));
    }

    // Components

    let mut cmp_index = 0;
    for c in &tree_constraints.subcomponents {
        let mut v = Vec::<Stmt>::new();

        // Subcomponent outputs
        for idx in 0..c.number_outputs {
            let s = idx + c.initial_signal;
            v.push(Stmt::Node(node!(s.to_string();
                attr!("label", esc signal_name_map.get(&s).unwrap())
            )));
        }

        // Subcomponent inputs

        for idx in 0..c.number_inputs {
            let s = idx + c.number_outputs +  c.initial_signal;
            v.push(Stmt::Node(node!(s.to_string();
                attr!("label", esc signal_name_map.get(&s).unwrap())
            )));
        }

        let subgraph_id = format!("cluster_{cmp_index}");
        let mut subgraph = subgraph!(esc subgraph_id);
        subgraph.stmts.push(Stmt::Attribute(attr!("style", "filled")));
        subgraph.stmts.push(Stmt::Attribute(attr!("color", "lightgrey")));
        subgraph.stmts.push(Stmt::Attribute(attr!("label", esc c.template_name)));
        subgraph.stmts.push(Stmt::GAttribute(GraphAttributes::Node(vec![
                attr!("style", "filled"),
                attr!("color", "white")
        ])));

        subgraph.stmts.append(&mut v);

        g.add_stmt(Stmt::Subgraph(subgraph));

        cmp_index += 1;
    }

    return g;
}

pub fn print_tree_constraint_graph(tree_constraints: &TreeConstraints, signal_name_map: &SignalNameMap, path: &Path) -> Result<(), Box<dyn Error>> {
    let g = construct_graph_from_tree_constraint(tree_constraints, signal_name_map);

    // Debug print of Graphviz code
    let s = print(g.clone(), &mut PrinterContext::default());
    println!("{}", s);

    let graph_svg = exec(
        g,
        &mut PrinterContext::default(),
        vec![Format::Svg.into()],
    )?;

    let mut f = File::create(path)?;
    f.write_all(graph_svg.as_bytes())?;

    Ok(())
}
