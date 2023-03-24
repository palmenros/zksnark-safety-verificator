use crate::{SignalNameMap, TreeConstraints};
use graphviz_rust::attributes::{color_name, shape, NodeAttributes};
use graphviz_rust::cmd::Format;
use graphviz_rust::dot_generator::*;
use graphviz_rust::dot_structures::*;
use graphviz_rust::printer::{DotPrinter, PrinterContext};
use graphviz_rust::{exec, print};
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use circom_algebra::constraint_storage::ConstraintStorage;

fn construct_graph_from_tree_constraint(
    tree_constraints: &TreeConstraints,
    signal_name_map: &SignalNameMap,
    storage: &ConstraintStorage,
) -> Graph {
    let mut g = graph!(di id!("id"));

    // Outputs
    for idx in 0..tree_constraints.number_outputs {
        let s = idx + tree_constraints.initial_signal;
        g.add_stmt(Stmt::Node(
            node!(s.to_string();
                attr!("label", esc signal_name_map.get(&s).unwrap()),
                attr!("color", "red"),
                attr!("shape", "Mdiamond")
            ),
        ));

        // Output edge going to nowhere
        let tmp_str = format!("output_dummy_{idx}");
        g.add_stmt(Stmt::Node(node!(tmp_str; attr!("shape", "none"), attr!("label", esc ""))));
        g.add_stmt(Stmt::Edge(edge!(node_id!(s.to_string()) => node_id!(tmp_str) )));
    }

    // Inputs
    for idx in 0..tree_constraints.number_inputs {
        let s = idx + tree_constraints.number_outputs + tree_constraints.initial_signal;
        g.add_stmt(Stmt::Node(
            node!(s.to_string();
                attr!("label", esc signal_name_map.get(&s).unwrap()),
                attr!("color", "orange"),
                attr!("shape", "Mdiamond")
            ),
        ));

        // Input edge coming from nowhere
        let tmp_str = format!("input_dummy_{idx}");
        g.add_stmt(Stmt::Node(node!(tmp_str; attr!("shape", "none"), attr!("label", esc ""))));
        g.add_stmt(Stmt::Edge(edge!(node_id!(tmp_str) => node_id!(s.to_string()))));
    }

    // Intermediates
    let number_intermediates = tree_constraints.number_signals - tree_constraints.number_outputs
        - tree_constraints.number_inputs;

    for idx in 0..number_intermediates {
        let s = idx
            + tree_constraints.number_outputs
            + tree_constraints.number_inputs
            + tree_constraints.initial_signal;
        g.add_stmt(Stmt::Node(
            node!(s.to_string(); attr!("label", esc signal_name_map.get(&s).unwrap())),
        ));
    }

    // Components

    let mut cmp_index = 0;
    for c in &tree_constraints.subcomponents {
        let mut v = Vec::<Stmt>::new();

        let dummy_node_str = format!("dummy_{cmp_index}");

        // Dummy point for edges
        v.push(Stmt::Node(node!(dummy_node_str;
            attr!("shape", "point")
        )));

        // Subcomponent outputs
        for idx in 0..c.number_outputs {
            let s = idx + c.initial_signal;
            v.push(Stmt::Node(node!(s.to_string();
                attr!("label", esc signal_name_map.get(&s).unwrap()),
                attr!("color", "green")
            )));

            // Edge
            v.push(Stmt::Edge(edge!(node_id!(dummy_node_str) => node_id!(s.to_string()))));
        }

        // Subcomponent inputs

        for idx in 0..c.number_inputs {
            let s = idx + c.number_outputs + c.initial_signal;
            let i = node!(s.to_string();
                attr!("label", esc signal_name_map.get(&s).unwrap()),
                attr!("color", "blue")
            );

            v.push(Stmt::Node(i));

            // Edge
            v.push(Stmt::Edge(edge!(node_id!(s.to_string()) => node_id!(dummy_node_str);
                  attr!("dir", "none")
            )));
        }

        let subgraph_id = format!("cluster_{cmp_index}");
        let mut subgraph = subgraph!(esc subgraph_id);
        subgraph.stmts.push(Stmt::Attribute(attr!("style", "filled")));
        subgraph.stmts.push(Stmt::Attribute(attr!("color", "lightgrey")));
        subgraph.stmts.push(Stmt::Attribute(attr!("label", esc c.template_name)));
        subgraph.stmts.push(Stmt::GAttribute(GraphAttributes::Node(vec![
            attr!("style", "filled"),
            attr!("fillcolor", "white"),
        ])));

        subgraph.stmts.append(&mut v);

        g.add_stmt(Stmt::Subgraph(subgraph));

        cmp_index += 1;
    }

    // Constraints

    // Double arrow constraints
    for (cnt, assigned_signal) in &tree_constraints.are_double_arrow {
        let c = storage.read_constraint(*cnt).unwrap();
        let mut sources = c.take_cloned_signals();
        sources.remove(assigned_signal);

        // TODO: Expand to assignment from multiple sources
        assert_eq!(sources.len(), 1);

        for source in sources {
            g.add_stmt(Stmt::Edge(edge!(
                node_id!(source.to_string()) => node_id!(assigned_signal.to_string());
                attr!("label", esc " <=="),
                attr!("fontname", "Courier"),
                attr!("color", "red")
            )));
        }
    }

    // TODO: Handle non-double arrow signals
    assert_eq!(tree_constraints.no_constraints, tree_constraints.are_double_arrow.len());

    return g;
}

pub fn print_tree_constraint_graph(
    tree_constraints: &TreeConstraints,
    signal_name_map: &SignalNameMap,
    storage: &ConstraintStorage,
    path: &Path,
) -> Result<(), Box<dyn Error>> {
    let g = construct_graph_from_tree_constraint(tree_constraints, signal_name_map, storage);

    // Debug print of Graphviz code
    let s = print(g.clone(), &mut PrinterContext::default());
    println!("{}", s);

    let graph_svg = exec(g, &mut PrinterContext::default(), vec![Format::Svg.into()])?;

    let mut f = File::create(path)?;
    f.write_all(graph_svg.as_bytes())?;

    Ok(())
}
