use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::error::Error as StdError;
use rustc_hash::FxHashMap;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::fs::File;
use std::io::{BufRead, BufReader};
use rayon::prelude::*;

//==================================================
//Data Structures
//==================================================

#[derive(Debug, Deserialize, Serialize)]
struct Graph {
    nodes: FxHashMap<String, Node>,
    root_eclasses: Vec<String>,
}


#[derive(Debug, Deserialize, Serialize)]
struct Node {
    op: String,
    children: Vec<String>,
    eclass: String,
    cost: f64,
}

// ==================================================
// Step 1: Process JSON with Choices
// ==================================================

/// Processes JSON with choices to filter nodes
fn process_json_with_choices(
    extraction_result_json: &str,
    saturated_graph_json: &str,
) -> Result<String, Box<dyn StdError>> {
    let extraction_data: Value = serde_json::from_str(extraction_result_json)?;
    let saturated_graph: Graph = serde_json::from_str(saturated_graph_json)?;

    let choices: FxHashMap<String, String> = serde_json::from_value(extraction_data["choices"].clone())?;
    let values: HashSet<&str> = choices.values().map(|v| v.as_str()).collect();

    let new_nodes: FxHashMap<String, Node> = saturated_graph
        .nodes
        .into_iter()
        .filter(|(key, _)| values.contains(key.as_str()))
        .collect();

    let result = serde_json::json!({ "nodes": new_nodes });
    Ok(serde_json::to_string_pretty(&result)?)
}

// ==================================================
// Step 2: Simplify JSON Keys
// ==================================================
/// Simplifies keys in the input JSON
fn process_json_simplify_keys(input_json: &str) -> Result<String, Box<dyn StdError>> {
    let data: Value = serde_json::from_str(input_json)?;
    let mut new_nodes: FxHashMap<String, Node> = FxHashMap::default();

    if let Some(nodes) = data["nodes"].as_object() {
        for (key, value) in nodes {
            let new_key = key.split('.').next().unwrap().to_string();
            let mut node: Node = serde_json::from_value(value.clone())?;
            node.children = node.children
                .iter()
                .map(|child| child.split('.').next().unwrap().to_string())
                .collect();
            new_nodes.insert(new_key, node);
        }
    } else {
        return Err("Input JSON does not contain a 'nodes' object".into());
    }

    let result = serde_json::json!({ "nodes": new_nodes });
    Ok(serde_json::to_string_pretty(&result)?)
}

// ==================================================
// Step 3: Update Root Eclasses
// ==================================================

/// Updates root eclasses in the target JSON
fn update_root_eclasses(graph_json: &str, target_json: &str) -> Result<String, Box<dyn StdError>> {
    let source_data: Value = serde_json::from_str(graph_json)?;
    let mut target_data: Value = serde_json::from_str(target_json)?;

    let root_eclasses = source_data["root_eclasses"].as_array().unwrap_or(&Vec::new()).to_owned();
    target_data["root_eclasses"] = serde_json::json!(root_eclasses);

    Ok(serde_json::to_string_pretty(&target_data)?)
}

// ==================================================
// Step 4: Convert JSON to Equation Format
// ==================================================

/// Converts JSON representation to equation format
fn json_to_eqn(json_str: &str, prefix_mapping_path: &str, is_large: bool) -> Result<String, Box<dyn StdError>> {
    let graph: Graph = serde_json::from_str(json_str)?;

    if is_cyclic_graph(&graph.nodes) {
        return Err("The graph is cyclic.".into());
    }

    let prefix_mapping = read_prefix_mapping(prefix_mapping_path);
    //println!("prefix mapping: {:?}", prefix_mapping);
    let mut final_content = String::with_capacity(graph.root_eclasses.len() * 1000);

    for root in &graph.root_eclasses {
        let mut visited = FxHashMap::default();
        let mut visit_count = FxHashMap::default();

        let equation = dag_to_equations(&graph.nodes, root, &mut visited, &mut visit_count, is_large);

        let variables: Vec<String> = graph.nodes.values()
            .filter(|node| node.children.is_empty() && node.op != "1" && node.op != "0")
            .map(|node| node.op.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let parts: Vec<String> = equation.split('&').map(str::trim).map(String::from).collect();

        let content = generate_eqn_content(&variables, parts, "p", visited, &prefix_mapping);
        final_content.push_str(&content);
        final_content.push('\n');
    }

    Ok(final_content)
}

// ===================================================
// Helper functions for JSON to Equation Conversion
// ===================================================

// ===================================================
// Helper functions (in json2eqn): Check for Cycles
// ===================================================

/// Checks if the given graph contains cycles
fn is_cyclic_graph(nodes: &FxHashMap<String, Node>) -> bool {
    let mut visited = FxHashMap::default();
    let mut rec_stack = FxHashMap::default();

    nodes.keys().any(|node_id| {
        !visited.contains_key(node_id) && is_cyclic_util(nodes, node_id, &mut visited, &mut rec_stack)
    })
}

/// Helper function for is_cyclic_graph to perform depth-first search
fn is_cyclic_util(
    nodes: &FxHashMap<String, Node>,
    node_id: &str,
    visited: &mut FxHashMap<String, bool>,
    rec_stack: &mut FxHashMap<String, bool>,
) -> bool {
    visited.insert(node_id.to_string(), true);
    rec_stack.insert(node_id.to_string(), true);

    if let Some(node) = nodes.get(node_id) {
        for child_id in &node.children {
            if !visited.get(child_id).unwrap_or(&false) {
                if is_cyclic_util(nodes, child_id, visited, rec_stack) {
                    return true;
                }
            } else if *rec_stack.get(child_id).unwrap_or(&false) {
                return true;
            }
        }
    }

    rec_stack.remove(node_id);
    false
}

// ===================================================
// Helper functions (in json2eqn): Generate Unique IDs
// ===================================================

/// Generates a unique ID for a given string
fn string_to_unique_id(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

// ===================================================
// Helper functions (in json2eqn): Convert DAG to Equations
// ===================================================

/// Converts a DAG to equations
fn dag_to_equations(
    nodes: &FxHashMap<String, Node>,
    node_id: &str,
    visited: &mut FxHashMap<String, String>,
    visit_count: &mut FxHashMap<String, usize>,
    is_large: bool,
) -> String {
    *visit_count.entry(node_id.to_string()).or_insert(0) += 1;

    if let Some(expr) = visited.get(node_id) {
        return format!("new_n_{}", node_id);
    }

    let node = &nodes[node_id];
    let expression = match node.op.as_str() {
        "&" => {
            let mut result = String::with_capacity(node.children.len() * 20);
            for (i, child_id) in node.children.iter().enumerate() {
                if i > 0 {
                    result.push_str(" & ");
                }
                result.push_str(&dag_to_equations(nodes, child_id, visited, visit_count, is_large));
            }
            result
        }
        _ => {
            let operands: Vec<String> = node.children
                .iter()
                .map(|child_id| dag_to_equations(nodes, child_id, visited, visit_count, is_large))
                .collect();

            match operands.len() {
                0 => node.op.clone(),
                1 => format!("{}({})", node.op, operands[0]),
                2 => {
                    if is_large {
                        let (lhs, rhs) = (operands[0].clone(), operands[1].clone());
                        let (lhs_id, rhs_id) = (
                            if lhs.len() > 50 { Some(string_to_unique_id(&lhs)) } else { None },
                            if rhs.len() > 50 { Some(string_to_unique_id(&rhs)) } else { None },
                        );
                        if let Some(id) = lhs_id { visited.insert(id.to_string(), lhs.clone()); }
                        if let Some(id) = rhs_id { visited.insert(id.to_string(), rhs.clone()); }
                        format!(
                            "({} {} {})",
                            lhs_id.map_or_else(|| lhs, |id| format!("new_n_{}", id)),
                            node.op,
                            rhs_id.map_or_else(|| rhs, |id| format!("new_n_{}", id))
                        )
                    } else {
                        format!("({} {} {})", operands[0], node.op, operands[1])
                    }
                }
                _ => unreachable!(),
            }
        }
    };

    if visit_count[node_id] > 1 && (expression.contains(' ') || expression.contains('(')) {
        visited.insert(node_id.to_string(), expression.clone());
    }

    expression
}

// ===================================================
// Helper functions (in json2eqn): Read Prefix Mapping
// ===================================================

/// Reads the prefix mapping from a file
fn read_prefix_mapping(file_path: &str) -> FxHashMap<String, String> {
    //println!("file path: {}", file_path);
    let file = File::open(file_path).expect("Unable to open file");
    let reader = BufReader::new(file);
    let mut mapping = FxHashMap::default();

    for line in reader.lines() {
        let line = line.expect("Unable to read line");
        if line.starts_with("OUTORDER = ") {
            let parts: Vec<&str> = line
                .trim_start_matches("OUTORDER = ")
                .trim_end_matches(';')
                .split_whitespace()
                .collect();
            for (index, part) in parts.iter().enumerate() {
                mapping.insert(format!("p[{}]", index), part.to_string());
            }
            break;
        }
    }

    mapping
}


// ===================================================
// Helper function (in json2eqn): Equation Content Generation
// ===================================================

/// Generates the content for the equation file
fn generate_eqn_content(
    variables: &[String],
    parts: Vec<String>,
    f_prefix: &str,
    visited: FxHashMap<String, String>,
    prefix_mapping: &FxHashMap<String, String>,
) -> String {
    let mut content = format!("INORDER = {};\n", variables.join(" "));

    for (index, part) in parts.iter().enumerate() {
        let f_number = format!("{}[{}]", f_prefix, index);
        let mapped_prefix = prefix_mapping.get(&f_number).unwrap_or(&f_number);
        content.push_str(&format!("{} = {};\n", mapped_prefix, part));
    }

    let outorder: Vec<String> = (0..parts.len())
        .map(|i| format!("{}[{}]", f_prefix, i))
        .map(|f_number| prefix_mapping.get(&f_number).unwrap_or(&f_number).to_string())
        .collect();

    content.push_str(&format!("OUTORDER = {};\n", outorder.join(" ")));

    for (node_id, expr) in visited {
        content.push_str(&format!("new_n_{} = {};\n", node_id, expr));
    }

    content
}

// ==================================================
// Main Process: Circuit Conversion
// ==================================================

/// Processes the circuit conversion
pub fn process_circuit_conversion(
    extraction_result: &crate::ExtractionResult,
    saturated_graph_json: &str,
    prefix_mapping_path: &str,
    is_large: bool,
) -> Result<String, Box<dyn StdError>> {
    let dag_cost_json = extraction_result.dag_cost_json.as_ref()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "DAG cost JSON not found"))?;

    extraction_result_to_eqn(dag_cost_json, saturated_graph_json, prefix_mapping_path, is_large)
}

/// Converts extraction result to equation format
pub fn extraction_result_to_eqn(
    dag_cost_json: &str,
    saturated_graph_json: &str,
    prefix_mapping_path: &str,
    is_large: bool,
) -> Result<String, Box<dyn StdError>> {
    let processed_json = process_json_with_choices(dag_cost_json, saturated_graph_json)?;
    let simplified_json = process_json_simplify_keys(&processed_json)?;
    let final_json = update_root_eclasses(saturated_graph_json, &simplified_json)?;
    json_to_eqn(&final_json, prefix_mapping_path, is_large)
}

