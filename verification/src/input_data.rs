use circom_algebra::algebra::Constraint;
use circom_algebra::constraint_storage::ConstraintStorage;
use itertools::Itertools;
use num_bigint_dig::BigInt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::io::BufRead;
use std::path::{Path};
use std::{
    collections::{HashMap},
    io,
};

fn parse_constraint_list(path: &Path) -> Result<ConstraintStorage, Box<dyn Error>> {
    let f = File::open(path)?;
    let data: Value = serde_json::from_reader(f)?;

    let o = data
        .as_object()
        .ok_or("constraint.json main value is not an object")?;
    let json_constraint_list = o
        .get("constraints")
        .ok_or("constraint.json main object does not contain a constraints array")?;

    let v = json_constraint_list
        .as_array()
        .ok_or("constraint.json 'constraints' value is not an array")?;
    let mut storage = ConstraintStorage::new();

    for val in v {
        // Read one constraint
        let arr = val
            .as_array()
            .ok_or("constraint.json contains a non-array in constraint list")?;
        if arr.len() != 3 {
            return Err("Constraint in constraint.json has more than 3 terms".into());
        }

        let maybe_cs: Result<Vec<_>, _> = arr
            .iter()
            .map(|x| -> Result<HashMap<SignalIndex, BigInt>, Box<dyn Error>> {
                let m = x
                    .as_object()
                    .ok_or("Constraint in 'constraint.json' has a non-object")?;
                m.iter()
                    .map(|(k, v)| -> Result<(SignalIndex, BigInt), Box<dyn Error>> {
                        let s = v
                            .as_str()
                            .ok_or("Coefficient in 'constraint.json' is not a string")?;
                        Ok((k.parse::<usize>()?, s.parse::<BigInt>()?))
                    })
                    .collect()
            })
            .collect();

        let (a, b, c) = maybe_cs?.into_iter().collect_tuple().unwrap();
        storage.add_constraint(Constraint::new(a, b, c));
    }

    Ok(storage)
}

pub type ConstraintIndex = usize;
pub type Witness = HashMap<ConstraintIndex, BigInt>;

fn parse_witness(path: &Path) -> Result<Witness, Box<dyn Error>> {
    let f = File::open(path)?;
    let data: Value = serde_json::from_reader(f)?;

    let o = data
        .as_object()
        .ok_or("witness.json main value is not an object")?;
    let map = o
        .iter()
        .map(|(k, v)| -> Result<(usize, BigInt), Box<dyn Error>> {
            let s = v
                .as_str()
                .ok_or("witness.json has a witness value that is not a string")?;
            Ok((k.parse::<usize>()?, s.parse::<BigInt>()?))
        })
        .collect::<Result<Witness, Box<dyn Error>>>()?;

    Ok(map)
}

pub type SignalIndex = usize;
pub type SignalNameMap = HashMap<SignalIndex, String>;

fn parse_signal_name_map(path: &Path) -> Result<SignalNameMap, Box<dyn Error>> {
    let f = File::open(path)?;
    let mut map = SignalNameMap::new();

    for maybe_line in io::BufReader::new(f).lines() {
        let line = maybe_line.unwrap();
        let (id, _, _, fully_qualified_name) = line
            .split(',')
            .collect_tuple()
            .ok_or("Invalid number of entries per line in 'circuit_signals.sym'")?;

        // Remove first component path from name, that is, remove the initial "main."
        let (_, name) = fully_qualified_name.split_once(".").unwrap();
        map.insert(id.parse::<SignalIndex>()?, name.to_string());
    }

    Ok(map)
}

pub type ComponentIndex = usize;

#[derive(Default, Deserialize, Serialize)]
pub struct TreeConstraints {
    /* prime number corresponding to the field Z_p*/
    pub field: String,
    pub no_constraints: usize,
    pub initial_constraint: SignalIndex,
    pub node_id: ComponentIndex,
    pub template_name: String,
    pub component_name: String,
    pub number_inputs: usize,
    pub number_outputs: usize,
    pub number_signals: usize,
    pub initial_signal: SignalIndex,
    pub are_double_arrow: Vec<(ConstraintIndex, SignalIndex)>,
    // first number constraint, second number assigned signal
    pub subcomponents: Vec<TreeConstraints>,
}

fn parse_tree_constraints(path: &Path) -> Result<TreeConstraints, Box<dyn Error>> {
    let f = File::open(path)?;
    let constraints: TreeConstraints = serde_json::from_reader(f)?;

    Ok(constraints)
}

pub struct InputDataContext {
    pub constraint_storage: ConstraintStorage,
    pub witness: Witness,
    pub signal_name_map: SignalNameMap,
    pub tree_constraints: TreeConstraints,
}

pub struct InputDataContextView<'a> {
    pub constraint_storage: &'a ConstraintStorage,
    pub witness: &'a Witness,
    pub signal_name_map: &'a SignalNameMap,
    pub tree_constraints: &'a TreeConstraints,
}

impl InputDataContext {
    pub fn parse_from_files(folder_base_path: &Path) -> Result<InputDataContext, Box<dyn Error>> {
        let constraint_storage = parse_constraint_list(folder_base_path.join("circuit_constraints.json").as_path())?;
        let witness = parse_witness(folder_base_path.join("witness.json").as_path())?;
        let signal_name_map = parse_signal_name_map(folder_base_path.join("circuit_signals.sym").as_path())?;
        let tree_constraints = parse_tree_constraints(folder_base_path.join("circuit_treeconstraints.json").as_path())?;

        Ok(InputDataContext {
            constraint_storage,
            witness,
            signal_name_map,
            tree_constraints,
        })
    }

    pub fn get_context_view(&self) -> InputDataContextView {
        InputDataContextView {
            constraint_storage: &self.constraint_storage,
            witness: &self.witness,
            signal_name_map: &self.signal_name_map,
            tree_constraints: &self.tree_constraints,
        }
    }
}

/* Represents a view of the context. tree_constraints might be a subcomponent instead of main component */
impl<'a> InputDataContextView<'a> {
    pub fn get_subcomponent_context_view(&self, idx: ComponentIndex) -> InputDataContextView {
        InputDataContextView {
            constraint_storage: self.constraint_storage,
            witness: self.witness,
            signal_name_map: self.signal_name_map,
            tree_constraints: self.tree_constraints.subcomponents.get(idx).unwrap(),
        }
    }

    pub fn is_signal_public(&self, signal: ConstraintIndex) -> bool {
        let initial_signal = self.tree_constraints.initial_signal;
        let number_inputs = self.tree_constraints.number_inputs;
        let number_outputs = self.tree_constraints.number_outputs;

        signal >= initial_signal + number_outputs && signal < initial_signal + number_outputs + number_inputs
    }
}

/* Printer functions to print parsed Input Data */

pub fn print_constraint(c: &Constraint<ConstraintIndex>) {
    println!("Linear Expression A:");
    for c2 in c.a() {
        println!("     Signal: {:}", c2.0);
        println!("     Value : {:}", c2.1.to_string());
    }
    println!("Linear Expression B:");
    for c2 in c.b() {
        println!("     Signal: {:}", c2.0);
        println!("     Value : {:}", c2.1.to_string());
    }
    println!("Linear Expression C: ");
    for c2 in c.c() {
        println!("     Signal: {:}", c2.0);
        println!("     Value : {:}", c2.1.to_string());
    }
}

pub fn print_constraint_storage(storage: &ConstraintStorage) {
    for id in storage.get_ids() {
        let constraint = storage.read_constraint(id).unwrap();
        println!("\nConstraint ID: {id}");
        print_constraint(&constraint);
    }
}

pub fn print_witness(witness: &Witness) {
    for (id, val) in witness {
        println!("Id: {id}, val: {val}");
    }
}

pub fn print_signal_name_map(map: &SignalNameMap) {
    for (id, name) in map {
        println!("Id: {id}, name: '{name}'");
    }
}

pub fn print_tree_constraints(tree_constraints: &TreeConstraints) {
    println!("{}", serde_json::to_string(&tree_constraints).unwrap());
}
