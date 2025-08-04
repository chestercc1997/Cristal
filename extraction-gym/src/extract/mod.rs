// This module uses the following external crates:
// - indexmap: Provides the IndexMap data structure
// - rustc_hash: Provides the FxHashMap and FxHashSet data structures
// - rand: Provides random number generation functionality
// - serde: Provides serialization and deserialization functionality

use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use core::num;
use std::collections::HashMap;
use std::collections::VecDeque;

pub use crate::*;

pub mod bottom_up;
pub mod faster_bottom_up;
pub mod faster_greedy_dag;
pub mod global_greedy_dag;
pub mod greedy_dag;
mod circuit_conversion;
mod lib;
mod demo;
//mod build;
// pub mod sim_ann_based_bottom_up;
// pub mod sim_ann_based_faster_bottom_up;
use rand::Rng;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
#[cfg(feature = "ilp-cbc")]
pub mod ilp_cbc;

use tonic::Request;
use std::future::Future;
// use crate::vectorservice::vector_service_client::VectorServiceClient;
// use crate::vectorservice::CircuitFilesRequest;

// Extractor trait defines the interface for extracting a result from an EGraph
pub trait Extractor: Sync {
    // extract method takes an EGraph, roots, and cost_function as input
    // and returns an ExtractionResult
    fn extract(&self, egraph: &EGraph, roots: &[ClassId], cost_function: &str, random_prob: f64) -> ExtractionResult;

    // boxed method allows creating a boxed instance of the Extractor trait
    fn boxed(self) -> Box<dyn Extractor>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

// Add a new trait for async extraction
pub trait AsyncExtractor: Sync {
    fn extract_async<'a>(
        &'a self,
        egraph: &'a EGraph,
        roots: &'a [ClassId],
        cost_function: &'a str,
        random_prob: f64,
    ) -> impl Future<Output = ExtractionResult> + Send + 'a;
}

// MapGet trait defines a generic interface for getting a value from a map-like data structure
pub trait MapGet<K, V> {
    // get method takes a key of type K and returns an optional reference to the corresponding value of type V
    fn get(&self, key: &K) -> Option<&V>;
}

// Implement MapGet for HashMap
impl<K, V> MapGet<K, V> for HashMap<K, V>
where
    K: Eq + std::hash::Hash,
{
    fn get(&self, key: &K) -> Option<&V> {
        HashMap::get(self, key)
    }
}

// Implement MapGet for FxHashMap
impl<K, V> MapGet<K, V> for FxHashMap<K, V>
where
    K: Eq + std::hash::Hash,
{
    fn get(&self, key: &K) -> Option<&V> {
        FxHashMap::get(self, key)
    }
}

// Implement MapGet for IndexMap
impl<K, V> MapGet<K, V> for IndexMap<K, V>
where
    K: Eq + std::hash::Hash,
{
    fn get(&self, key: &K) -> Option<&V> {
        IndexMap::get(self, key)
    }
}

// ExtractionResult struct represents the result of an extraction operation
#[derive(Default, Clone, Deserialize, Serialize, PartialEq)]
pub struct ExtractionResult {
    // choices is an IndexMap that maps ClassId to NodeId
    pub choices: IndexMap<ClassId, NodeId>,
    #[serde(skip)]
    pub tree_cost_json: Option<String>,
    #[serde(skip)]
    pub dag_cost_json: Option<String>,
    // #[serde(skip)]
    // pub saturated_json: Option<String>,
}

// Cost_extract struct is an empty struct (placeholder)
pub struct Cost_extract {}

// Status enum represents the status of a node during cycle detection
#[derive(Clone, Copy)]
enum Status {
    Doing,
    Done,
}

#[derive(Serialize)]
struct Wrapper<T> {
    choices: T,
}


// Implement methods for ExtractionResult
impl ExtractionResult {
    // choose method inserts a mapping from ClassId to NodeId into the choices map
    pub fn choose(&mut self, class_id: ClassId, node_id: NodeId) {
        self.choices.insert(class_id, node_id);
    }

    // return the node ID for a given class ID
    pub fn get_node(&self, class_id: &ClassId) -> Option<&NodeId> {
        self.choices.get(class_id)
    }

    // find_cycles method finds cycles in the EGraph starting from the given roots
    // and returns a vector of ClassIds representing the cycles
    pub fn find_cycles(&self, egraph: &EGraph, roots: &[ClassId]) -> Vec<ClassId> {
        let mut status = IndexMap::<ClassId, Status>::default();
        let mut cycles = vec![];
        for root in roots {
            self.cycle_dfs(egraph, root, &mut status, &mut cycles)
        }
        cycles
    }

    // cycle_dfs method performs a depth-first search to detect cycles in the EGraph
    fn cycle_dfs(
        &self,
        egraph: &EGraph,
        class_id: &ClassId,
        status: &mut IndexMap<ClassId, Status>,
        cycles: &mut Vec<ClassId>,
    ) {
        match status.get(class_id).cloned() {
            Some(Status::Done) => (),
            Some(Status::Doing) => cycles.push(class_id.clone()),
            None => {
                status.insert(class_id.clone(), Status::Doing);
                // print classid if key is not found
                if !self.choices.contains_key(class_id) {
                    println!("Class ID not found: {:?}", class_id);
                }
                let node_id = &self.choices[class_id];
                let node = &egraph[node_id];
                for child in &node.children {
                    let child_cid = egraph.nid_to_cid(child);
                    self.cycle_dfs(egraph, child_cid, status, cycles)
                }
                status.insert(class_id.clone(), Status::Done);
            }
        }
    }

    // tree_cost method calculates the cost of the extracted tree
    pub fn tree_cost(&self, egraph: &EGraph, roots: &[ClassId]) -> Cost {
        let node_roots = roots
            .iter()
            .map(|cid| self.choices[cid].clone())
            .collect::<Vec<NodeId>>();
        self.tree_cost_rec(egraph, &node_roots, &mut HashMap::new())
    }

    // tree_cost_rec method recursively calculates the cost of the extracted tree
    fn tree_cost_rec(
        &self,
        egraph: &EGraph,
        roots: &[NodeId],
        memo: &mut HashMap<NodeId, Cost>,
    ) -> Cost {
        let mut cost = Cost::default();
        for root in roots {
            if let Some(c) = memo.get(root) {
                cost += *c;
                continue;
            }
            let class = egraph.nid_to_cid(root);
            let node = &egraph[&self.choices[class]];
            let inner = node.cost + self.tree_cost_rec(egraph, &node.children, memo);
            memo.insert(root.clone(), inner);
            cost += inner;
        }
        cost
    }

    // dag_cost method calculates the cost of the extracted directed acyclic graph (DAG)
    pub fn dag_cost(&self, egraph: &EGraph, roots: &[ClassId]) -> Cost {
        let mut costs: IndexMap<ClassId, Cost> = IndexMap::new();
        let mut todo: Vec<ClassId> = roots.to_vec();
        while let Some(cid) = todo.pop() {
            let node_id = &self.choices[&cid];
            let node = &egraph[node_id];
            if costs.insert(cid.clone(), node.cost).is_some() {
                continue;
            }
            for child in &node.children {
                todo.push(egraph.nid_to_cid(child).clone());
            }
        }
        costs.values().sum()
    }

    // dag_cost_with_extraction_result method calculates the cost of the extracted DAG
    // and returns the cost along with the extraction result
    pub fn calculate_dag_cost_with_extraction_result(
        &self,
        egraph: &EGraph,
        roots: &[ClassId],
    ) -> (Cost, ExtractionResult) {
        let mut costs: IndexMap<ClassId, Cost> = IndexMap::new();
        let mut todo: Vec<ClassId> = roots.to_vec();
        let mut extraction_result = ExtractionResult {
            choices: IndexMap::new(),
            tree_cost_json: None,
            dag_cost_json: None,
           // saturated_json: None,
        };

        while let Some(cid) = todo.pop() {
            let node_id = &self.choices[&cid];
            let node = &egraph[node_id];
            if costs.insert(cid.clone(), node.cost).is_some() {
                continue;
            }
            extraction_result
                .choices
                .insert(cid.clone(), node_id.clone());
            for child in &node.children {
                todo.push(egraph.nid_to_cid(child).clone());
            }
        }

        let total_cost = costs.values().sum(); // calculate total cost

        (total_cost, extraction_result)
    }

    // node_sum_cost method calculates the sum of the costs of a node and its children
    pub fn node_sum_cost<M>(&self, egraph: &EGraph, node: &Node, costs: &M) -> Cost
    where
        M: MapGet<ClassId, Cost>,
    {
        node.cost
            + node
                .children
                .iter()
                .map(|n| {
                    let cid = egraph.nid_to_cid(n);
                    costs.get(cid).unwrap_or(&INFINITY)
                })
                .sum::<Cost>()
    }

    // node_depth_cost method calculates the maximum cost among a node and its children
    pub fn node_depth_cost<M>(&self, egraph: &EGraph, node: &Node, costs: &M) -> Cost
    where
        M: MapGet<ClassId, Cost>,
    {
        let child_max_cost = node
            .children
            .iter()
            .map(|n| {
                let cid = egraph.nid_to_cid(n);
                costs.get(cid).unwrap_or(&INFINITY)
            })
            .max()
            .copied()
            .unwrap_or(Cost::default());

        node.cost + child_max_cost
    }

    // record_costs_random method records the costs of random extractions
    pub fn record_costs_random(
        &self,
        num_runs: u32,
        random_ratio: f64,
        egraph: &EGraph,
        dag_cost_with_extraction_result: &ExtractionResult,
    ) {
        let n2c = |nid: &NodeId| egraph.nid_to_cid(nid);

        for num in 0..num_runs {
            // dump dag_cost_with_extraction_result to file
            
            let mut result: FxHashMap<ClassId, NodeId> = FxHashMap::default();
            let mut selected_ids: FxHashSet<ClassId> = HashSet::default(); // used to track selected nodes
            for classid in dag_cost_with_extraction_result.choices.keys() {
                let class = egraph.classes().get(classid).unwrap();
                let nodes = class.nodes.clone();
                let mut rng = rand::thread_rng();

                if !selected_ids.contains(&classid)
                    && rng.gen::<f64>() <= random_ratio
                    && nodes.len() > 1
                {
                    let random_index = rng.gen_range(0..nodes.len());
                    result.insert(classid.clone(), class.nodes[random_index].clone());

                    // Add the selected node ID to the set
                    selected_ids.insert(classid.clone());
                } else {
                    result.insert(
                        class.id.clone(),
                        dag_cost_with_extraction_result.choices[classid].clone(),
                    );
                }
            }

            let filename = format!("result{}.json", num);
            let path = format!("random_result/{}", filename);

            // Create directory if it doesn't exist
            if let Err(err) = fs::create_dir_all("random_result") {
                eprintln!("Failed to create directory: {}", err);
                continue; // Skip current iteration if directory creation fails
            }

            let wrapped_result = Wrapper {
                choices: result,
            };
        

            if let Ok(mut file) = File::create(path) {
                match to_string_pretty(&wrapped_result) {
                    Ok(json_dag_result) => {
                        let _ = write!(file, "{}", json_dag_result);
                    },
                    Err(e) => eprintln!("Failed to serialize data: {}", e),
                }
            }

        }
    }
    // // return extracted_egraph
    pub fn get_extracted_egraph(&self, egraph: &EGraph) -> EGraph {
        let mut extracted_egraph = EGraph::default();

        for (class_id, node_id) in &self.choices {
            if let Some(node) = egraph.nodes.get(node_id) {
                extracted_egraph.add_node(node_id.clone(), node.clone());

                // Add class data if it exists
                if let Some(class_data) = egraph.class_data.get(class_id) {
                    extracted_egraph.class_data.insert(class_id.clone(), class_data.clone());
                }
            }
        }

        // Set root e-classes
        extracted_egraph.root_eclasses = egraph.root_eclasses.clone();

        extracted_egraph
    }
}

fn topological_sort_dfs(egraph: &EGraph) -> Vec<ClassId> {
    println!("Starting DFS-based topological sort");
    let mut sorted = Vec::new();
    let mut visited = FxHashMap::default();
    let mut stack = Vec::new();

    fn dfs(egraph: &EGraph, class_id: &ClassId, visited: &mut FxHashMap<ClassId, bool>, stack: &mut Vec<ClassId>) {
        visited.insert(class_id.clone(), true);

        if let Some(class) = egraph.classes().get(class_id) {
            for node in &class.nodes {
                for child in &egraph[node].children {
                    let child_class_id = egraph.nid_to_cid(child);
                    if !visited.contains_key(child_class_id) {
                        dfs(egraph, child_class_id, visited, stack);
                    }
                }
            }
        }

        stack.push(class_id.clone());
    }

    for class_id in egraph.classes().keys() {
        if !visited.contains_key(class_id) {
            dfs(egraph, class_id, &mut visited, &mut stack);
        }
    }

    while let Some(class_id) = stack.pop() {
        sorted.push(class_id.clone());
    }

    println!("DFS-based topological sort completed");
    sorted
}

fn topological_sort_bfs(egraph: &EGraph) -> Vec<ClassId> {
    println!("Starting BFS-based topological sort");
    let mut sorted = Vec::new();
    let mut in_degree = FxHashMap::default();
    let mut queue = VecDeque::new();

    // Initialize in-degree for all classes
    for class_id in egraph.classes().keys() {
        in_degree.insert(class_id.clone(), 0);
    }

    // Calculate in-degree for each class
    for class in egraph.classes().values() {
        for node in &class.nodes {
            for child in &egraph[node].children {
                let child_class_id = egraph.nid_to_cid(child);
                *in_degree.entry(child_class_id.clone()).or_insert(0) += 1;
            }
        }
    }

    // Enqueue all classes with in-degree 0
    for (class_id, degree) in &in_degree {
        if *degree == 0 {
            queue.push_back(class_id.clone());
        }
    }

    // BFS
    while let Some(class_id) = queue.pop_front() {
        sorted.push(class_id.clone());

        if let Some(class) = egraph.classes().get(&class_id) {
            for node in &class.nodes {
                for child in &egraph[node].children {
                    let child_class_id = egraph.nid_to_cid(child);
                    if let Some(degree) = in_degree.get_mut(child_class_id) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(child_class_id.clone());
                        }
                    }
                }
            }
        }
    }

    println!("BFS-based topological sort completed");
    sorted
}