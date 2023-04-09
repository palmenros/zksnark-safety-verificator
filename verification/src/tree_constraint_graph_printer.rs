use crate::verification_graph::VerificationGraph;
use crate::InputDataContextView;
use graphviz_rust::cmd::Format;
use graphviz_rust::dot_generator::*;
use graphviz_rust::dot_structures::*;
use graphviz_rust::exec;
use graphviz_rust::printer::PrinterContext;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::verification_graph::Node as VNode;

//noinspection SpellCheckingInspection
fn construct_graphviz_graph_from_verification_graph(
    verification_graph: &VerificationGraph,
    context: &InputDataContextView,
) -> Graph {
    let mut g = graph!(di id!("id"));

    // Nodes

    // Extra-style attributes for already fixed nodes
    let fixed_attrs = vec![
        attr!("style", "filled"),
        attr!("fillcolor", "firebrick4"),
        attr!("fontcolor", "white"),
    ];

    for (s, node) in verification_graph.nodes.iter().filter(|(_, n)| {
        matches!(
            **n,
            VNode::InputSignal | VNode::OutputSignal | VNode::IntermediateSignal
        )
    }) {
        let mut attrs = match node {
            VNode::InputSignal | VNode::OutputSignal => vec![
                attr!("label", esc context.signal_name_map.get(s).unwrap()),
                attr!("color", "orange"),
                attr!("shape", "Mdiamond"),
            ],
            VNode::IntermediateSignal => {
                vec![attr!("label", esc context.signal_name_map.get(s).unwrap())]
            }

            _ => unreachable!(),
        };

        // Add style if this node has been fixed
        if verification_graph.fixed_nodes.contains(s) {
            attrs.append(&mut fixed_attrs.clone());
        }

        g.add_stmt(Stmt::Node(node!(s.to_string(), attrs)));

        //  Handle input and output edges from nowhere

        if let VNode::OutputSignal = node {
            // Outputs
            let tmp_str = format!("output_dummy_{s}");
            g.add_stmt(Stmt::Node(
                node!(tmp_str; attr!("shape", "none"), attr!("label", esc "")),
            ));
            g.add_stmt(Stmt::Edge(
                edge!(node_id!(s.to_string()) => node_id!(tmp_str) ),
            ));
        }

        if let VNode::InputSignal = node {
            // Inputs
            let tmp_str = format!("input_dummy_{s}");
            g.add_stmt(Stmt::Node(
                node!(tmp_str; attr!("shape", "none"), attr!("label", esc "")),
            ));
            g.add_stmt(Stmt::Edge(
                edge!(node_id!(tmp_str) => node_id!(s.to_string())),
            ));
        }
    }

    // Component edges
    for (cmp_index, c) in &verification_graph.subcomponents {
        let mut v = Vec::<Stmt>::new();

        // We will only draw edges inside the component if there are both inputs and outputs.
        // A component may not have inputs or outputs if they have been previously fixed and deleted.
        let should_draw_edges = !c.input_signals.is_empty() && !c.output_signals.is_empty();

        // Add subcomponent inputs and outputs

        let dummy_node_str = format!("dummy_{cmp_index}");

        if should_draw_edges {
            // Dummy point for edges
            v.push(Stmt::Node(node!(dummy_node_str;
            attr!("shape", "point"),
            attr!("fontname", "Courier")
            // attr!("xlabel", "Component")
            )));
        }

        for output in &c.output_signals {
            let mut attrs = vec![
                attr!("label", esc context.signal_name_map.get(output).unwrap()),
                attr!("color", "blue"),
            ];

            // Add style if this node has been fixed
            if verification_graph.fixed_nodes.contains(output) {
                attrs.append(&mut fixed_attrs.clone());
            }

            v.push(Stmt::Node(node!(output.to_string(), attrs)));

            if should_draw_edges {
                v.push(Stmt::Edge(
                    edge!(node_id!(dummy_node_str) => node_id!(output.to_string())),
                ));
            }
        }

        for input in &c.input_signals {
            let mut attrs = vec![
                attr!("label", esc context.signal_name_map.get(input).unwrap()),
                attr!("color", "green"),
            ];
            // Add style if this node has been fixed
            if verification_graph.fixed_nodes.contains(input) {
                attrs.append(&mut fixed_attrs.clone());
            }

            v.push(Stmt::Node(node!(input.to_string(), attrs)));

            if should_draw_edges {
                v.push(Stmt::Edge(
                    edge!(node_id!(input.to_string()) => node_id!(dummy_node_str); attr!("dir", "none")),
                ));
            }
        }

        let subgraph_id = format!("cluster_{cmp_index}");
        let mut subgraph = subgraph!(esc subgraph_id);
        subgraph
            .stmts
            .push(Stmt::Attribute(attr!("style", "filled")));
        subgraph
            .stmts
            .push(Stmt::Attribute(attr!("color", "lightgrey")));

        let comp = context
            .tree_constraints
            .subcomponents
            .get(*cmp_index)
            .unwrap();

        let (_, component_name) = comp.component_name.split_once('.').unwrap();
        let component_subgraph_name = format!("{}: {}", component_name, comp.template_name);
        subgraph
            .stmts
            .push(Stmt::Attribute(attr!("label", esc component_subgraph_name)));
        subgraph
            .stmts
            .push(Stmt::GAttribute(GraphAttributes::Node(vec![
                attr!("style", "filled"),
                attr!("fillcolor", "white"),
            ])));

        subgraph.stmts.append(&mut v);

        g.add_stmt(Stmt::Subgraph(subgraph));
    }

    // Safe assignment double_arrow <== constraints

    for ass in &verification_graph.safe_assignments {
        if !ass.active {
            continue;
        }

        let lhs = ass.lhs_signal;
        // TODO: Handle rhs_signals of length 0 (for example i <== 1).
        if ass.rhs_signals.len() == 1 {
            let rhs = ass.rhs_signals.iter().next().unwrap();
            // Only one source, create direct edge
            g.add_stmt(Stmt::Edge(edge!(
                node_id!(rhs.to_string()) => node_id!(lhs.to_string());
                attr!("label", esc " <=="),
                attr!("fontname", "Courier"),
                attr!("color", "red")
            )));
        } else {
            // Multiple sources, create intermediate node
            let intermediate_node_str = format!("safe_assign_{lhs}");
            g.add_stmt(Stmt::Node(node!(
                intermediate_node_str;
                attr!("shape", "point"),
                attr!("fontname", "Courier"),
                attr!("xlabel", esc "<==")
            )));
            g.add_stmt(Stmt::Edge(edge!(
                node_id!(intermediate_node_str) => node_id!(lhs.to_string());
                attr!("color", "red")
            )));

            for rhs in &ass.rhs_signals {
                g.add_stmt(Stmt::Edge(edge!(
                    node_id!(rhs.to_string()) => node_id!(intermediate_node_str);
                    attr!("color", "red")
                )));
            }
        }
    }

    // Handle unsafe constraints ===
    for c in &verification_graph.unsafe_constraints {
        if !c.active {
            continue;
        }

        if c.signals.len() == 1 {
            // Only one signal appears, make a loop
            let signal = c.signals.iter().next().unwrap();
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

            let tmp_node_str = format!("constraint_{}", c.associated_constraint);

            // TODO: Find a better way to label the point with ===
            g.add_stmt(Stmt::Node(node!(
                tmp_node_str;
                attr!("shape", "point"),
                attr!("xlabel", esc " ===")
            )));

            for signal in &c.signals {
                // The direction of the edge matters for aesthetics in the graph.
                // As a heuristic, if the node is an input, it will be the origin, else,
                //   it will be a destination
                let attrs = vec![attr!("dir", "none"), attr!("color", "green")];

                if context.is_signal_public(*signal) {
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

    g
}

pub fn print_verification_graph(
    verification_graph: &VerificationGraph,
    context: &InputDataContextView,
    path: &Path,
) -> Result<(), Box<dyn Error>> {
    let g = construct_graphviz_graph_from_verification_graph(verification_graph, context);

    // TODO: Remove println
    // Debug print of Graphviz code
    // let s = graphviz_rust::print(g.clone(), &mut PrinterContext::default());
    // println!("{}", s);

    let graph_svg = exec(g, &mut PrinterContext::default(), vec![Format::Svg.into()])?;

    let mut f = File::create(path)?;
    f.write_all(graph_svg.as_bytes())?;

    Ok(())
}
