use super::*;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
//use rand::Rng;
use rand::prelude::IteratorRandom;

pub struct BottomUpExtractor;
impl Extractor for BottomUpExtractor {
    fn extract(&self, egraph: &EGraph, _roots: &[ClassId], cost_function: &str, random_prob: f64) -> ExtractionResult {
        let mut result = ExtractionResult::default();
        let mut costs = FxHashMap::<ClassId, Cost>::with_capacity_and_hasher(
            egraph.classes().len(),
            Default::default(),
        );
        let mut did_something = false;

        let use_bfs = true;
        // Perform topological sort
        let sorted_classes = if use_bfs {
            topological_sort_bfs(egraph)
        } else {
            topological_sort_dfs(egraph)
        };
        println!("Topological sort completed. Number of classes: {}", sorted_classes.len());


        loop {
            for class in egraph.classes().values() {
                for node in &class.nodes {
                    let cost = result.node_sum_cost(egraph, &egraph[node], &costs);
                    if &cost < costs.get(&class.id).unwrap_or(&INFINITY) {
                        result.choose(class.id.clone(), node.clone());
                        costs.insert(class.id.clone(), cost);
                        did_something = true;
                    }
                }
            }

            //println!("Costs: {:?}", costs);

            if did_something {
                did_something = false;
            } else {
                break;
            }
        }

        result
    }
}



pub struct SimulatedAnnealingExtractor;

impl Extractor for SimulatedAnnealingExtractor {
    fn extract(&self, egraph: &EGraph, _roots: &[ClassId], cost_function: &str, random_prob: f64) -> ExtractionResult {
        let mut rng = rand::thread_rng();
        let mut result = ExtractionResult::default();
        let mut costs = FxHashMap::<ClassId, Cost>::with_capacity_and_hasher(
            egraph.classes().len(),
            Default::default(),
        );

        let mut did_something = false;

        loop {
            // Initialize with a random extraction
            for class in egraph.classes().values() {
                // if let Some(random_node) = class.nodes.choose(&mut thread_rng()){
                for node in &class.nodes {
                    //result.choose(class.id.clone(), random_node.clone());
                    //println!("Cost for this class: {:?}", class.id);
                    // costs.insert(class.id.clone(), match cost_function {
                    //     "node_sum_cost" => result.node_sum_cost(egraph, &egraph[random_node], &costs),
                    //     "node_depth_cost" => result.node_depth_cost(egraph, &egraph[random_node], &costs),
                    //     _ => panic!("Unknown cost function: {}", cost_function),
                    // });
                    //let cost = result.node_sum_cost(egraph, &egraph[node], &costs);
                    let cost = result.node_depth_cost(egraph, &egraph[node], &costs);
                    //println!("Cost for this class: {:?}", cost);
                    if &cost < costs.get(&class.id).unwrap_or(&INFINITY) {
                        result.choose(class.id.clone(), node.clone());
                        costs.insert(class.id.clone(), cost);
                        did_something = true;
                    }
                }
            }
            if did_something {
                did_something = false;
            } else {
                // if cost size is equal to egraph.classes().len() then break
                // if costs.len() == egraph.classes().len() {
                //     break;
                // }
                break;
            }
        }

        //println!("Initial costs: {:?}", costs);

        let initial_temp = 100.0;
        let cooling_rate = 0.8;
        let mut temperature = initial_temp;

        // sort the egraph.class with topo order
        
        while temperature > 1.0 {
            println!("Temperature: {}", temperature);
            for class in egraph.classes().values() {

                let current_cost = *costs.get(&class.id).unwrap_or(&INFINITY);

                // Choose a random neighbor
                if let Some(neighbor_node) = class.nodes.choose(&mut thread_rng()) {
                    let neighbor_cost = match cost_function {
                        "node_sum_cost" => result.node_sum_cost(egraph, &egraph[neighbor_node], &costs),
                        "node_depth_cost" => result.node_depth_cost(egraph, &egraph[neighbor_node], &costs),
                        _ => panic!("Unknown cost function: {}", cost_function),
                    };

                    // if neighbor_cost is larger than 1000000000.0 then break
                    if neighbor_cost > NotNan::new(1000000000.0).unwrap() {
                        break;
                    }

                    // Create a temporary result with the new choice
                    let mut temp_result = result.clone();
                    temp_result.choose(class.id.clone(), neighbor_node.clone());

                    // Check for cycles
                    let cycles = temp_result.find_cycles(egraph, &egraph.root_eclasses);

                    // create a empty vector for cycles
                    //let mut cycles: Vec<ClassId> = Vec::new();

                    // Only consider the change if it doesn't introduce cycles
                    if cycles.is_empty() && (neighbor_cost < current_cost || 
                    (rng.gen::<f64>() < ((current_cost - neighbor_cost) / temperature).exp() && neighbor_cost < INFINITY)) {
                        result = temp_result;
                        costs.insert(class.id.clone(), neighbor_cost);
                    }
                }
            }

            temperature *= cooling_rate;
        }

        result
    }
}