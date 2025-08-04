
use ::serde::{Deserialize, Serialize};
use egg::*;
use serde::__private::fmt::Display;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::fs::{OpenOptions};
use std::io::{BufWriter, Write};
// use crate::Graph;
use anyhow::Context;
use indexmap::IndexMap;
use std::collections::{HashSet, VecDeque};
use egraph_serialize::*;
use rustc_hash::FxHashMap;
use crate::language::*;
#[derive(Deserialize, Debug,Clone)]

pub struct Node {
    pub op: String,
    pub children: Vec<String>,
    pub eclass: String,
    pub cost: f64,
}

#[derive(Deserialize, Debug,Clone)]
pub struct Graph {
    pub nodes: FxHashMap<String, Node>,
    pub root_eclasses: Vec<String>,
}
pub fn print_filtered_nodes(graph: &Graph) {
    for node in graph.nodes.values() {
        if !matches!(node.op.as_str(), "!" | "+" | "*" | "0" | "1") {
            println!("{:?}", node);
        }
    }
}

pub fn parse_json(json_str: &str) -> Graph {
    serde_json::from_str(json_str).expect("JSON was not well-formatted")
}
pub fn parse_egraph_from_str(json_str: &str) -> egraph_serialize::EGraph {
    serde_json::from_str(json_str)
        .with_context(|| "Failed to parse egraph from JSON string".to_string())
        .unwrap()
}

pub fn save_egraph_to_json(egraph: &egg::EGraph<Prop, ConstantFold>, file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let json_rep = serde_json::to_string_pretty(&egraph).unwrap();
    fs::write(&file_path, json_rep)?;
    Ok(())
}

pub fn save_serialized_egraph_to_json(serialized_egraph: &egraph_serialize::EGraph, file_path: &PathBuf, root_ids: &[usize]) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(file_path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &serialized_egraph)?;

    let root_eclasses_value: serde_json::Value = root_ids
        .iter()
        .map(|id| serde_json::Value::String(id.to_string()))
        .collect();

    let json_string = std::fs::read_to_string(file_path)?;
    let mut json_data: serde_json::Value = serde_json::from_str(&json_string)?;
    json_data["root_eclasses"] = root_eclasses_value;

    let file = File::create(file_path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &json_data)?;

    Ok(())
}


pub fn egg_to_serialized_egraph<L, A>(egraph: &egg::EGraph<L, A>) -> egraph_serialize::EGraph
where
    L: Language + Display,
    A: Analysis<L>,
{
    use egraph_serialize::*;
    let mut out = EGraph::default();
    for class in egraph.classes() {
        for (i, node) in class.nodes.iter().enumerate() {
            out.add_node(
                format!("{}.{}", class.id, i),
                Node {
                    op: node.to_string(),
                    children: node
                        .children()
                        .iter()
                        .map(|id| NodeId::from(format!("{}.0", id)))
                        .collect(),
                    eclass: ClassId::from(format!("{}", class.id)),
                    cost: Cost::new(1.0).unwrap(),
                },
            )
        }
    }
    out
}

pub fn egg_to_serialized_egraph_wt<L, A>(egraph: &egg::EGraph<L, A>) -> egraph_serialize::EGraph
where
    L: Language + Display,
    A: Analysis<L>,
{
    use egraph_serialize::*;
    let mut out = EGraph::default();
    for class in egraph.classes() {
        for (i, node) in class.nodes.iter().enumerate() {
            out.add_node(
                format!("{}", class.id),
                Node {
                    op: node.to_string(),
                    children: node
                        .children()
                        .iter()
                        .map(|id| NodeId::from(format!("{}", id)))
                        .collect(),
                    eclass: ClassId::from(format!("{}", class.id)),
                    cost: Cost::new(1.0).unwrap(),
                },
            )
        }
    }
    out
}
pub fn process_json_prop_cost(json_str: &str) -> String {
    let mut data: Value = serde_json::from_str(&json_str).unwrap();

    if let Some(nodes) = data.get_mut("nodes").and_then(|nodes| nodes.as_object_mut()) {
        for node in nodes.values_mut() {
            let op = node["op"].as_str().unwrap();
            let cost = node["cost"].as_f64().unwrap();

            let new_cost = match op {
                "+" => 1000.0,
                "!" => 1.0,
                "*" => 4.0,
                // "+" => 1.0,
                // "!" => 1.0,
                // "*" => 1.0,
                _ => cost,
            };

            node["cost"] = serde_json::to_value(new_cost).unwrap();
        }
    }

    serde_json::to_string_pretty(&data).unwrap()
}

pub fn bottom_up_traversal(egraph: egraph_serialize::EGraph) -> Vec<String> {
    // Step 1: Initialize data structures.
    let mut parents = IndexMap::<ClassId, Vec<NodeId>>::with_capacity(egraph.classes().len());
    let mut pending_count = HashMap::<NodeId, usize>::new(); // Track unprocessed children for each node.
    let mut ready_queue = VecDeque::new(); // Nodes ready to process (all dependencies resolved).
    let mut traversal_order = Vec::new(); // Final traversal order.

    // Helper closure to map `NodeId` to `ClassId`.
    let n2c = |nid: &NodeId| egraph.nid_to_cid(nid);

    // Step 2: Initialize `parents` and `pending_count`.
    for class in egraph.classes().values() {
        parents.insert(class.id.clone(), Vec::new());

        for node in &class.nodes {
            // Initialize `pending_count` for each node.
            let child_count = egraph[node].children.len();
            pending_count.insert(node.clone(), child_count);

            // Populate `parents` map.
            for child in &egraph[node].children {
                parents.entry(n2c(child).clone()).or_default().push(node.clone());
            }

            // If the node has no children, it is ready to process.
            if child_count == 0 {
                ready_queue.push_back(node.clone()); // Leaf nodes are ready to process.
            }
        }
    }

    // Step 3: Process nodes in a bottom-up manner.
    while let Some(node) = ready_queue.pop_front() {
        // Add the node to the traversal order (it's now processed).
        traversal_order.push(node.clone());

        // Notify parents of this node.
        for parent in &parents[n2c(&node)] {
            // Decrement the pending count for the parent.
            if let Some(count) = pending_count.get_mut(parent) {
                *count -= 1;

                // If all children of the parent are processed, it's ready to process.
                if *count == 0 {
                    ready_queue.push_back(parent.clone());
                }
            }
        }
    }

    // Convert `NodeId` to `String` and return the traversal order.
    traversal_order.into_iter().map(|node_id| node_id.to_string()).collect()
}


pub fn dfs_traversal(graph: &Graph, start_node: &str) -> Vec<String> {
    let mut visited = HashSet::new();
    let mut result = Vec::new();

    // 定义递归函数
    fn dfs(node_id: &str, graph: &Graph, visited: &mut HashSet<String>, result: &mut Vec<String>) {
        // 如果当前节点已经访问过，直接返回
        if visited.contains(node_id) {
            return;
        }
        visited.insert(node_id.to_string());

        // 递归访问子节点
        if let Some(node) = graph.nodes.get(node_id) {
            for child_id in &node.children {
                dfs(child_id, graph, visited, result);
            }
        }

        // 子节点全部处理完后，将当前节点加入结果（后序遍历）
        result.push(node_id.to_string());
    }

    // 从起始节点开始 DFS
    dfs(start_node, graph, &mut visited, &mut result);

    result
}
// pub fn process_file_sig_output(graph: &Graph, output_file: &str) -> (egg::Id, Vec<Id>, i32) {
    pub fn process_file_sig_output(
        graph: &Graph,
        traversal_order: Vec<String>,
        output_file: &str,
    ) -> (egg::Id, Vec<Id>, i32) {
        let mut egraph: egg::EGraph<SymbolLang, ()> = egg::EGraph::default();
        let mut input_id: Vec<Id> = Vec::new();
        let mut count_out = 0;
        let mut one_out_sig = 0;
    
        // 用于记录原始 node_id 和对应的 egraph Id 的映射
        let mut node_id_to_egraph_id: HashMap<String, Id> = HashMap::new();
    
        // Add constants "0" and "1" to egraph
        let id0 = egraph.add(SymbolLang::leaf("n0"));
        let id1 = egraph.add(SymbolLang::new("Not",vec![id0]));
        // let id1 = egraph.add(SymbolLang::leaf("1"));
        node_id_to_egraph_id.insert("0".to_string(), id0);
        node_id_to_egraph_id.insert("1".to_string(), id1);
    
        // Add operator nodes (!, +, *)
        let mut or_count = 0;
        let mut and_count = 0;
        let mut not_count = 0;
    
        for node_id in traversal_order {
          //  println!("Processing node_id: {}", node_id); // 打印当前正在处理的 NodeId
    
            if let Some(node) = graph.nodes.get(&node_id) {
             //   println!("Found node: {:?}", node); // 打印找到的节点信息
    
                // Get child ids for the current node, 通过映射表获取真实的 egraph `Id`
                let child_ids: Vec<Id> = node
                    .children
                    .iter()
                    .filter_map(|child| {
                        if let Some(id) = node_id_to_egraph_id.get(child) {
                            Some(*id)
                        } else {
                            println!("Warning: Node {} not found in map", child);
                            None
                        }
                    })
                    .collect();
    
               // println!("Child IDs: {:?}", child_ids); // 打印子节点的 ID 列表
    
                // Match based on `node.op`
                match node.op.as_str() {
                    "!" => {
                        if child_ids.len() == 1 {
                            let id = egraph.add(SymbolLang::new("Not", vec![child_ids[0]]));
                            node_id_to_egraph_id.insert(node_id.clone(), id); // 保存映射关系
                            not_count += 1; // Increment Not counter
                            // println!(
                            //     "Added Not operator with child: {:?}, Id: {:?}",
                            //     child_ids[0], id
                            // );
                        } else {
                            println!("Error: Not operator expects 1 child, got {:?}", child_ids);
                        }
                    }
                    "+" => {
                        if child_ids.len() == 2 {
                            // Step 1: Add Not nodes for each child
                            let not_a = egraph.add(SymbolLang::new("Not", vec![child_ids[0]])); // !A
                            let not_b = egraph.add(SymbolLang::new("Not", vec![child_ids[1]])); // !B
                    
                            // Map these Not nodes back to the node_id_to_egraph_id
                    
                            // Step 2: Add And node for (!A AND !B)
                            let and_node = egraph.add(SymbolLang::new("And", vec![not_a, not_b])); // !A AND !B
                    
                            // Map this And node back to the node_id_to_egraph_id
                    
                            // Step 3: Add Not node for !( !A AND !B )
                            let or_equivalent = egraph.add(SymbolLang::new("Not", vec![and_node])); // !( !A AND !B )
                    
                            // Map the final OR equivalent node back to the original node_id
                            node_id_to_egraph_id.insert(node_id.clone(), or_equivalent);
                    
                            or_count += 1; // Increment Or counter
                        } else {
                            println!("Error: Or operator expects 2 children, got {:?}", child_ids);
                        }
                    }
                    "*" => {
                        if child_ids.len() == 2 {
                            let id =
                                egraph.add(SymbolLang::new("And", vec![child_ids[0], child_ids[1]]));
                            node_id_to_egraph_id.insert(node_id.clone(), id); // 保存映射关系
                            or_count += 1; // Increment Or counter
                            // println!(
                            //     "Added Or operator with children: {:?}, Id: {:?}",
                            //     child_ids, id
                            // );
                        } else {
                            println!("Error: Or operator expects 2 children, got {:?}", child_ids);
                        }
                    }
                    "&" => {
                        if child_ids.len() == 2 {
                            let id =
                                egraph.add(SymbolLang::new("Concat", vec![child_ids[0], child_ids[1]]));
                            node_id_to_egraph_id.insert(node_id.clone(), id); // 保存映射关系
                        
                            // println!(
                            //     "Added Or operator with children: {:?}, Id: {:?}",
                            //     child_ids, id
                            // );
                        } else {
                            println!("Error: Or operator expects 2 children, got {:?}", child_ids);
                        }
                    }
                    _ => {
                        // If not an operator, treat as a leaf and add to `egraph`
                        let id = egraph.add(SymbolLang::leaf(&node.op));
                        node_id_to_egraph_id.insert(node_id.clone(), id); // 保存映射关系
                        input_id.push(id); // Save the Id for later use
                      //  println!("Added leaf node: {}, Id: {:?}", node_id, id);
                    }
                }
            } else {
                println!("Node not found in graph.nodes: {}", node_id);
            }
        }
    
        // Print the counts after the loop
        // println!("Number of Or nodes: {}", or_count);
        // println!("Number of And nodes: {}", and_count);
        // println!("Number of Not nodes: {}", not_count);
    
        // // Print debugging information
        // println!("Input IDs: {:?}", input_id);
        // println!("Input IDs Length: {}", input_id.len());
        let root_id = if let Some(root_eclass) = graph.root_eclasses.get(0) {
            println!("Root eclass: {}", root_eclass);
            
            if let Some(egraph_id) = node_id_to_egraph_id.get(root_eclass) {
                let root_new = egraph.add(SymbolLang::new("Root", vec![*egraph_id]));
                
                // println!(
                //     "Root eclass found: Node ID: {}, EGraph ID: {:?}",
                //     root_eclass, egraph_id
                // );
                count_out += 1;
                one_out_sig = 1;
                root_new
            } else {
                println!("Error: Root eclass node_id not found in map");
                Id::from(0) // Default or invalid ID
            }
        } else {
            println!("Error: Root eclass not found");
            Id::from(0) // Default or invalid ID
        };
        // Rebuild the egraph
        egraph.rebuild();
    
        // Serialize the egraph to JSON and write to the output file
        let json_str = serde_json::to_string_pretty(&egraph).expect("Failed to serialize EGraph");
        fs::write(output_file, json_str).expect("Failed to write output JSON file");
    
        // Handle root eclass (moved to the end)
       
    
        (root_id, input_id, one_out_sig)
    }

    