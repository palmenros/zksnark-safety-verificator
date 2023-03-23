use std::{collections::{HashMap, LinkedList}, io};
use std::fs::File;
use std::error::Error;
use std::io::BufRead;
use std::path::Path;
use itertools::Itertools;
use num_bigint_dig::BigInt;
use serde_json::Value;
use circom_algebra::algebra::Constraint;
use circom_algebra::constraint_storage::ConstraintStorage;
use serde::{Serialize, Deserialize};


pub fn parse_constraint_list(path: &Path) -> Result<ConstraintStorage, Box<dyn Error>> {
    let f = File::open(path)?;
    let data : Value = serde_json::from_reader(f)?;

    if let Value::Object(o) = data {
        let maybe_json_constraint_list = o.get("constraints");

        let json_constraint_list;
        match maybe_json_constraint_list {
            None => {
                return Err("constraint.json main object does not contain a constraints array".into());
            }
            Some(e) => {json_constraint_list = e;}
        }

        if let Value::Array(v) = json_constraint_list {
            let mut storage = ConstraintStorage::new();

            for val in v {
                // Read one constraint
                if let Value::Array(arr) = val {
                    if arr.len() != 3 {
                        return Err("Constraint in constraint.json has more than 3 terms".into());
                    }

                    let maybe_cs : Result<Vec<_>, _> = arr.into_iter().map(|x| -> Result<HashMap<usize, BigInt>, Box<dyn Error>> {
                        if let Value::Object(m) = x {
                            m.into_iter()
                                .map(|(k, v)| -> Result<(usize, BigInt), Box<dyn Error>> {
                                    if let Value::String(s) = v {
                                        Ok((k.parse::<usize>()?, s.parse::<BigInt>()?))
                                    } else {
                                        Err("Coefficient in 'constraint.json' is not a string".into())
                                    }
                                }).collect()
                        } else {
                            return Err("Constraint in 'constraint.json' has a non-object".into());
                        }
                    }).collect();

                    let cs = maybe_cs?;

                    storage.add_constraint(Constraint::new(cs[0].clone(), cs[1].clone(), cs[2].clone()));
                } else {
                    return Err("constraint.json contains a non-array in constraint list".into());
                }
            }
            return Ok(storage);
        } else {
            return Err("constraint.json 'constraints' value is not an array".into());
        }
    } else {
        return Err("constraint.json main value is not an object".into());
    }
}

type Witness = HashMap<usize, BigInt>;

pub fn parse_witness(path: &Path) -> Result<Witness, Box<dyn Error>> {
    let f = File::open(path)?;
    let data : Value = serde_json::from_reader(f)?;

    if let Value::Object(o) = data {
        let map = o.into_iter().map(|(k, v) : (String, Value)| -> Result<(usize, BigInt), Box<dyn Error>> {
            if let Value::String(s) = v {
                Ok((k.parse::<usize>()?, s.parse::<BigInt>()?))
            } else {
                Err("witness.json has a witness value that is not a string".into())
            }
        }).collect::<Result<Witness, Box<dyn Error>>>()?;
        return Ok(map);
    } else {
        return Err("witness.json main value is not an object".into());
    }
}

type SignalNameMap = HashMap<usize, String>;

pub fn parse_signal_name_map(path: &Path) -> Result<SignalNameMap, Box<dyn Error>> {
    let f = File::open(path)?;
    let mut map = SignalNameMap::new();

    for maybe_line in io::BufReader::new(f).lines() {
        let line = maybe_line.unwrap();
        let (id, _, _, name) = line.split(',').collect_tuple().unwrap();
        map.insert(id.parse::<usize>()?, name.to_string());
    }

    Ok(map)
}

#[derive(Default, Deserialize, Serialize)]
pub struct TreeConstraints {
    no_constraints: usize,
    initial_constraint: usize,
    node_id: usize,
    template_name: String,
    number_inputs: usize,
    number_outputs: usize,
    number_signals: usize,
    initial_signal: usize,
    are_double_arrow: Vec<(usize, usize)>, // first number constraint, second number assigned signal
    subcomponents: LinkedList<TreeConstraints>,
}

pub fn parse_tree_constraints(path: &Path) -> Result<TreeConstraints, Box<dyn Error>> {
    let f = File::open(path)?;
    let constraints : TreeConstraints = serde_json::from_reader(f)?;

    Ok(constraints)
}

pub fn print_constraint(c: &Constraint<usize>) {
    println!("Linear Expression A:");
    for c2 in c.a(){
        println!("     Signal: {:}",c2.0);
        println!("     Value : {:}",c2.1.to_string());
    }
    println!("Linear Expression B:");
    for c2 in c.b(){
        println!("     Signal: {:}",c2.0);
        println!("     Value : {:}",c2.1.to_string());
    }
    println!("Linear Expression C: ");
    for c2 in c.c(){
        println!("     Signal: {:}",c2.0);
        println!("     Value : {:}",c2.1.to_string());
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
