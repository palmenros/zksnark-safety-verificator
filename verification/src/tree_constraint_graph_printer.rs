use crate::verification_graph::VerificationGraph;
use crate::InputDataContextView;
use graphviz_rust::cmd::Format;
use graphviz_rust::dot_generator::*;
use graphviz_rust::dot_structures::*;
use graphviz_rust::exec;
use graphviz_rust::printer::PrinterContext;
use std::cell::RefCell;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::verification_graph::Node as VNode;

pub struct DebugSVGPrinter {
    // String containing the base filepath of the base SVG folder output
    svg_folder_path: String,

    // This index counts which SVG file is the next to be printed, to be able to have sequential
    //  filenames
    index: RefCell<i32>,
}

impl DebugSVGPrinter {
    pub fn new(svg_folder_path: &str) -> Self {
        // TODO: Maybe only delete SVGs if options.generate_svg_diagrams
        delete_all_files(Path::new(svg_folder_path));

        Self {
            svg_folder_path: String::from(svg_folder_path),
            index: RefCell::new(0),
        }
    }

    pub fn print_verification_graph(
        &self,
        verification_graph: &VerificationGraph,
        context: &InputDataContextView,
        file_name: &str,
        graph_title: Option<&str>,
    ) -> Result<(), Box<dyn Error>> {
        // If debug SVGs are deactivated, do not try to draw
        if !context.options.generate_svg_diagrams {
            return Ok(());
        }

        let g = construct_graphviz_graph_from_verification_graph(
            verification_graph,
            context,
            graph_title,
        );

        // The following commented code prints the textual version of the graphviz code
        // let s = graphviz_rust::print(g.clone(), &mut PrinterContext::default());
        // println!("{}", s);

        let graph_svg = exec(g, &mut PrinterContext::default(), vec![Format::Svg.into()])?;

        // Create a sequential filename: for example: svg/000-components.svg

        let mut index = self.index.borrow_mut();

        let path = Path::new(self.svg_folder_path.as_str())
            .join(format!("{:0>3}-{}.svg", index, file_name));

        *index += 1;

        fs::create_dir_all(path.parent().unwrap())?;
        let mut f = File::create(path)?;
        f.write_all(graph_svg.as_bytes())?;

        Ok(())
    }
}

fn delete_all_files(base_path: &Path) {
    if base_path.is_dir() {
        fs::remove_dir_all(base_path).unwrap();
    }
    fs::create_dir(base_path).unwrap();
}

//noinspection SpellCheckingInspection
fn construct_graphviz_graph_from_verification_graph(
    verification_graph: &VerificationGraph,
    context: &InputDataContextView,
    graph_title: Option<&str>,
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
        let highlight_node = verification_graph
            .debug_polynomial_system_generator_data
            .nodes
            .contains(s);
        let highlight_color = "fuchsia";

        let mut attrs = match node {
            VNode::InputSignal | VNode::OutputSignal => vec![
                attr!("label", esc context.signal_name_map.get(s).unwrap()),
                attr!("color", esc if highlight_node {highlight_color} else {"orange"}),
                attr!("shape", "Mdiamond"),
            ],
            VNode::IntermediateSignal => {
                vec![
                    attr!("label", esc context.signal_name_map.get(s).unwrap()),
                    attr!("color", esc if highlight_node {highlight_color} else {"black"}),
                ]
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

        let highlight_color = "fuchsia";

        if should_draw_edges {
            // Dummy point for edges
            v.push(Stmt::Node(node!(dummy_node_str;
            attr!("shape", "point"),
            attr!("fontname", "Courier")
            // attr!("xlabel", "Component")
            )));
        }

        for output in &c.output_signals {
            let highlight_node = verification_graph
                .debug_polynomial_system_generator_data
                .nodes
                .contains(output);

            let mut attrs = vec![
                attr!("label", esc context.signal_name_map.get(output).unwrap()),
                attr!("color", esc if highlight_node {highlight_color} else {"blue"}),
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
            let highlight_node = verification_graph
                .debug_polynomial_system_generator_data
                .nodes
                .contains(input);

            let mut attrs = vec![
                attr!("label", esc context.signal_name_map.get(input).unwrap()),
                attr!("color", esc if highlight_node {highlight_color} else {"green"}),
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

    for (s_idx, ass) in verification_graph.safe_assignments.iter().enumerate() {
        if !ass.active {
            continue;
        }

        // Highlight if chosen in connected component debug info

        let lhs = ass.lhs_signal;

        let highlight_edge = verification_graph
            .debug_polynomial_system_generator_data
            .safe_assignments
            .contains(&s_idx);
        let edge_color = if highlight_edge {
            "fuchsia:fuchsia"
        } else {
            "red"
        };

        // TODO: Better handle rhs_signals of length 0 (for example i <== 1).
        if ass.rhs_signals.len() == 1 {
            let rhs = ass.rhs_signals.iter().next().unwrap();
            // Only one source, create direct edge
            g.add_stmt(Stmt::Edge(edge!(
                node_id!(rhs.to_string()) => node_id!(lhs.to_string());
                attr!("label", esc " <=="),
                attr!("fontname", "Courier"),
                attr!("color", esc edge_color)
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
                attr!("color", esc edge_color)
            )));

            for rhs in &ass.rhs_signals {
                g.add_stmt(Stmt::Edge(edge!(
                    node_id!(rhs.to_string()) => node_id!(intermediate_node_str);
                    attr!("color", esc edge_color)
                )));
            }
        }
    }

    // Handle unsafe constraints ===
    for (c_idx, c) in verification_graph.unsafe_constraints.iter().enumerate() {
        if !c.active {
            continue;
        }

        let highlight_edge = verification_graph
            .debug_polynomial_system_generator_data
            .unsafe_constraints
            .contains(&c_idx);
        let edge_color = if highlight_edge {
            "deeppink:fuchsia:fuchsia:deeppink"
        } else {
            "green"
        };

        if c.signals.len() == 1 {
            // Only one signal appears, make a loop
            let signal = c.signals.iter().next().unwrap();
            g.add_stmt(Stmt::Edge(edge!(
                node_id!(signal.to_string()) => node_id!(signal.to_string());
                attr!("dir", "none"),
                attr!("color", esc edge_color),
                attr!("label", esc " ==="),
                attr!("fontname", "Courier")
            )));
        } else {
            // TODO: Create a special case for === constraints where only 2 signals appear so we
            //  don't draw the inner point.

            let tmp_node_str = format!("constraint_{}", c.associated_constraint);

            // Label the point with ===
            g.add_stmt(Stmt::Node(node!(
                tmp_node_str;
                attr!("shape", "point"),
                attr!("xlabel", esc " ===")
            )));

            for signal in &c.signals {
                // The direction of the edge matters for aesthetics in the graph.
                // As a heuristic, if the node is an input, it will be the origin, else,
                //   it will be a destination
                let attrs = vec![attr!("dir", "none"), attr!("color",esc edge_color)];

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

    // Add graph title
    if let Some(s) = graph_title {
        g.add_stmt(Stmt::Attribute(attr!("label", esc s)));
        g.add_stmt(Stmt::Attribute(attr!("labelloc", "t")));
    }

    g
}
