use circom_algebra::algebra::Constraint;
use circom_algebra::constraint_storage::ConstraintStorage;
use itertools::Itertools;
use num_bigint_dig::BigInt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::io::BufRead;
use std::path::Path;
use std::{
    collections::{HashMap, LinkedList},
    io,
};

pub fn parse_constraint_list(path: &Path) -> Result<ConstraintStorage, Box<dyn Error>> {
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
            .into_iter()
            .map(|x| -> Result<HashMap<usize, BigInt>, Box<dyn Error>> {
                let m = x
                    .as_object()
                    .ok_or("Constraint in 'constraint.json' has a non-object")?;
                m.into_iter()
                    .map(|(k, v)| -> Result<(usize, BigInt), Box<dyn Error>> {
                        let s = v
                            .as_str()
                            .ok_or("Coefficient in 'constraint.json' is not a string")?;
                        Ok((k.parse::<usize>()?, s.parse::<BigInt>()?))
                    })
                    .collect()
            })
            .collect();

        let (A, B, C) = maybe_cs?.into_iter().collect_tuple().unwrap();
        storage.add_constraint(Constraint::new(A, B, C));
    }

    Ok(storage)
}

pub type Witness = HashMap<usize, BigInt>;

pub fn parse_witness(path: &Path) -> Result<Witness, Box<dyn Error>> {
    let f = File::open(path)?;
    let data: Value = serde_json::from_reader(f)?;

    let o = data
        .as_object()
        .ok_or("witness.json main value is not an object")?;
    let map = o
        .into_iter()
        .map(|(k, v)| -> Result<(usize, BigInt), Box<dyn Error>> {
            let s = v
                .as_str()
                .ok_or("witness.json has a witness value that is not a string")?;
            Ok((k.parse::<usize>()?, s.parse::<BigInt>()?))
        })
        .collect::<Result<Witness, Box<dyn Error>>>()?;

    Ok(map)
}

pub type SignalNameMap = HashMap<usize, String>;

pub fn parse_signal_name_map(path: &Path) -> Result<SignalNameMap, Box<dyn Error>> {
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
        map.insert(id.parse::<usize>()?, name.to_string());
    }

    Ok(map)
}

#[derive(Default, Deserialize, Serialize)]
pub struct TreeConstraints {
    /* prime number corresponding to the field Z_p*/
    pub field: String,
    pub no_constraints: usize,
    pub initial_constraint: usize,
    pub node_id: usize,
    pub template_name: String,
    pub component_name: String,
    pub number_inputs: usize,
    pub number_outputs: usize,
    pub number_signals: usize,
    pub initial_signal: usize,
    pub are_double_arrow: Vec<(usize, usize)>,
    // first number constraint, second number assigned signal
    pub subcomponents: LinkedList<TreeConstraints>,
}

pub fn parse_tree_constraints(path: &Path) -> Result<TreeConstraints, Box<dyn Error>> {
    let f = File::open(path)?;
    let constraints: TreeConstraints = serde_json::from_reader(f)?;

    Ok(constraints)
}

pub fn print_constraint(c: &Constraint<usize>) {
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
