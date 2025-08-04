use ::serde::{Deserialize, Serialize};
use rustc_hash::{FxHashMap, FxHashSet};
use egraph_serialize::*;
use anyhow::Context;
use crate::aig::*;
use crate::Lit;
#[derive(Deserialize, Debug)]
pub struct Node {
    pub op: String,
    pub children: Vec<String>,
    pub eclass: String,
    pub cost: f64,
    pub order: Option<usize>, // 新增字段，记录排序顺序，默认是 None
}

#[derive(Deserialize, Debug)]
pub struct Graph {
    pub nodes: FxHashMap<String, Node>,
    pub root_eclasses: Vec<String>,
}
pub fn parse_json_sd(json_str: &str) -> Graph {
    serde_json::from_str(json_str).expect("JSON was not well-formatted")
}
pub fn parse_egraph_from_str(json_str: &str) -> egraph_serialize::EGraph {
    serde_json::from_str(json_str)
        .with_context(|| "Failed to parse egraph from JSON string".to_string())
        .unwrap()
}
pub fn generate_input_vec(graph: &Graph) -> Vec<String> {
    // Step 1: Collect all nodes where children are empty
    let mut leaf_nodes: Vec<String> = graph
        .nodes
        .iter()
        .filter(|(_, node)| node.children.is_empty()) // Filter nodes with empty children
        .map(|(_, node)| node.op.clone()) // Collect their `op`
        // .filter(|op| op != "0" && op != "1") // Exclude "0" and "1"
        .filter(|op| op != "n0" ) // Exclude "0" and "1"
        .collect();

    // Step 2: Sort the leaf nodes based on the numeric part of the `op`
    leaf_nodes.sort_by_key(|op| {
        // Extract the numeric part of the `op`, assuming it's in the format like "PI000"
        op.chars()
            .filter(|c| c.is_numeric()) // Keep only numeric characters
            .collect::<String>() // Collect them into a string
            .parse::<usize>() // Parse the numeric part as a number
            .unwrap_or(usize::MAX) // Fallback to a large value if parsing fails
    });

    // Step 3: Return the sorted list
    leaf_nodes
}
impl Graph {
    pub fn to_aig<L: Lit>(&self) -> Aig<L> {
        Aig::from_graph(self)
    }
}
impl Graph {
    pub fn reorder(&self, input_vec: Vec<String>) -> Graph {
        let mut new_id_map: FxHashMap<String, String> = FxHashMap::default();
        let mut used_ids = FxHashSet::default();
        let mut next_id = 1;
        let mut order = 1;
        for (old_id, node) in &self.nodes {
            if node.op == "n0" {
                let new_id = "0".to_string(); // Assign ID "0" for node.op == "n0"
                new_id_map.insert(old_id.clone(), new_id.clone());
                used_ids.insert(new_id.clone());
                // println!("Processed node with op=n0: old_id={}, new_id={}", old_id, new_id);
                break; // 假设只有一个节点满足 op == "n0"，如有多个，可移除此行
            }
        }

        // Step 1: Assign new IDs to nodes in input_vec (keep their order)
        for input in &input_vec {
            for (old_id, node) in &self.nodes {
                if node.op == *input && !new_id_map.contains_key(old_id) {
                    let new_id = next_id.to_string();
                    new_id_map.insert(old_id.clone(), new_id.clone());
                    used_ids.insert(new_id.clone());
                    next_id += 1;
                    break;
                }
            }
        }

        // Step 3: Perform bottom-up DFS from the determined root nodes
        for root in &self.root_eclasses {
  
     
                self.dfs_reorder(root, &mut new_id_map, &mut used_ids, &mut next_id);

            } 

        // Step 4: Update the graph with new IDs and assign `order`
        let mut reordered_nodes = FxHashMap::default();
        for (old_id, node) in &self.nodes {
            if let Some(new_id) = new_id_map.get(&node.eclass) {
                let new_children: Vec<String> = node
                .children
                .iter()
                .filter_map(|child| {
                    if let Some(new_id) = new_id_map.get(child) {
                        Some(new_id.clone())
                    } else {
                        println!("Warning: Child ID `{}` not found in new_id_map", child);
                        None
                    }
                })
                .collect();

                reordered_nodes.insert(
                    new_id.clone(),
                    Node {
                        op: node.op.clone(),
                        children: new_children,
                        eclass: new_id.clone(),
                        cost: node.cost,
                        order: Some(order), // 设置排序顺序
                    },
                );

                order += 1; // Increment order for the next node
            }
        }

        // Debug: 打印 root_eclasses 的映射
        for root in &self.root_eclasses {
            if let Some(new_id) = new_id_map.get(root) {
                println!("Root eclass: {}, New ID: {}", root, new_id);
            } else {
                println!("Warning: Root eclass `{}` not found in new_id_map", root);
            }
        }
        
        // Return reordered graph
        Graph {
            nodes: reordered_nodes,
            root_eclasses: self
                .root_eclasses
                .iter()
                .filter_map(|root| {
                    if let Some(new_id) = new_id_map.get(root) {
                        Some(new_id.clone())
                    } else {
                        println!("Warning: Root eclass `{}` not found in new_id_map", root);
                        None
                    }
                })
                .collect(),
        }
    }

    fn dfs_reorder(
        &self,
        current: &String,
        new_id_map: &mut FxHashMap<String, String>,
        used_ids: &mut FxHashSet<String>,
        next_id: &mut usize,
    ) {
        if new_id_map.contains_key(current) {
            return; // Skip already visited nodes
        }

        // Visit children recursively first (bottom-up traversal)
        if let Some(node) = self.nodes.get(current) {
            for child in &node.children {
                self.dfs_reorder(child, new_id_map, used_ids, next_id);
            }
        }

        // Assign a new ID to the current node after processing all its children
        let new_id = next_id.to_string();
        new_id_map.insert(current.clone(), new_id.clone());
        used_ids.insert(new_id.clone());
        *next_id += 1; // Increment to assign the next larger number
    }
}

impl Graph {
    pub fn filter_nodes_by_op(&mut self) {
        // 将节点按 ID 排序
        let mut sorted_nodes: Vec<_> = self.nodes.iter_mut().collect();
        sorted_nodes.sort_by_key(|(id, _)| id.parse::<usize>().unwrap());

        let mut order = 1; // 初始排序号

        for (_, node) in sorted_nodes {
            if node.op == "n0" {
                // 如果节点的 op 为 "n0"，将其 order 设置为 0
                node.order = Some(0);
            } else if node.op != "!" {
                // 其他节点按照正常逻辑设置顺序
                node.order = Some(order);
                order += 1;
            } else {
                // 如果节点的 op 为 "!"，将其 order 设置为 None
                node.order = None;
            }
        }
    }
    // 处理 `&` 节点并生成去重排序的值
    // pub fn process_and_nodes(&self) -> Vec<usize> {
    //     let mut result = Vec::new();

    //     for (_, node) in &self.nodes {
    //         if node.op == "&" {
    //             // 遍历 `&` 节点的子节点
    //             for child_id in &node.children {
    //                 if let Some(child_node) = self.nodes.get(child_id) {
    //                     if let Some(order) = child_node.order {
    //                         match child_node.op.as_str() {
    //                             "!" => result.push(order * 2 + 1), // 如果是 `!`，记录 `order * 2 + 1`
    //                             "*" => result.push(order * 2),     // 如果是 `*`，记录 `order * 2`
    //                             "&" => {}                         // 如果是 `&`，跳过
    //                             _ => {}                           // 其他情况无需处理
    //                         }
    //                     }
    //                 }
    //             }
    //         }
    //     }

    //     // 去重并排序
    //     result.sort_unstable(); // 按从小到大排序
    //     result.dedup();         // 去重
    //     result
    // }
}
