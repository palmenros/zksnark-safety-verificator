use std::collections::HashSet;
use crate::{print_constraint, SignalNameMap, TreeConstraints};
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
            attr!("shape", "point"),
            attr!("fontname", "Courier"),
            attr!("xlabel", "Component")
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

        let (_, component_name) = c.component_name.split_once(".").unwrap();
        let component_subgraph_name = format!("{}: {}", component_name, c.template_name);
        subgraph.stmts.push(Stmt::Attribute(attr!("label", esc component_subgraph_name)));
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
    let mut double_arrow_constraints = HashSet::<usize>::new();

    for (cnt, assigned_signal) in &tree_constraints.are_double_arrow {
        double_arrow_constraints.insert(*cnt);

        let c = storage.read_constraint(*cnt).unwrap();
        let mut sources = c.take_cloned_signals();
        sources.remove(assigned_signal);

        if sources.len() == 1 {
            let source = sources.into_iter().nth(0).unwrap();
            // Only one source, create direct edge
            g.add_stmt(Stmt::Edge(edge!(
                node_id!(source.to_string()) => node_id!(assigned_signal.to_string());
                attr!("label", esc " <=="),
                attr!("fontname", "Courier"),
                attr!("color", "red")
            )));
        } else {
            // Multiple sources, create intermediate node
            let intermediate_node_str = format!("safe_assign_{assigned_signal}");
            g.add_stmt(Stmt::Node(node!(
                intermediate_node_str;
                attr!("shape", "point")
            )));
            g.add_stmt(Stmt::Edge(edge!(
                node_id!(intermediate_node_str) => node_id!(assigned_signal.to_string())
            )));

            for source in sources {
                g.add_stmt(Stmt::Edge(edge!(
                    node_id!(source.to_string()) => node_id!(intermediate_node_str)
                )));
            }
        }
    }

    if tree_constraints.no_constraints != tree_constraints.are_double_arrow.len() {
        // There are constraints not generated by safe <== assignments
        for idx in 0..tree_constraints.no_constraints {
            let cnt = idx + tree_constraints.initial_constraint;
            if double_arrow_constraints.contains(&cnt) {
                continue;
            }

            // cnt is a constraint note generated by an <== assignment

            let constraint = storage.read_constraint(cnt).unwrap();
            let signals = constraint.take_signals();

            if signals.len() == 1 {
                // Only one signal appears, make a loop
                let signal = signals.into_iter().nth(0).unwrap();
                g.add_stmt(Stmt::Edge(edge!(
                    node_id!(signal.to_string()) => node_id!(signal.to_string());
                    attr!("dir", "none"),
                    attr!("color", "green"),
                    attr!("label", esc " ==="),
                    attr!("fontname", "Courier")
                )));
            } else {
                // TODO: Maybe special case for constraints where only 2 signals appear where we don't
                //          draw the inner point?

                let tmp_node_str = format!("constraint_{cnt}");
                // TODO: Find a way to label the point with ===
                g.add_stmt(Stmt::Node(node!(
                    tmp_node_str;
                    attr!("shape", "point"),
                    attr!("xlabel", esc " ===")
                )));

                for signal in constraint.take_signals() {
                    // The direction of the edge matters for aesthetics in the graph.
                    // As a heuristic, if the node is an input, it will be the origin, else,
                    //   it will be a destination
                    let attrs = vec![
                        attr!("dir", "none"),
                        attr!("color", "green"),
                    ];

                    if *signal >= tree_constraints.initial_signal + tree_constraints.number_outputs &&
                        *signal < tree_constraints.initial_signal + tree_constraints.number_outputs + tree_constraints.number_inputs {
                        // This signal is an input
                        g.add_stmt(Stmt::Edge(edge!(
                            node_id!(signal.to_string()) => node_id!(tmp_node_str), attrs
                        )));
                    } else {
                        g.add_stmt(Stmt::Edge(edge!(
                            node_id!(tmp_node_str) => node_id!(signal.to_string()), attrs
                        )));
                    }
                }
            }
        }
    }

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
