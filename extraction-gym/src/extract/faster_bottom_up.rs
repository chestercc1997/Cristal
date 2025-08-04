use super::*;
use crate::extract::circuit_conversion::process_circuit_conversion;
use rand::prelude::*;
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::error::Error;
use std::env;
use std::process;
use tokio::runtime::Runtime;
use std::time::Instant;
//use abc::Abc;

// use crate::extract::lib::Abc;

use std::fs;
use std::future::Future;
use std::io::Write;
use tempfile::NamedTempFile;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

/// A faster bottom up extractor inspired by the faster-greedy-dag extractor.
/// It should return an extraction result with the same cost as the bottom-up extractor.
///
/// Bottom-up extraction works by iteratively computing the current best cost of each
/// node in the e-graph based on the current best costs of its children.
/// Extraction terminates when our estimates of the best cost for each node
/// reach a fixed point.
/// The baseline bottom-up implementation visits every node during each iteration
/// of the fixed point.
/// This algorithm instead only visits the nodes whose current cost estimate may change:
/// it does this by tracking parent-child relationships and storing relevant nodes
/// in a work list (UniqueQueue).
pub struct FasterBottomUpExtractor; // baseline faster bottom-up extractor
//pub struct FasterBottomUpExtractorGRPC; // extraction method based on faster bottom-up
pub struct FasterBottomUpExtractorRandom; // extraction method based on random extraction

// ========================================== Extractor Interface ==========================================
impl Extractor for FasterBottomUpExtractor {
    fn extract(
        &self,
        egraph: &EGraph,
        _roots: &[ClassId],
        cost_function: &str,
        random_prob: f64,
    ) -> ExtractionResult {
        let mut parents = IndexMap::<ClassId, Vec<NodeId>>::with_capacity(egraph.classes().len());
        let n2c = |nid: &NodeId| egraph.nid_to_cid(nid);
        let mut analysis_pending = UniqueQueue::default();

        for class in egraph.classes().values() {
            parents.insert(class.id.clone(), Vec::new());
        }

        for class in egraph.classes().values() {
            for node in &class.nodes {
                for c in &egraph[node].children {
                    // compute parents of this enode
                    parents[n2c(c)].push(node.clone());
                }

                // start the analysis from leaves
                if egraph[node].is_leaf() {
                    analysis_pending.insert(node.clone());
                }
            }
        }

        let mut result = ExtractionResult::default();
        let mut costs = FxHashMap::<ClassId, Cost>::with_capacity_and_hasher(
            egraph.classes().len(),
            Default::default(),
        );

        while let Some(node_id) = analysis_pending.pop() {
            let class_id = n2c(&node_id);
            let node = &egraph[&node_id];
            let prev_cost = costs.get(class_id).unwrap_or(&INFINITY);
            let cost = match cost_function {
                "node_sum_cost" => result.node_sum_cost(egraph, node, &costs),
                "node_depth_cost" => result.node_depth_cost(egraph, node, &costs),
                _ => panic!("Unknown cost function: {}", cost_function),
            };
            if cost < *prev_cost {
                result.choose(class_id.clone(), node_id.clone());
                costs.insert(class_id.clone(), cost);
                analysis_pending.extend(parents[class_id].iter().cloned());
            }
        }

        result
    }
}
impl Extractor for FasterBottomUpExtractorRandom {
    fn extract(
        &self,
        egraph: &EGraph,
        _roots: &[ClassId],
        cost_function: &str,
        random_prob: f64,
    ) -> ExtractionResult {
        let k = random_prob;
        let mut parents = IndexMap::<ClassId, Vec<NodeId>>::with_capacity(egraph.classes().len());
        let n2c = |nid: &NodeId| egraph.nid_to_cid(nid);
        let mut analysis_pending = UniqueQueue::default();

        for class in egraph.classes().values() {
            parents.insert(class.id.clone(), Vec::new());
        }

        for class in egraph.classes().values() {
            for node in &class.nodes {
                for c in &egraph[node].children {
                    // compute parents of this enode
                    parents[n2c(c)].push(node.clone());
                    //println!("Node: {:?}", node);
                }

                // start the analysis from leaves
                if egraph[node].is_leaf() {
                    analysis_pending.insert(node.clone());
                }
            }
        }

        let mut result = ExtractionResult::default();
        let mut costs = FxHashMap::<ClassId, Cost>::with_capacity_and_hasher(
            egraph.classes().len(),
            Default::default(),
        );
        let mut chosen_classes = HashSet::<ClassId>::new(); // 新增的 HashSet
        while let Some(node_id) = analysis_pending.pop() {
            let class_id = n2c(&node_id);
            let node = &egraph[&node_id];
            let prev_cost = costs.get(class_id).unwrap_or(&INFINITY);
            let cost = match cost_function {
                "node_sum_cost" => result.node_sum_cost(egraph, node, &costs),
                "node_depth_cost" => result.node_depth_cost(egraph, node, &costs),
                _ => panic!("Unknown cost function: {}", cost_function),
            };
            let mut rng = rand::thread_rng();
            let random_value: f64 = rng.gen();

            if  prev_cost ==&INFINITY  {
                result.choose(class_id.clone(), node_id.clone());
                costs.insert(class_id.clone(), cost);
                analysis_pending.extend(parents[class_id].iter().cloned());
            }
            // else if (cost <= *prev_cost) &&random_value>=k {
            //     result.choose(class_id.clone(), node_id.clone());
            //     costs.insert(class_id.clone(), cost);
            //     analysis_pending.extend(parents[class_id].iter().cloned());
            // }
            else if(cost <= *prev_cost)&&random_value>=k {
                    result.choose(class_id.clone(), node_id.clone());
                    costs.insert(class_id.clone(), cost);
                    analysis_pending.extend(parents[class_id].iter().cloned());
            }
            
        }

        result
    }
}

// impl Extractor for FasterBottomUpExtractorRandom {
//     fn extract(
//         &self,
//         egraph: &EGraph,
//         _roots: &[ClassId],
//         cost_function: &str,
//         random_prob: f64,
//     ) -> ExtractionResult {
//         let k = random_prob;
//         let mut parents = IndexMap::<ClassId, Vec<NodeId>>::with_capacity(egraph.classes().len());
//         let n2c = |nid: &NodeId| egraph.nid_to_cid(nid);
//         let mut analysis_pending = UniqueQueue::default();

//         for class in egraph.classes().values() {
//             parents.insert(class.id.clone(), Vec::new());
//         }

//         for class in egraph.classes().values() {
//             for node in &class.nodes {
//                 for c in &egraph[node].children {
//                     // compute parents of this enode
//                     parents[n2c(c)].push(node.clone());
//                     //println!("Node: {:?}", node);
//                 }

//                 // start the analysis from leaves
//                 if egraph[node].is_leaf() {
//                     analysis_pending.insert(node.clone());
//                 }
//             }
//         }

//         let mut result = ExtractionResult::default();
//         let mut costs = FxHashMap::<ClassId, Cost>::with_capacity_and_hasher(
//             egraph.classes().len(),
//             Default::default(),
//         );
//         let mut chosen_classes = HashSet::<ClassId>::new(); // 新增的 HashSet
//         while let Some(node_id) = analysis_pending.pop() {
//             let class_id = n2c(&node_id);
//             let node = &egraph[&node_id];
//             let prev_cost = costs.get(class_id).unwrap_or(&INFINITY);
//             let cost = match cost_function {
//                 "node_sum_cost" => result.node_sum_cost(egraph, node, &costs),
//                 "node_depth_cost" => result.node_depth_cost(egraph, node, &costs),
//                 _ => panic!("Unknown cost function: {}", cost_function),
//             };
//             let mut rng = rand::thread_rng();
//             let random_value: f64 = rng.gen();

//             if  prev_cost ==&INFINITY  {
//                 result.choose(class_id.clone(), node_id.clone());
//                 costs.insert(class_id.clone(), cost);
//                 analysis_pending.extend(parents[class_id].iter().cloned());
//             }else if (cost < *prev_cost) {
//                 result.choose(class_id.clone(), node_id.clone());
//                 costs.insert(class_id.clone(), cost);
//                 analysis_pending.extend(parents[class_id].iter().cloned());
//             }
//             else if(cost == *prev_cost)&&random_value>=k {
//                     result.choose(class_id.clone(), node_id.clone());
//                     costs.insert(class_id.clone(), cost);
//                     analysis_pending.extend(parents[class_id].iter().cloned());
//             }
            
//         }

//         result
//     }
// }
// impl Extractor for FasterBottomUpExtractorRandom {
//     fn extract(
//         &self,
//         egraph: &EGraph,
//         _roots: &[ClassId],
//         _cost_function: &str,
//         _random_prob: f64,
//     ) -> ExtractionResult {
//         let mut result = ExtractionResult::default();
//         let mut class_data: FxHashMap<ClassId, (usize, Option<NodeId>)> = FxHashMap::default();
//         let mut reverse_deps: FxHashMap<ClassId, Vec<NodeId>> = FxHashMap::default();
//         let mut remaining_deps: FxHashMap<NodeId, usize> = FxHashMap::default();
//         let mut queue = UniqueQueue::default();

//         // 初始化每个节点的子类集合和剩余依赖数
//         for class in egraph.classes().values() {
//             for node in &class.nodes {
//                 let children_classes: HashSet<ClassId> = egraph[node]
//                 .children
//                 .iter()
//                 .map(|n| egraph.nid_to_cid(n)) // This returns &ClassId
//                 .cloned()                      // Clone the &ClassId into an owned ClassId
//                 .collect();                    // Now collect into HashSet<ClassId>
//                 let deps_count = children_classes.len();
//                 remaining_deps.insert(node.clone(), deps_count);

//                 // 构建反向依赖映射
//                 for cid in children_classes {
//                     reverse_deps.entry(cid).or_default().push(node.clone());
//                 }

//                 // 叶子节点直接加入队列
//                 if deps_count == 0 {
//                     queue.insert(node.clone());
//                 }
//             }
//         }

//         // 处理队列中的节点
//         while let Some(node_id) = queue.pop() {
//             let class_id = egraph.nid_to_cid(&node_id);
//             let (count, selected) = class_data.entry(class_id.clone()).or_insert((0, None));

//             // 蓄水池抽样：第count+1个元素以1/(count+1)概率选中
//             *count += 1;
//             let r: f64 = rand::thread_rng().gen();
//             if r < 1.0 / (*count as f64) {
//                 *selected = Some(node_id.clone());
//                 result.choose(class_id.clone(), node_id.clone());
//             }

//             // 更新依赖该类的父节点的剩余依赖数
//             if let Some(dependers) = reverse_deps.get(&class_id) {
//                 for depender in dependers {
//                     if let Some(remaining) = remaining_deps.get_mut(depender) {
//                         *remaining -= 1;
//                         if *remaining == 0 {
//                             queue.insert(depender.clone());
//                         }
//                     }
//                 }
//             }
//         }

//         result
//     }
// }
// impl Extractor for FasterBottomUpSimulatedAnnealingExtractor {
//     fn extract(
//         &self,
//         egraph: &EGraph,
//         _roots: &[ClassId],
//         cost_function: &str,
//         random_prob: f64,
//     ) -> ExtractionResult {
//         let mut rng = thread_rng();
//         let saturated_graph_path = "input/rewritten_egraph_with_weight_cost_serd.json";
//         let prefix_mapping_path = "../e-rewriter/circuit0_opt.eqn";

//         let saturated_graph_json = fs::read_to_string(saturated_graph_path).unwrap_or_else(|e| {
//             eprintln!("Failed to read saturated graph file: {}", e);
//             String::new()
//         });

//         // Generate base solution using faster bottom-up
//         let mut base_result =
//             generate_initial_solution(egraph, InitialSolutionType::Base, cost_function, &mut rng);
//         update_json_buffers_in_result(&mut base_result, egraph);
//         let base_abc_cost = calculate_abc_cost_or_dump(
//             &base_result,
//             &saturated_graph_json,
//             &prefix_mapping_path,
//             false,
//         );

//         // Generate random initial solution for SA
//         let mut current_result =
//             generate_initial_solution(egraph, InitialSolutionType::Base, cost_function, &mut rng); // TODO: adjust here
//         update_json_buffers_in_result(&mut current_result, egraph);
//         let mut current_abc_cost = calculate_abc_cost_or_dump(
//             &current_result,
//             &saturated_graph_json,
//             &prefix_mapping_path,
//             false,
//         );

//         let initial_temp: f64 = 100.0;
//         let cooling_rate: f64 = 0.7;
//         let mut temperature: f64 = initial_temp;
//         let sample_size = (egraph.classes().len() as f64 * 0.3).max(1.0) as usize;
//         let iterations_per_temp = 2;
//         let min_temperature: f64 = 0.1;
//         let verbose = true;

//         // Calculate total iterations using logarithms
//         //let total_iterations = ((initial_temp / min_temperature).ln() / cooling_rate.ln()).ceil() as u64 * iterations_per_temp as u64;

//         // let total interation = ceil( (log(min_temp) - log(initial_temp))/log(cooling_rate) ) * iterations_per_temp
//         let total_iterations = ((min_temperature.ln() - initial_temp.ln()) / cooling_rate.ln())
//             .ceil() as u64
//             * iterations_per_temp as u64;
//         let mut best_result = current_result.clone();
//         let mut best_abc_cost = current_abc_cost;

//         let m = MultiProgress::new();
//         let pb = m.add(ProgressBar::new(total_iterations));
//         pb.set_style(
//             ProgressStyle::default_bar()
//                 .template(
//                     "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
//                 )
//                 .unwrap()
//                 .progress_chars("#>-"),
//         );

//         let panel = m.add(ProgressBar::new(1));
//         panel.set_style(
//             ProgressStyle::default_spinner()
//                 .template("{spinner:.green} {wide_msg}")
//                 .unwrap(),
//         );

//         println!("========== Starting Simulated Annealing ==========");
//         panel.set_message(format!(
//             "Base solution ABC cost: {:.6}. Initial solution's ABC cost: {:.6}",
//             base_abc_cost, current_abc_cost
//         ));

//         let mut iteration_count = 0;

//         while temperature > min_temperature {
//             for _ in 0..iterations_per_temp {
//                 let mut new_result = generate_neighbor_solution(
//                     &current_result,
//                     egraph,
//                     NeighborSolutionType::RandomExtraction,
//             // NeighborSolutionType:: SmartIntermediatePropagation,
//                     sample_size,
//                     &mut rng,
//                     cost_function,
//                     0.5,
//                     temperature,
//                     initial_temp,
//                 );
//                 update_json_buffers_in_result(&mut new_result, egraph);
//                 let new_abc_cost = calculate_abc_cost_or_dump(
//                     &new_result,
//                     &saturated_graph_json,
//                     &prefix_mapping_path,
//                     false,
//                 );

//                 let cost_change = new_abc_cost - current_abc_cost;

//                 if verbose {
//                     panel.set_message(format!(
//                         "Temp: {:.2}\nCurrent ABC cost: {:.6}\nNew ABC cost: {:.6}\nChange: {:.6}",
//                         temperature, current_abc_cost, new_abc_cost, cost_change
//                     ));
//                 }

//                 if cost_change <= 0.0 || rng.gen::<f64>() < (-cost_change / temperature).exp() {
//                     current_result = new_result;
//                     current_abc_cost = new_abc_cost;

//                     if current_abc_cost < best_abc_cost {
//                         best_result = current_result.clone();
//                         best_abc_cost = current_abc_cost;
//                         panel.println(format!(
//                             "New best solution found! Cost: {:.6}",
//                             best_abc_cost
//                         ));
//                     }
//                 }
//                 iteration_count += 1;
//                 pb.set_position(iteration_count);
//             }

//             temperature *= cooling_rate;
//         }

//         pb.finish_with_message("Simulated Annealing Complete");
//         panel.finish_with_message(format!(
//             "SA-final ABC cost: {:.6}\nBase solution ABC cost: {:.6}",
//             best_abc_cost, base_abc_cost
//         ));

//         // Compare SA-final with base solution
//         if best_abc_cost <= base_abc_cost {
//             println!("SA-final solution is better. Returning SA-final.");
//             // save the best result to file
//             _ = calculate_abc_cost_or_dump(
//                 &best_result,
//                 &saturated_graph_json,
//                 &prefix_mapping_path,
//                 true,
//             );
//             best_result
//         } else {
//             println!("Base solution is better. Returning base solution.");
//             // save the base result to file
//             _ = calculate_abc_cost_or_dump(
//                 &base_result,
//                 &saturated_graph_json,
//                 &prefix_mapping_path,
//                 true,
//             );
//             base_result
//         }
//     }
// }
// //=========================kkkkkkkk================================
// impl Extractor for FasterBottomUpFastSimulatedAnnealingExtractorml{
//     fn extract(
//         &self,
//         egraph: &EGraph,
//         roots: &[ClassId],
//         cost_function: &str,
//         random_prob: f64,
//     ) -> ExtractionResult {
//         // Create a new runtime for this extraction
//         let rt = Runtime::new().unwrap();
//         // Use the runtime to block on the async extraction
//         rt.block_on(self.extract_async(egraph, roots, cost_function, random_prob))
//     }
// }
// impl AsyncExtractor for FasterBottomUpFastSimulatedAnnealingExtractorml {
//     fn extract_async<'a>(
//         &'a self,
//         egraph: &'a EGraph,
//         roots: &'a [ClassId],
//         cost_function: &'a str,
//         random_prob: f64,
//     ) -> impl Future<Output = ExtractionResult> + Send + 'a {
//         let rng = Arc::new(Mutex::new(StdRng::from_entropy()));

//         async move {
//             let saturated_graph_path = "input/rewritten_egraph_with_weight_cost_serd.json";
//             let prefix_mapping_path = "../e-rewriter/circuit0_opt.eqn";

//             // Read the saturated graph JSON
//             let saturated_graph_json = fs::read_to_string(saturated_graph_path).unwrap_or_else(|e| {
//                 eprintln!("Failed to read saturated graph file: {}", e);
//                 String::new()
//             });

//             let mut base_result;
//             let base_abc_cost;

//             // Generate base solution
//             {
//                 let mut rng = rng.lock().unwrap(); // Lock the mutex for this scope
//                 base_result = generate_initial_solution(egraph, InitialSolutionType::Base, cost_function, &mut *rng);
//                 update_json_buffers_in_result(&mut base_result, egraph);
//                 base_abc_cost = calculate_abc_cost_or_dump(&base_result, &saturated_graph_json, &prefix_mapping_path, false);
//             }

//             // Generate random initial solution for SA
//             let mut current_result;
//             let mut current_abc_cost;

//             {
//                 let mut rng = rng.lock().unwrap(); // Lock the mutex again for this scope
//                 current_result = generate_initial_solution(egraph, InitialSolutionType::Base, cost_function, &mut *rng);
//                 update_json_buffers_in_result(&mut current_result, egraph);
//                 current_abc_cost = calculate_abc_cost_or_dump(&current_result, &saturated_graph_json, &prefix_mapping_path, false);
                
//             }

//             let initial_randomprob_sa = 0.95_f64;
//             let initial_temp: f64 = -1.0 * (current_abc_cost / initial_randomprob_sa.ln())/2.0;
//             let c = 1000.0;
//             let k = 4.0;

//             let mut temperature: f64 = initial_temp;
//             println!("initial_temp: {:.6}\n", temperature);
//             current_abc_cost = 100000000.0;
//             let sample_size = (egraph.classes().len() as f64 * 0.3).max(1.0) as usize;
//             let verbose = true;

//             let mut best_result = current_result.clone();

//             let mut best_abc_cost = 100000000.0;
//             let total_iterations = 10;

//             let m = MultiProgress::new();
//             let pb = m.add(ProgressBar::new(total_iterations));
//             pb.set_style(ProgressStyle::default_bar()
//                 .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
//                 .unwrap()
//                 .progress_chars("#>-"));

//             let panel = m.add(ProgressBar::new(1));
//             panel.set_style(ProgressStyle::default_spinner()
//                 .template("{spinner:.green} {wide_msg}")
//                 .unwrap());

//             println!("========== Starting Simulated Annealing ==========");
//             panel.set_message(format!(
//                 "Base solution ABC cost: {:.6}. Initial solution's ABC cost: {:.6}",
//                 base_abc_cost, current_abc_cost
//             ));

//             let mut iteration_count = 0.0;
//             let mut uphill = 0;
//             let mut total_calculate_time_1: std::time::Duration = std::time::Duration::new(0, 0);
//             let mut total_calculate_time_0: std::time::Duration = std::time::Duration::new(0, 0);
//             while iteration_count <= total_iterations as f64 && uphill <= total_iterations / 2 {
//                 let start_time = Instant::now();
//                 let mut new_result;

//                 {
//                     let mut rng = rng.lock().unwrap(); // Lock the mutex for the next operation
//                     new_result = 
//                     generate_neighbor_solution(&current_result, egraph, NeighborSolutionType::RandomExtraction, sample_size, &mut *rng, cost_function, 0.2, temperature, initial_temp);
//                 }
//                 let elapsed_time = Instant::now() - start_time;
//                 total_calculate_time_0 += elapsed_time;
//                 let start_time = Instant::now();
//                 update_json_buffers_in_result(&mut new_result, egraph);
                
//                 // Process the new result and send it to the server
//                 let mode = "small";
//                 //let mode = "large";
//                 let eqn_content = match process_circuit_conversion(&new_result, &saturated_graph_json, &prefix_mapping_path, mode == "small") {
//                     //let eqn_content = match process_circuit_conversion(&new_result, &saturated_graph_json, &prefix_mapping_path, mode == "large") {
//                     Ok(content) => content,
//                     Err(e) => {
//                         eprintln!("Error in circuit conversion: {}", e);
//                         return Default::default(); // or handle the error appropriately
//                     }
//                 };

//                 if let Err(e) = std::fs::write("src/extract/tmp/output.eqn", &eqn_content) {
//                     eprintln!("Error writing to file: {}", e);
//                 }
//                 let _ = call_abc_ml(&eqn_content);
//                 // Read files and send to the server
//                 let el_content = fs::read_to_string("src/extract/tmp/opt_1.el").expect("Failed to read el file");
//                 let csv_content = fs::read_to_string("src/extract/tmp/opt-feats.csv").expect("Failed to read csv file");
//                 let json_content = fs::read_to_string("src/extract/tmp/opt_1.json").expect("Failed to read json file");

//                 let new_abc_cost = match send_circuit_files_to_server(&el_content, &csv_content, &json_content).await {
//                     Ok(d) => d,
//                     Err(e) => {
//                         eprintln!("Error sending circuit files to server: {}", e);
//                         0.0
//                     }
//                 };

//                 let elapsed_time = Instant::now() - start_time;
//                 total_calculate_time_1 += elapsed_time;

//                 let cost_change = new_abc_cost - current_abc_cost;
//                 if verbose {
//                     panel.set_message(format!(
//                         "Temp: {:.2}\nCurrent ABC cost: {:.6}\nNew ABC cost: {:.6}\nChange: {:.6}\n\nuphill: {:.6}",
//                         temperature, current_abc_cost, new_abc_cost, cost_change,uphill
//                     ));
//                 }
//                 if cost_change <= 0.0 || rng.clone().lock().unwrap().gen::<f64>() < (-cost_change / temperature).exp() {
//                     if cost_change > 0.0 {
//                         uphill += 1;
//                     }
//                     current_result = new_result;
//                     current_abc_cost = new_abc_cost;

//                     if current_abc_cost < best_abc_cost {
//                         best_result = current_result.clone();
//                         best_abc_cost = current_abc_cost;
//                         panel.println(format!(
//                             "New best solution found! Cost: {:.6}", 
//                             best_abc_cost));
//                     }
//                 }

//                 iteration_count += 1.0;
//                 pb.set_position(iteration_count as u64);

//                 if iteration_count <= k {
//                     temperature = initial_temp * (cost_change / initial_temp) / (iteration_count * c);
//                 } else {
//                     temperature = initial_temp * (cost_change / initial_temp) / iteration_count;
//                 }
//             }

//             pb.finish_with_message("Simulated Annealing Complete");
//             panel.finish_with_message(format!(
//                 "SA-final ABC cost: {:.6}\nBase solution ABC cost: {:.6}",
//                 best_abc_cost, base_abc_cost
//             ));

//             // Return the best result or base result based on comparison
//             if best_abc_cost <= base_abc_cost {
//                 println!("SA-final solution is better. Returning SA-final.");
//                 println!("generate random soultion: {:?}", total_calculate_time_0);
//                 println!("Total time spent in calculate_abc_cost_or_dump(): {:?}", total_calculate_time_1);
//                 // save the best result to file
//                 _ = calculate_abc_cost_or_dump(
//                     &best_result,
//                     &saturated_graph_json,
//                     &prefix_mapping_path,
//                     true,
//                 );
//                 best_result
//             } else {
//                 println!("Base solution is better. Returning base solution.");
//             println!("generate random soultion: {:?}", total_calculate_time_0);
//             println!("Total time spent in calculate_abc_cost_or_dump(): {:?}", total_calculate_time_1);
            
//             // save the base result to file
//             _ = calculate_abc_cost_or_dump(
//                 &base_result,
//                 &saturated_graph_json,
//                 &prefix_mapping_path,
//                 true,
//             );
//             base_result
//             }
//         }
//     }
// }

// //=========================ppppppppppp================================

// impl Extractor for FasterBottomUpFastSimulatedAnnealingExtractor {
//     fn extract(
//         &self,
//         egraph: &EGraph,
//         _roots: &[ClassId],
//         cost_function: &str,
//         random_prob: f64,
//     ) -> ExtractionResult {
//         let mut rng = thread_rng();
//         let saturated_graph_path = "input/rewritten_egraph_with_weight_cost_serd.json";
//         let prefix_mapping_path = "../e-rewriter/circuit0_opt.eqn";

//         let saturated_graph_json = fs::read_to_string(saturated_graph_path).unwrap_or_else(|e| {
//             eprintln!("Failed to read saturated graph file: {}", e);
//             String::new()
//         });

//         // Generate base solution using faster bottom-up
//         // let mut base_result =
//         //     generate_initial_solution(egraph, InitialSolutionType::Base, cost_function, &mut rng);
//         // update_json_buffers_in_result(&mut base_result, egraph);
//         // let base_abc_cost = calculate_abc_cost_or_dump(
//         //     &base_result,
//         //     &saturated_graph_json,
//         //     &prefix_mapping_path,
//         //     false,
//         // );

//         // Generate random initial solution for SA
//         let start_time= Instant::now();
//         let mut current_result =
//             generate_initial_solution(egraph, InitialSolutionType::Base, cost_function, &mut rng); // TODO: adjust here
//         update_json_buffers_in_result(&mut current_result, egraph);
//         let mut current_abc_cost = calculate_abc_cost_or_dump(
//             &current_result,
//             &saturated_graph_json,
//             &prefix_mapping_path,
//             false,
//         );
//         let elapsed_time = Instant::now() - start_time;
//         println!("generate random soultion: {:?}", elapsed_time);
//         let base_result = current_result.clone();
//         let base_abc_cost = current_abc_cost;
//         let initial_randomprob_sa =0.95_f64;
//         let initial_temp: f64 = -1.0 * current_abc_cost / initial_randomprob_sa.ln();
//         println!("initial_temp: {:.6}", initial_temp);
//         let c= 1000.0; //parameter for 2nd stagetemperature update
//         let k =3.0;//parameter for 2nd stage termination

//         //let cooling_rate: f64 = 0.7;
//         let mut temperature: f64 = initial_temp;
//         let sample_size = (egraph.classes().len() as f64 * 0.3).max(1.0) as usize;
//         let verbose = true;
    



//         let mut best_result = current_result.clone();
//         let mut best_abc_cost = current_abc_cost;
//         let mut initial_cost = current_abc_cost;
//         let total_iterations = 4;
//         let m = MultiProgress::new();
//         let pb = m.add(ProgressBar::new(total_iterations));
//         pb.set_style(
//             ProgressStyle::default_bar()
//                 .template(
//                     "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
//                 )
//                 .unwrap()
//                 .progress_chars("#>-"),
//         );

//         let panel = m.add(ProgressBar::new(1));
//         panel.set_style(
//             ProgressStyle::default_spinner()
//                 .template("{spinner:.green} {wide_msg}")
//                 .unwrap(),
//         );

//         println!("========== Starting Simulated Annealing ==========");
//         panel.set_message(format!(
//             "Base solution ABC cost: {:.6}. Initial solution's ABC cost: {:.6}",
//             base_abc_cost, current_abc_cost
//         ));

//         let mut iteration_count = 1.0;
//         let mut uphill =  0;
        
//        let mut total_calculate_time_1: std::time::Duration = std::time::Duration::new(0, 0);
//        let mut total_calculate_time_0: std::time::Duration = std::time::Duration::new(0, 0);
//         //uphill<= total_iterations/2)|| 
//         while (iteration_count as u64<= total_iterations && uphill<= total_iterations/2)  {
//                 let start_time = Instant::now();
//                 let mut new_result = 
//                 generate_neighbor_solution(&current_result, egraph,NeighborSolutionType::RandomExtraction,
//                     sample_size,&mut rng,cost_function,0.9,temperature,initial_temp,);
//                 let elapsed_time = Instant::now() - start_time;
//             // NeighborSolutionType:: SmartIntermediatePropagation,
//                 total_calculate_time_0 += elapsed_time;
//                 let start_time = Instant::now();
//                 update_json_buffers_in_result(&mut new_result, egraph);
                
//                 let new_abc_cost = calculate_abc_cost_or_dump(&new_result,&saturated_graph_json,
//                     &prefix_mapping_path,false,);
//                 let elapsed_time = Instant::now() - start_time;
//                 total_calculate_time_1 += elapsed_time;

//                 let cost_change = new_abc_cost - current_abc_cost;
            

//                 if verbose {
//                     panel.set_message(format!(
//                         "Temp: {:.2}\nCurrent ABC cost: {:.6}\nNew ABC cost: {:.6}\nChange: {:.6}\n\nuphill: {:.6}",
//                         temperature, current_abc_cost, new_abc_cost, cost_change,uphill
//                     ));
//                 }

//                 if cost_change <= 0.0 || rng.gen::<f64>() < (-cost_change / temperature).exp() {
//                     if cost_change > 0.0{
// 					uphill+=1;
//                     }
//                     current_result = new_result;
//                     current_abc_cost = new_abc_cost;

//                     if current_abc_cost < best_abc_cost {
//                         best_result = current_result.clone();
//                         best_abc_cost = current_abc_cost;
//                         panel.println(format!(
//                             "New best solution found! Cost: {:.6}",
//                             best_abc_cost
//                         ));
//                     }
//                 }
//                 iteration_count += 1.0;
//                 pb.set_position(iteration_count as u64);

        

//                 if iteration_count <= k {
//                     temperature = initial_temp * (cost_change / initial_temp) / (iteration_count * c);
//                 } else {
//                     temperature = initial_temp * (cost_change / initial_temp) / iteration_count;
//                 }
//             }

//         pb.finish_with_message("Simulated Annealing Complete");
//         panel.finish_with_message(format!(
//             "SA-final ABC cost: {:.6}\nBase solution ABC cost: {:.6}",
//             best_abc_cost, base_abc_cost
//         ));
        
//         // Compare SA-final with base solution
//         if best_abc_cost <= base_abc_cost {
//             println!("SA-final solution is better. Returning SA-final.");
//             println!("generate random soultion: {:?}", total_calculate_time_0);
//             println!("Total time spent in calculate_abc_cost_or_dump(): {:?}", total_calculate_time_1);
//             // save the best result to file
//             _ = calculate_abc_cost_or_dump(
//                 &best_result,
//                 &saturated_graph_json,
//                 &prefix_mapping_path,
//                 true,
//             );
//             best_result
//         } else {
//             println!("Base solution is better. Returning base solution.");
//             println!("generate random soultion: {:?}", total_calculate_time_0);
//             println!("Total time spent in calculate_abc_cost_or_dump(): {:?}", total_calculate_time_1);
            
//             // save the base result to file
//             _ = calculate_abc_cost_or_dump(
//                 &base_result,
//                 &saturated_graph_json,
//                 &prefix_mapping_path,
//                 true,
//             );
//             base_result
//         }
//     }
// }

// ========================== Helper Functions For SA-based faster bottom-up ==========================
// Save best result to file
// ========================== Helper Functions For SA-based faster bottom-up ==========================

// fn save_best_result_to_file(
//     pass;
// }

// ========================== Helper Functions For SA-based faster bottom-up ==========================
// Generate initial or base solution for SA
// ========================== Helper Functions For SA-based faster bottom-up ==========================

// Generate random initial solution for SA
// fn generate_random_solution(egraph: &EGraph) -> ExtractionResult {
//     let mut rng = thread_rng();
//     let mut result = ExtractionResult::default();

//     for class in egraph.classes().values() {
//         if let Some(random_node) = class.nodes.choose(&mut rng) {
//             result.choose(class.id.clone(), random_node.clone());
//         }
//     }

//     result
// }

// // Generate base solution for Simulated Annealing

// fn generate_base_solution(egraph: &EGraph, cost_function: &str) -> ExtractionResult {
//     let mut parents = IndexMap::<ClassId, Vec<NodeId>>::with_capacity(egraph.classes().len());
//     let n2c = |nid: &NodeId| egraph.nid_to_cid(nid);
//     let mut analysis_pending = UniqueQueue::default();

//     for class in egraph.classes().values() {
//         parents.insert(class.id.clone(), Vec::new());
//     }

//     for class in egraph.classes().values() {
//         for node in &class.nodes {
//             for c in &egraph[node].children {
//                 parents[n2c(c)].push(node.clone());
//             }
//             if egraph[node].is_leaf() {
//                 analysis_pending.insert(node.clone());
//             }
//         }
//     }

//     let mut result = ExtractionResult::default();
//     let mut costs = FxHashMap::<ClassId, Cost>::with_capacity_and_hasher(
//         egraph.classes().len(),
//         Default::default(),
//     );

//     while let Some(node_id) = analysis_pending.pop() {
//         let class_id = n2c(&node_id);
//         let node = &egraph[&node_id];
//         let prev_cost = costs.get(class_id).unwrap_or(&INFINITY);
//         let cost = match cost_function {
//             "node_sum_cost" => result.node_sum_cost(egraph, node, &costs),
//             "node_depth_cost" => result.node_depth_cost(egraph, node, &costs),
//             _ => panic!("Unknown cost function: {}", cost_function),
//         };
//         if cost < *prev_cost {
//             result.choose(class_id.clone(), node_id.clone());
//             costs.insert(class_id.clone(), cost);
//             analysis_pending.extend(parents[class_id].iter().cloned());
//         }
//     }

//     result
// }

// fn generate_greedy_depth_solution(egraph: &EGraph, cost_function: &str) -> ExtractionResult {
//     let mut result = ExtractionResult::default();
//     let mut costs = FxHashMap::<ClassId, Cost>::default();
//     let n2c = |nid: &NodeId| egraph.nid_to_cid(nid);

//     // Sort classes by depth (leaves first)
//     let mut classes: Vec<_> = egraph.classes().values().collect();
//     classes.sort_by_key(|class| {
//         egraph.nodes.get(&class.nodes[0]).map_or(0, |node| node.children.len())
//     });

//     for class in classes {
//         let mut best_cost = INFINITY;
//         let mut best_node = None;

//         for node_id in &class.nodes {
//             let node = &egraph[node_id];
//             let cost = match cost_function {
//                 "node_sum_cost" => result.node_sum_cost(egraph, node, &costs),
//                 "node_depth_cost" => result.node_depth_cost(egraph, node, &costs),
//                 _ => panic!("Unknown cost function: {}", cost_function),
//             };

//             if cost < best_cost {
//                 best_cost = cost;
//                 best_node = Some(node_id.clone());
//             }
//         }

//         if let Some(node_id) = best_node {
//             result.choose(class.id.clone(), node_id);
//             costs.insert(class.id.clone(), best_cost);
//         }
//     }

//     result
// }

// fn generate_greedy_cost_solution(egraph: &EGraph, cost_function: &str) -> ExtractionResult {
//     let mut result = ExtractionResult::default();
//     let mut costs = FxHashMap::<ClassId, Cost>::default();
//     let n2c = |nid: &NodeId| egraph.nid_to_cid(nid);

//     // Sort classes by minimum node cost
//     let mut classes: Vec<_> = egraph.classes().values().collect();
//     classes.sort_by_key(|class| {
//         class.nodes.iter()
//             .map(|node_id| egraph[node_id].cost)
//             .min()
//             .unwrap_or(INFINITY)
//     });

//     for class in classes {
//         let mut best_cost = INFINITY;
//         let mut best_node = None;

//         for node_id in &class.nodes {
//             let node = &egraph[node_id];
//             let cost = match cost_function {
//                 "node_sum_cost" => result.node_sum_cost(egraph, node, &costs),
//                 "node_depth_cost" => result.node_depth_cost(egraph, node, &costs),
//                 _ => panic!("Unknown cost function: {}", cost_function),
//             };

//             if cost < best_cost {
//                 best_cost = cost;
//                 best_node = Some(node_id.clone());
//             }
//         }

//         if let Some(node_id) = best_node {
//             result.choose(class.id.clone(), node_id);
//             costs.insert(class.id.clone(), best_cost);
//         }
//     }

//     result
// }

// fn generate_hybrid_random_greedy_solution(egraph: &EGraph, cost_function: &str, rng: &mut impl Rng) -> ExtractionResult {
//     let mut result = ExtractionResult::default();
//     let mut costs = FxHashMap::<ClassId, Cost>::default();
//     let n2c = |nid: &NodeId| egraph.nid_to_cid(nid);

//     for class in egraph.classes().values() {
//         if rng.gen_bool(0.5) {
//             // Random selection
//             if let Some(random_node) = class.nodes.choose(rng) {
//                 result.choose(class.id.clone(), random_node.clone());
//                 let cost = match cost_function {
//                     "node_sum_cost" => result.node_sum_cost(egraph, &egraph[random_node], &costs),
//                     "node_depth_cost" => result.node_depth_cost(egraph, &egraph[random_node], &costs),
//                     _ => panic!("Unknown cost function: {}", cost_function),
//                 };
//                 costs.insert(class.id.clone(), cost);
//             }
//         } else {
//             // Greedy selection
//             let mut best_cost = INFINITY;
//             let mut best_node = None;

//             for node_id in &class.nodes {
//                 let node = &egraph[node_id];
//                 let cost = match cost_function {
//                     "node_sum_cost" => result.node_sum_cost(egraph, node, &costs),
//                     "node_depth_cost" => result.node_depth_cost(egraph, node, &costs),
//                     _ => panic!("Unknown cost function: {}", cost_function),
//                 };

//                 if cost < best_cost {
//                     best_cost = cost;
//                     best_node = Some(node_id.clone());
//                 }
//             }

//             if let Some(node_id) = best_node {
//                 result.choose(class.id.clone(), node_id);
//                 costs.insert(class.id.clone(), best_cost);
//             }
//         }
//     }

//     result
// }

// ========================== Helper Functions For SA-based faster bottom-up ==========================
// Generate neighbor solution relate to domain structure
// ========================== Helper Functions For SA-based faster bottom-up ==========================

// easy one

// fn generate_neighbor_solution_naive(
//     current: &ExtractionResult,
//     egraph: &EGraph,
//     sample_size: usize,
//     rng: &mut impl Rng,
// ) -> ExtractionResult {
//     let mut new_result = current.clone();
//     let sampled_classes: Vec<_> = egraph.classes().values().choose_multiple(rng, sample_size);

//     for class in sampled_classes {
//         if let Some(neighbor_node) = class.nodes.choose(rng) {
//             new_result.choose(class.id.clone(), neighbor_node.clone());
//         }
//     }

//     new_result
// }

// //with cycle check

// fn generate_neighbor_solution_with_cycle_check(
//     current: &ExtractionResult,
//     egraph: &EGraph,
//     sample_size: usize,
//     rng: &mut impl Rng,
// ) -> ExtractionResult {
//     let mut new_result = current.clone();
//     let sampled_classes: Vec<_> = egraph.classes().values().choose_multiple(rng, sample_size);

//     let mut proposed_changes = Vec::new();

//     for class in sampled_classes {
//         if let Some(neighbor_node) = class.nodes.choose(rng) {
//             proposed_changes.push((class.id.clone(), neighbor_node.clone()));
//         }
//     }

//     // Apply changes sequentially and check for cycles
//     let mut temp_result = new_result.clone();

//     for (class_id, node_id) in proposed_changes {
//         temp_result.choose(class_id.clone(), node_id.clone());
//         let cycles = temp_result.find_cycles(egraph, &egraph.root_eclasses);

//         if cycles.is_empty() {
//             new_result.choose(class_id, node_id);
//         } else {
//             // Revert the change if it introduces a cycle
//             temp_result = new_result.clone();
//         }
//     }

//     new_result
// }

// // idea from random extraction

// fn generate_neighbor_solution_random_extraction(
//     current: &ExtractionResult,
//     egraph: &EGraph,
//     cost_function: &str,
//     random_prob: f64,
//     rng: &mut impl Rng,
// ) -> ExtractionResult {
//     let start_time = std::time::Instant::now();
//     // map each ClassId to its parent nodeID
//     let mut parents = IndexMap::<ClassId, Vec<NodeId>>::with_capacity(egraph.classes().len());
//         let n2c = |nid: &NodeId| egraph.nid_to_cid(nid);
//         let mut analysis_pending = UniqueQueue::default();
//         for class in egraph.classes().values() {
//             parents.insert(class.id.clone(), Vec::new());
//         }

//         for class in egraph.classes().values() {
//             for node in &class.nodes {
//                 for c in &egraph[node].children {
//                     // compute parents of this enode
//                     parents[n2c(c)].push(node.clone());
//                     //println!("Node: {:?}", node);
//                 }

//                 // start the analysis from leaves
//                 if egraph[node].is_leaf() {
//                     analysis_pending.insert(node.clone());
//                 }
//             }
//         }

//         //let mut result = current.clone();
//         let mut result = ExtractionResult::default();
//         let mut costs = FxHashMap::<ClassId, Cost>::with_capacity_and_hasher(
//             egraph.classes().len(),
//             Default::default(),
//         );
//     let mut i=0;
//     // propagate changes from leaves to roots
//     while let Some(node_id) = analysis_pending.pop() {
            
//         let class_id = n2c(&node_id);
//         let node = &egraph[&node_id];
//         let prev_cost = costs.get(class_id).unwrap_or(&INFINITY);
//         let cost = match cost_function {
//             "node_sum_cost" => result.node_sum_cost(egraph, node, &costs),
//             "node_depth_cost" => result.node_depth_cost(egraph, node, &costs),
//             _ => panic!("Unknown cost function: {}", cost_function),
//         };
//         let mut rng = rand::thread_rng();
//         let random_value: f64 = rng.gen();

//         i+=1;
//         if  prev_cost ==&INFINITY &&(cost < *prev_cost)  {
            
//             result.choose(class_id.clone(), node_id.clone());
//             costs.insert(class_id.clone(), cost);
//             analysis_pending.extend(parents[class_id].iter().cloned());
//         }else if random_value>=random_prob &&(cost < *prev_cost) {
//             result.choose(class_id.clone(), node_id.clone());
//             costs.insert(class_id.clone(), cost);
//             analysis_pending.extend(parents[class_id].iter().cloned());
            
//         }
//         // if  prev_cost ==&INFINITY  {

//         //     result.choose(class_id.clone(), node_id.clone());
//         //     costs.insert(class_id.clone(), cost);
//         //     analysis_pending.extend(parents[class_id].iter().cloned());
//         // }else if (cost < *prev_cost) {
//         //     result.choose(class_id.clone(), node_id.clone());
//         //     costs.insert(class_id.clone(), cost);

//         //     analysis_pending.extend(parents[class_id].iter().cloned());
            
//         // }
//         // else if(cost == *prev_cost)&&random_value>=random_prob {
//         //         result.choose(class_id.clone(), node_id.clone());
//         //         costs.insert(class_id.clone(), cost);
    
//         //         analysis_pending.extend(parents[class_id].iter().cloned());
//         // }

        
//     }
//     let us = start_time.elapsed().as_micros();
//     print!("iteration:{}", i);
//     println!(
//         "Time consumption of faster bottom-up extractor: {} seconds",
//         us as f64 / 1000000.0
//     );
   
//     result
// }


// // add randomization when previous cost is infinity

// fn generate_neighbor_solution_skip_inf(
//     current: &ExtractionResult,
//     egraph: &EGraph,
//     cost_function: &str,
//     random_prob: f64,
//     rng: &mut impl Rng,
// ) -> ExtractionResult {
//     // map each ClassId to its parent nodeID
//     let mut parents = IndexMap::<ClassId, Vec<NodeId>>::with_capacity(egraph.classes().len());
//     let n2c = |nid: &NodeId| egraph.nid_to_cid(nid);
//     let mut analysis_pending = UniqueQueue::default(); // a queue of nodes to be analyzed

//     for class in egraph.classes().values() {
//         parents.insert(class.id.clone(), Vec::new());
//     }

//     // build the parent-child relationships
//     for class in egraph.classes().values() {
//         for node in &class.nodes {
//             for c in &egraph[node].children {
//                 parents[n2c(c)].push(node.clone());
//             }
//             if egraph[node].is_leaf() {
//                 analysis_pending.insert(node.clone()); // add leaf nodes to the analysis queue
//             }
//         }
//     }

//     // initialize the result and costs
//     let mut result = current.clone();
//     let mut costs = FxHashMap::<ClassId, Cost>::with_capacity_and_hasher(
//         egraph.classes().len(),
//         Default::default(),
//     );

//     let mut selected_any = false;
//     let mut extended_nodes = Vec::new();

//     // propagate changes from leaves to roots
//     while let Some(node_id) = analysis_pending.pop() {
//         let class_id = n2c(&node_id);
//         let node = &egraph[&node_id];
//         let prev_cost = costs.get(class_id).unwrap_or(&INFINITY);

//         // generate the cost of the current e-node
//         let cost = match cost_function {
//             "node_sum_cost" => result.node_sum_cost(egraph, node, &costs),
//             "node_depth_cost" => result.node_depth_cost(egraph, node, &costs),
//             _ => panic!("Unknown cost function: {}", cost_function),
//         };
//         let random_value: f64 = rng.gen();

//         if random_value >= random_prob && cost <= prev_cost * 1.0 {
//             result.choose(class_id.clone(), node_id.clone());
//             costs.insert(class_id.clone(), cost);
//             selected_any = true;
//             extended_nodes.extend(parents[class_id].iter().cloned());
//         }
//     }

//     // If no node was selected, choose one randomly from the extended nodes
//     if !selected_any && !extended_nodes.is_empty() {
//         let random_node = extended_nodes.choose(rng).unwrap();
//         let class_id = n2c(random_node);
//         result.choose(class_id.clone(), random_node.clone());
//     }

//     // Add all extended nodes to the analysis pending queue
//     analysis_pending.extend(extended_nodes.iter().cloned());

//     result
// }

// // permutate the pending nodes to generate a neighbor solution

// fn generate_neighbor_solution_permutate_pending_nodes(
//     current: &ExtractionResult,
//     egraph: &EGraph,
//     cost_function: &str,
//     random_prob: f64,
//     rng: &mut impl Rng,
// ) -> ExtractionResult {
//     let mut result = current.clone();
//     let mut costs = FxHashMap::<ClassId, Cost>::with_capacity_and_hasher(
//         egraph.classes().len(),
//         Default::default(),
//     );

//     // Build parent relationships
//     let mut parents = IndexMap::<ClassId, Vec<NodeId>>::with_capacity(egraph.classes().len());
//     let n2c = |nid: &NodeId| egraph.nid_to_cid(nid);

//     for class in egraph.classes().values() {
//         parents.insert(class.id.clone(), Vec::new());
//         for node in &class.nodes {
//             for c in &egraph[node].children {
//                 parents
//                     .entry(n2c(c).clone())
//                     .or_default()
//                     .push(node.clone());
//             }
//         }
//     }

//     // Initialize the analysis queue with leaf nodes
//     let mut analysis_pending = UniqueQueue::default();
//     for class in egraph.classes().values() {
//         for node in &class.nodes {
//             if egraph[node].is_leaf() {
//                 analysis_pending.insert(node.clone());
//             }
//         }
//     }

//     while let Some(node_id) = analysis_pending.pop() {
//         let class_id = n2c(&node_id);
//         let node = &egraph[&node_id];
//         let prev_cost = costs.get(class_id).unwrap_or(&INFINITY);

//         // Calculate the cost of the current node
//         let cost = match cost_function {
//             "node_sum_cost" => result.node_sum_cost(egraph, node, &costs),
//             "node_depth_cost" => result.node_depth_cost(egraph, node, &costs),
//             _ => panic!("Unknown cost function: {}", cost_function),
//         };

//         let random_value: f64 = rng.gen();

//         // Decide whether to update the current choice
//         if prev_cost == &INFINITY || (random_value >= random_prob && cost < *prev_cost) {
//             result.choose(class_id.clone(), node_id.clone());
//             costs.insert(class_id.clone(), cost);

//             // Permutate and add parents to the analysis queue
//             if let Some(parent_nodes) = parents.get(class_id) {
//                 let mut shuffled_parents = parent_nodes.clone();
//                 shuffled_parents.shuffle(rng);
//                 for parent_id in shuffled_parents {
//                     analysis_pending.insert(parent_id);
//                 }
//             }
//         } else {
//             // If we don't update, still add parents to ensure full exploration
//             if let Some(parent_nodes) = parents.get(class_id) {
//                 for parent_id in parent_nodes {
//                     analysis_pending.insert(parent_id.clone());
//                 }
//             }
//         }
//     }

//     result
// }

// with intermediate node changes propagations





// ========================== Helper Functions For SA-based faster bottom-up ==========================
// Calculate ABC cost for a given solution
// ========================== Helper Functions For SA-based faster bottom-up ==========================



// ========================== Helper Functions For SA-based faster bottom-up ==========================
// Update JSON buffers for a given solution
// ========================== Helper Functions For SA-based faster bottom-up ==========================

fn update_json_buffers_in_result(result: &mut ExtractionResult, egraph: &EGraph) {
    let tree_cost_json = to_string_pretty(&result).unwrap();
    let (dag_cost, dag_cost_extraction_result) =
        result.calculate_dag_cost_with_extraction_result(&egraph, &egraph.root_eclasses);
    let dag_cost_json = to_string_pretty(&dag_cost_extraction_result).unwrap();

    result.tree_cost_json = Some(tree_cost_json);
    result.dag_cost_json = Some(dag_cost_json);
}

/** A data structure to maintain a queue of unique elements.

Notably, insert/pop operations have O(1) expected amortized runtime complexity.

Thanks @Bastacyclop for the implementation!
*/
#[derive(Clone)]
#[cfg_attr(feature = "serde-1", derive(Serialize, Deserialize))]
pub(crate) struct UniqueQueue<T>
where
    T: Eq + std::hash::Hash + Clone,
{
    set: FxHashSet<T>, // hashbrown::
    queue: std::collections::VecDeque<T>,
}

impl<T> Default for UniqueQueue<T>
where
    T: Eq + std::hash::Hash + Clone,
{
    fn default() -> Self {
        UniqueQueue {
            set: Default::default(),
            queue: std::collections::VecDeque::new(),
        }
    }
}

impl<T> UniqueQueue<T>
where
    T: Eq + std::hash::Hash + Clone,
{
    pub fn insert(&mut self, t: T) {
        if self.set.insert(t.clone()) {
            self.queue.push_back(t);
        }
    }

    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        for t in iter.into_iter() {
            self.insert(t);
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        let res = self.queue.pop_front();
        res.as_ref().map(|t| self.set.remove(t));
        res
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        let r = self.queue.is_empty();
        debug_assert_eq!(r, self.set.is_empty());
        r
    }
}

// ========================================== Helper Functions For SA-based faster bottom-up ==========================================
// Send circuit files to server
// ========================================== Helper Functions For SA-based faster bottom-up ==========================================


// ========================================== Helper Functions For SA-based faster bottom-up ==========================================
// Call ABC
// ========================================== Helper Functions For SA-based faster bottom-up ==========================================





    //println!("Performing technology mapping...");
    //abc.execute_command(&format!("map"));
    //println!("Performing post-processing...(topo; gate sizing)");
    //abc.execute_command(&format!("topo"));
    //abc.execute_command(&format!("upsize"));
    //abc.execute_command(&format!("dnsize"));

    //println!("Executing stime command...");
    //let stime_output = abc.execute_command_with_output(&format!("stime -d"));

    // if let Some(delay) = parse_delay(&stime_output) {
    //     let delay_ns = delay / 1000.0;
    //     //println!("Delay in nanoseconds: {} ns", delay_ns);
    //     Ok(delay)
    // } else {
    //     Err("Failed to parse delay value".into())
    // }














