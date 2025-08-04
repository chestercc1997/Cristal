mod extract;
pub use extract::*;
use egraph_serialize::*;
use crate::faster_bottom_up::FasterBottomUpExtractorRandom;
use std::env::current_dir;
use anyhow::Context;
use im_rc::iter;
use indexmap::IndexMap;
use ordered_float::NotNan;
use rayon::ThreadPool;
use rayon::ThreadPoolBuilder;
use serde_json::to_string_pretty;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
pub type Cost = NotNan<f64>;

// Define a constant for infinity cost
pub const INFINITY: Cost = unsafe { NotNan::new_unchecked(std::f64::INFINITY) };

// pub mod vectorservice {
//     tonic::include_proto!("vectorservice");
// }

// Function to get the fast extractors
// Returns: An `IndexMap` mapping extractor names to their corresponding `Extractor` implementations
fn get_fast_extractors() -> IndexMap<&'static str, Box<dyn Extractor>> {
    [
        ("bottom-up", extract::bottom_up::BottomUpExtractor.boxed()),

        (
            "faster-bottom-up",
            extract::faster_bottom_up::FasterBottomUpExtractor.boxed(),
        ),
        (
            "greedy-dag",
            extract::greedy_dag::GreedyDagExtractor.boxed(),
        ),
        (
            "faster-greedy-dag",
            extract::faster_greedy_dag::FasterGreedyDagExtractor.boxed(),
        ),
        (
            "global-greedy-dag",
            extract::global_greedy_dag::GlobalGreedyDagExtractor.boxed(),
        ),
        (
            "random-based-faster-bottom-up",
            extract::faster_bottom_up::FasterBottomUpExtractorRandom.boxed(),
        ),
    ]
    .into_iter()
    .enumerate()
    //.filter(|(index, _)| *index == 1)
    .map(|(_, item)| item)
    .collect()
}

// Function to get the extractor name from the command-line arguments
// Input: A mutable reference to the `pico_args::Arguments` instance
// Returns: The extractor name as a `String`, defaulting to "bottom-up" if not provided
fn get_extractor_name(args: &mut pico_args::Arguments) -> String {
    args.opt_value_from_str("--extractor")
        .unwrap()
        .unwrap_or_else(|| "faster-bottom-up".into())
}

// Function to print the extractor names
// Input: A reference to the `IndexMap` of extractors
fn print_extractor_names(extractors: &IndexMap<&str, Box<dyn Extractor>>) {
    for name in extractors.keys() {
        println!("{}", name);
    }
}

// Function to get randomized extractors
fn get_random_sampling_settings(args: &mut pico_args::Arguments) -> (u32, f64) {
    let num_samples = args.opt_value_from_str("--num-samples")
        .unwrap()
        .unwrap_or_else(|| 30);
    let random_prob = args.opt_value_from_str("--random-prob")
        .unwrap()
        .unwrap_or_else(|| 0.1);
    (num_samples, random_prob)
}

// Function to get the cost function from the command-line arguments
// Input: A mutable reference to the `pico_args::Arguments` instance
// Returns: The cost function as a `String`, defaulting to "node_depth_cost" if not provided
fn get_cost_function(args: &mut pico_args::Arguments) -> String {
    args.opt_value_from_str("--cost-function")
        .unwrap()
        .unwrap_or_else(|| "node_depth_cost".into())
}

// Function to get the output filename from the command-line arguments
// Input: A mutable reference to the `pico_args::Arguments` instance
// Returns: The output filename as a `PathBuf`, defaulting to "out.json" if not provided
fn get_output_filename(args: &mut pico_args::Arguments) -> PathBuf {
    args.opt_value_from_str("--out")
        .unwrap()
        .unwrap_or_else(|| "out.json".into())
}

// Function to get the input filename from the command-line arguments
// Input: A mutable reference to the `pico_args::Arguments` instance
// Returns: The input filename as a `String`
fn get_input_filename(args: &mut pico_args::Arguments) -> String {
    args.free_from_str().unwrap()
}
// 在main函数中添加：
fn get_sub_dir(args: &mut pico_args::Arguments) -> String {
    args.opt_value_from_str("--sub-dir")
        .unwrap()
        .unwrap_or_else(|| "1".into())
}

// 在main函数中随机分支部分修改：
// Function to modify a filename by replacing a prefix
// Input:
//   - `filename`: The original filename
//   - `old_prefix`: The prefix to replace
//   - `new_prefix`: The new prefix to use
// Returns: The modified filename as a `String`
fn modify_filename(filename: &str, old_prefix: &str, new_prefix: &str) -> String {
    filename.replacen(old_prefix, new_prefix, 1)
}

// Function to parse an e-graph from a JSON file
// Input: The filename of the JSON file
// Returns: The parsed `EGraph` instance
fn parse_egraph(filename: &str) -> EGraph {
    EGraph::from_json_file(filename)
        .with_context(|| format!("Failed to parse {filename}"))
        .unwrap()
}

// Function to get the extractor based on the extractor name
// Input:
//   - `extractors`: A reference to the `IndexMap` of extractors
//   - `extractor_name`: The name of the extractor to retrieve
// Returns: A reference to the `Box<dyn Extractor>` corresponding to the extractor name
fn get_extractor<'a>(
    extractors: &'a IndexMap<&str, Box<dyn Extractor>>,
    extractor_name: &str,
) -> &'a Box<dyn Extractor> {
    // print all extractors
    // println!("Available extractors:");
    // for name in extractors.keys() {
    //     println!("{}", name);
    // }
    //println!("Your chosen extractor: {}", extractor_name);
    extractors
        .get(extractor_name)
        .with_context(|| format!("Unknown extractor: {extractor_name}"))
        .unwrap()
}

// Function to format a modified filename with the extractor name
// Input:
//   - `modified_filename`: The modified filename
//   - `extractor_name`: The name of the extractor
// Returns: The formatted filename as a `String`
fn format_modified_name(modified_filename: &str, extractor_name: &str) -> String {
    format!(
        "{}_{}.json",
        &modified_filename[..modified_filename.len() - 5],
        extractor_name,
    )
}

// Function to extract the result using the selected extractor
// Input:
//   - `extractor`: A reference to the `Box<dyn Extractor>` representing the extractor
//   - `egraph`: A reference to the `EGraph` instance
//   - `root_eclasses`: A reference to the root e-classes
//   - `cost_function`: The cost function to use
// Returns: The `ExtractionResult` obtained from the extraction process
fn extract_result(
    extractor: &Box<dyn Extractor>,
    egraph: &EGraph,
    root_eclasses: &[ClassId],
    cost_function: &str,
) -> ExtractionResult {
    extractor.extract(egraph, root_eclasses, cost_function, 0.0) // 0.0 here prohibits randomness
}

// Function to print the DAG cost
// Input: The DAG cost as a `Cost` value
fn print_dag_cost(dag_cost: Cost) {
    print!("-------------------------------------------\n");
    print!("dag cost: {}\n", dag_cost);
    print!("-------------------------------------------\n");
}

// Function to write a JSON result to a file
// Input:
//   - `filename`: The filename to write the JSON result to
//   - `data`: A reference to the data to serialize and write as JSON
fn write_json_result<T: serde::Serialize>(filename: &str, data: &T) {
    let json_result = to_string_pretty(data).unwrap();
    //let _ = fs::create_dir_all("out_json");
    let __ = fs::write(filename, json_result);
}

// Function to log the result
// Input:
//   - `filename`: The filename associated with the result
//   - `extractor_name`: The name of the extractor used
//   - `dag_cost`: The DAG cost
//   - `us`: The elapsed time in microseconds
fn log_result(filename: &str, extractor_name: &str, dag_cost: Cost, us: u128) {
    log::info!("{filename:40}\t{extractor_name:10}\t{dag_cost:5}\t{us:5}");
}

// Function to write the result to the output file
// Input:
//   - `out_file`: A mutable reference to the output file
//   - `filename`: The filename associated with the result
//   - `modified_name1`: The modified filename
//   - `extractor_name`: The name of the extractor used
//   - `dag_cost`: The DAG cost
//   - `us`: The elapsed time in microseconds
fn write_output_file(
    out_file: &mut File,
    filename: &str,
    modified_name1: &str,
    extractor_name: &str,
    dag_cost: Cost,
    us: u128,
) {
    writeln!(
        out_file,
        r#"{{ 
    "name": "{filename}",
    "md_name": "{modified_name1}",
    "extractor": "{extractor_name}", 
    "dag": {dag_cost}, 
    "micros": {us}
}}"#
    )
    .unwrap();
}

fn get_iteration(args: &mut pico_args::Arguments) -> u32 {
    args.opt_value_from_str("--iteration")
        .unwrap()
        .unwrap_or_else(|| 1)
}

fn run_extract_result_parallel(
    extractor: Arc<dyn Extractor + Send + Sync>,
    egraph: Arc<EGraph>,
    roots: Arc<[ClassId]>,
    cost_function: Arc<str>,
    k: f64, // random probability parameter
    num_samples: u32, // number of samples to take
    result_channel: Sender<ExtractionResult>,
) {
    // print the parameters of random sampling
   // println!("num samples: {}, random probability: {}", num_samples, k);
    let num_runs = num_samples;
    let pool = ThreadPoolBuilder::new().num_threads(64).build().unwrap();
    for _ in 0..num_runs {
        let extractor = Arc::clone(&extractor);
        let egraph = Arc::clone(&egraph);
        let roots = Arc::clone(&roots);
        let cost_function = Arc::clone(&cost_function);
        let result_channel = result_channel.clone();
        pool.spawn(move || {
            let result = extractor.extract(&egraph, &roots, &cost_function, k);
            result_channel.send(result).unwrap();
        });
    }
}

// Main function
fn main() {
    // Initialize the logger
    env_logger::init();

    // Get the fast extractors
    let extractors = get_fast_extractors();

    // Parse command-line arguments
    let mut args = pico_args::Arguments::from_env();

    // Get the extractor name from the arguments
    let extractor_name = get_extractor_name(&mut args);
    if extractor_name == "print" {
        // Print the extractor names and exit
        print_extractor_names(&extractors);
        return;
    }

    // Get the cost function from the arguments
    let cost_function = get_cost_function(&mut args);
    // Get the output filename from the arguments
    let out_filename = get_output_filename(&mut args);
    // Get the input filename from the arguments
    let filename = get_input_filename(&mut args);
    let sub_dir = get_sub_dir(&mut args);
    // Modify the filename for JSON output
    let modified_filename_for_tree_cost = modify_filename(&filename, "input/", "out_json/");
    let modified_filename_for_dag_cost = modify_filename(&filename, "input/", "out_dag_json/");

    let (num_samples, random_prob) = get_random_sampling_settings(&mut args);
    // Check for any remaining arguments
    let rest = args.finish();
    if !rest.is_empty() {
        panic!("Unknown arguments: {:?}", rest);
    }
    
    // Create the output file
    let mut out_file = std::fs::File::create(out_filename.clone()).unwrap();

    // Parse the e-graph from the input file
    let egraph = parse_egraph(&filename);

    // visulize the egraph
    // egraph.to_dot_file("egraph_saturated.dot").unwrap();

    // Get the extractor based on the extractor name
    let extractor = get_extractor(&extractors, &extractor_name);

    // Format the modified filename with the extractor name
    let modified_name_for_tree_cost =
        format_modified_name(&modified_filename_for_tree_cost, &extractor_name);
    let modified_name_for_dag_cost =
        format_modified_name(&modified_filename_for_dag_cost, &extractor_name);

    // Record the start time
    let start_time = std::time::Instant::now();

    // if the extractor is not random
    if extractor_name != "random-based-faster-bottom-up"  { // && extractor_name != "sim_ann_based_bottom-up"
        // Extract the result using the selected extractor
        let tree_cost_extraction_result =
            extract_result(extractor, &egraph, &egraph.root_eclasses, &cost_function);

        // Calculate the elapsed time in microseconds
        let us = start_time.elapsed().as_micros();

        // print cycles if any
        // let cycles = tree_cost_extraction_result
        //     .find_cycles(&egraph, &egraph.root_eclasses);
        // println!("Cycles: {:?}", cycles);
        // // Assert that the result has no cycles
        // // assert!(tree_cost_extraction_result
        // //     .find_cycles(&egraph, &egraph.root_eclasses)
        // //     .is_empty());
        // assert!(cycles.is_empty());

        // parse extracted egraph
        // let egraph_extracted = parse_egraph(&to_string_pretty(&tree_cost_extraction_result).unwrap());
        // egraph_extracted.to_dot_file("egraph_extracted.dot").unwrap();

        // save extract egraph as dot
        //egraph.to_dot_file("egraph_extracted.dot").unwrap();

        // save the extracted egraph as dot 
        // let mut egraph_extracted = tree_cost_extraction_result.get_extracted_egraph(&egraph);
        // egraph_extracted.to_dot_file("egraph_extracted.dot").unwrap();

        // Calculate the DAG cost and the DAG cost with extraction result
        let (dag_cost, dag_cost_extraction_result) = tree_cost_extraction_result
            .calculate_dag_cost_with_extraction_result(&egraph, &egraph.root_eclasses);
        // Print the DAG cost
       // print_dag_cost(dag_cost);

        // Record random costs based on the extraction result
        // tree_cost_extraction_result.record_costs_random(
        //     10,
        //     0.5,
        //     &egraph,
        //     &dag_cost_extraction_result,
        // );
        let current_dir = std::env::current_dir()
        .expect("Failed to get current directory");
    
        // 从输入路径中提取文件名
        let input_filename = std::path::Path::new(&filename)
             .file_name()
             .expect("Failed to get input filename")
             .to_str()
             .unwrap();
         
        let base_output_dir = current_dir
             .join("out_dag_json")
             .join(sub_dir);  
         
         // 使用输入文件名构建输出路径
        let dag_cost_file_name = base_output_dir
             .join(input_filename);  // 直接使用输入文件名
        if let Some(parent) = dag_cost_file_name.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|_| panic!("Failed to create directory: {:?}", parent));
        }
    
        // 写入文件
       // println!("Writing JSON result to: {}", dag_cost_file_name.display());
        write_json_result(
            &dag_cost_file_name.to_string_lossy(),  // 转换回字符串
            &dag_cost_extraction_result
        );
    
        // 验证文件
        if dag_cost_file_name.exists() {
         //   println!("Successfully created file: {}", dag_cost_file_name.display());
        } else {
            println!("Failed to create file: {}", dag_cost_file_name.display());
        }
        // Write the JSON result to files
       // write_json_result(&modified_name_for_tree_cost, &tree_cost_extraction_result);

        // Log the result
        //log_result(&filename, &extractor_name, dag_cost, us);
        // Write the result to the output file (log file)
        // write_output_file(
        //     &mut out_file,
        //     &filename,
        //     &modified_name_for_dag_cost,
        //     &extractor_name,
        //     dag_cost,
        //     us,
        // );

        // print time consumption of tree-based extraction as seconds
        println!(
            "Time consumption of tree-based extraction: {} seconds",
            us as f64 / 1000000.0
        );
    } else { // extractor is random-based-faster-bottom-up
        // if the extractor is random
        let extractor: Arc<dyn Extractor + Send + Sync> = Arc::new(FasterBottomUpExtractorRandom);
        let (result_sender, result_receiver) = channel();
        let cost_function: Arc<str> = Arc::from(cost_function);

        // Extract the result using the selected extractor
        //  let tree_cost_extraction_result = extract_result(extractor, &egraph, &egraph.root_eclasses, &cost_function);

        run_extract_result_parallel(
            extractor,
            Arc::new(egraph.clone()),
            Arc::from(egraph.root_eclasses.clone()),
            cost_function,
            //0.1, // random probability parameter
            //30, // number of samples to take
            random_prob,
            num_samples,
            result_sender,
        );
        //let extraction_result = result_receiver.recv().unwrap();
        let mut extraction_results = Vec::new();
        loop {
            match result_receiver.recv() {
                Ok(extraction_result) => {
                    extraction_results.push(extraction_result);
                }
                Err(_) => break,
            }
        }
        
        let current_dir = std::env::current_dir()
        .expect("Failed to get current directory");
    
    // 直接构建目标目录路径
    let base_output_dir = current_dir
        .join("random_out_dag_json")
        .join(sub_dir);  // sub_dir 参数来自外部输入
    
    // 在循环中构建完整文件路径
    for (i, extraction_result) in extraction_results.iter().enumerate() {
        let (dag_cost, dag_cost_extraction_result_depth) = extraction_result
            .calculate_dag_cost_with_extraction_result(&egraph, &egraph.root_eclasses);
    
        // 构建完整文件路径
        let dag_cost_file_name = base_output_dir
            .join(format!("rewritten_egraph_with_weight_cost_serd_{}.json", i));
    
        // 创建目录（如果不存在）
        if let Some(parent) = dag_cost_file_name.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|_| panic!("Failed to create directory: {:?}", parent));
        }
    
        // 写入文件
        println!("Writing JSON result to: {}", dag_cost_file_name.display());
        write_json_result(
            &dag_cost_file_name.to_string_lossy(),  // 转换回字符串
            &dag_cost_extraction_result_depth
        );
    
        // 验证文件
        if dag_cost_file_name.exists() {
            println!("Successfully created file: {}", dag_cost_file_name.display());
        } else {
            println!("Failed to create file: {}", dag_cost_file_name.display());
        }
    }
    }
}
// pub type Cost = NotNan<f64>;

// // Define a constant for infinity cost
// pub const INFINITY: Cost = unsafe { NotNan::new_unchecked(std::f64::INFINITY) };

// pub mod vectorservice {
//     tonic::include_proto!("vectorservice");
// }

// // Function to get the fast extractors
// // Returns: An `IndexMap` mapping extractor names to their corresponding `Extractor` implementations
// fn get_fast_extractors() -> IndexMap<&'static str, Box<dyn Extractor>> {
//     [
//         ("bottom-up", extract::bottom_up::BottomUpExtractor.boxed()),

//         (
//             "faster-bottom-up",
//             extract::faster_bottom_up::FasterBottomUpExtractor.boxed(),
//         ),
//         (
//             "greedy-dag",
//             extract::greedy_dag::GreedyDagExtractor.boxed(),
//         ),
//         (
//             "faster-greedy-dag",
//             extract::faster_greedy_dag::FasterGreedyDagExtractor.boxed(),
//         ),
//         (
//             "global-greedy-dag",
//             extract::global_greedy_dag::GlobalGreedyDagExtractor.boxed(),
//         ),
//         (
//             "random-based-faster-bottom-up",
//             extract::faster_bottom_up::FasterBottomUpExtractorRandom.boxed(),
//         ),
//     ]
//     .into_iter()
//     .enumerate()
//     //.filter(|(index, _)| *index == 1)
//     .map(|(_, item)| item)
//     .collect()
// }

// // Function to get the extractor name from the command-line arguments
// // Input: A mutable reference to the `pico_args::Arguments` instance
// // Returns: The extractor name as a `String`, defaulting to "bottom-up" if not provided
// fn get_extractor_name(args: &mut pico_args::Arguments) -> String {
//     args.opt_value_from_str("--extractor")
//         .unwrap()
//         .unwrap_or_else(|| "faster-bottom-up".into())
// }

// // Function to print the extractor names
// // Input: A reference to the `IndexMap` of extractors
// fn print_extractor_names(extractors: &IndexMap<&str, Box<dyn Extractor>>) {
//     for name in extractors.keys() {
//         println!("{}", name);
//     }
// }

// // Function to get randomized extractors
// fn get_random_sampling_settings(args: &mut pico_args::Arguments) -> (u32, f64) {
//     let num_samples = args.opt_value_from_str("--num-samples")
//         .unwrap()
//         .unwrap_or_else(|| 30);
//     let random_prob = args.opt_value_from_str("--random-prob")
//         .unwrap()
//         .unwrap_or_else(|| 0.1);
//     (num_samples, random_prob)
// }

// // Function to get the cost function from the command-line arguments
// // Input: A mutable reference to the `pico_args::Arguments` instance
// // Returns: The cost function as a `String`, defaulting to "node_depth_cost" if not provided
// fn get_cost_function(args: &mut pico_args::Arguments) -> String {
//     args.opt_value_from_str("--cost-function")
//         .unwrap()
//         .unwrap_or_else(|| "node_depth_cost".into())
// }

// // Function to get the output filename from the command-line arguments
// // Input: A mutable reference to the `pico_args::Arguments` instance
// // Returns: The output filename as a `PathBuf`, defaulting to "out.json" if not provided
// fn get_output_filename(args: &mut pico_args::Arguments) -> PathBuf {
//     args.opt_value_from_str("--out")
//         .unwrap()
//         .unwrap_or_else(|| "out.json".into())
// }

// // Function to get the input filename from the command-line arguments
// // Input: A mutable reference to the `pico_args::Arguments` instance
// // Returns: The input filename as a `String`
// fn get_input_filename(args: &mut pico_args::Arguments) -> String {
//     args.free_from_str().unwrap()
// }

// // Function to modify a filename by replacing a prefix
// // Input:
// //   - `filename`: The original filename
// //   - `old_prefix`: The prefix to replace
// //   - `new_prefix`: The new prefix to use
// // Returns: The modified filename as a `String`
// fn modify_filename(filename: &str, old_prefix: &str, new_prefix: &str) -> String {
//     filename.replacen(old_prefix, new_prefix, 1)
// }

// // Function to parse an e-graph from a JSON file
// // Input: The filename of the JSON file
// // Returns: The parsed `EGraph` instance
// fn parse_egraph(filename: &str) -> EGraph {
//     EGraph::from_json_file(filename)
//         .with_context(|| format!("Failed to parse {filename}"))
//         .unwrap()
// }

// // Function to get the extractor based on the extractor name
// // Input:
// //   - `extractors`: A reference to the `IndexMap` of extractors
// //   - `extractor_name`: The name of the extractor to retrieve
// // Returns: A reference to the `Box<dyn Extractor>` corresponding to the extractor name
// fn get_extractor<'a>(
//     extractors: &'a IndexMap<&str, Box<dyn Extractor>>,
//     extractor_name: &str,
// ) -> &'a Box<dyn Extractor> {
//     // print all extractors
//     println!("Available extractors:");
//     for name in extractors.keys() {
//         println!("{}", name);
//     }
//     println!("Your chosen extractor: {}", extractor_name);
//     extractors
//         .get(extractor_name)
//         .with_context(|| format!("Unknown extractor: {extractor_name}"))
//         .unwrap()
// }

// // Function to format a modified filename with the extractor name
// // Input:
// //   - `modified_filename`: The modified filename
// //   - `extractor_name`: The name of the extractor
// // Returns: The formatted filename as a `String`
// fn format_modified_name(modified_filename: &str, extractor_name: &str) -> String {
//     format!(
//         "{}_{}.json",
//         &modified_filename[..modified_filename.len() - 5],
//         extractor_name,
//     )
// }

// // Function to extract the result using the selected extractor
// // Input:
// //   - `extractor`: A reference to the `Box<dyn Extractor>` representing the extractor
// //   - `egraph`: A reference to the `EGraph` instance
// //   - `root_eclasses`: A reference to the root e-classes
// //   - `cost_function`: The cost function to use
// // Returns: The `ExtractionResult` obtained from the extraction process
// fn extract_result(
//     extractor: &Box<dyn Extractor>,
//     egraph: &EGraph,
//     root_eclasses: &[ClassId],
//     cost_function: &str,
// ) -> ExtractionResult {
//     extractor.extract(egraph, root_eclasses, cost_function, 0.0) // 0.0 here prohibits randomness
// }

// // Function to print the DAG cost
// // Input: The DAG cost as a `Cost` value
// fn print_dag_cost(dag_cost: Cost) {
//     print!("-------------------------------------------\n");
//     print!("dag cost: {}\n", dag_cost);
//     print!("-------------------------------------------\n");
// }

// // Function to write a JSON result to a file
// // Input:
// //   - `filename`: The filename to write the JSON result to
// //   - `data`: A reference to the data to serialize and write as JSON
// fn write_json_result<T: serde::Serialize>(filename: &str, data: &T) {
//     let json_result = to_string_pretty(data).unwrap();
//     //let _ = fs::create_dir_all("out_json");
//     let __ = fs::write(filename, json_result);
// }

// // Function to log the result
// // Input:
// //   - `filename`: The filename associated with the result
// //   - `extractor_name`: The name of the extractor used
// //   - `dag_cost`: The DAG cost
// //   - `us`: The elapsed time in microseconds
// fn log_result(filename: &str, extractor_name: &str, dag_cost: Cost, us: u128) {
//     log::info!("{filename:40}\t{extractor_name:10}\t{dag_cost:5}\t{us:5}");
// }

// // Function to write the result to the output file
// // Input:
// //   - `out_file`: A mutable reference to the output file
// //   - `filename`: The filename associated with the result
// //   - `modified_name1`: The modified filename
// //   - `extractor_name`: The name of the extractor used
// //   - `dag_cost`: The DAG cost
// //   - `us`: The elapsed time in microseconds
// fn write_output_file(
//     out_file: &mut File,
//     filename: &str,
//     modified_name1: &str,
//     extractor_name: &str,
//     dag_cost: Cost,
//     us: u128,
// ) {
//     writeln!(
//         out_file,
//         r#"{{ 
//     "name": "{filename}",
//     "md_name": "{modified_name1}",
//     "extractor": "{extractor_name}", 
//     "dag": {dag_cost}, 
//     "micros": {us}
// }}"#
//     )
//     .unwrap();
// }

// fn get_iteration(args: &mut pico_args::Arguments) -> u32 {
//     args.opt_value_from_str("--iteration")
//         .unwrap()
//         .unwrap_or_else(|| 1)
// }

// fn run_extract_result_parallel(
//     extractor: Arc<dyn Extractor + Send + Sync>,
//     egraph: Arc<EGraph>,
//     roots: Arc<[ClassId]>,
//     cost_function: Arc<str>,
//     k: f64, // random probability parameter
//     num_samples: u32, // number of samples to take
//     result_channel: Sender<ExtractionResult>,
// ) {
//     // print the parameters of random sampling
//     println!("num samples: {}, random probability: {}", num_samples, k);
//     let num_runs = num_samples;
//     let pool = ThreadPoolBuilder::new().num_threads(64).build().unwrap();
//     for _ in 0..num_runs {
//         let extractor = Arc::clone(&extractor);
//         let egraph = Arc::clone(&egraph);
//         let roots = Arc::clone(&roots);
//         let cost_function = Arc::clone(&cost_function);
//         let result_channel = result_channel.clone();
//         pool.spawn(move || {
//             let result = extractor.extract(&egraph, &roots, &cost_function, k);
//             result_channel.send(result).unwrap();
//         });
//     }
// }

// // Main function
// fn get_k_value(args: &mut pico_args::Arguments) -> u32 {
//     args.opt_value_from_str("--k")
//         .unwrap()
//         .unwrap_or_else(|| 1) // 默认值为 1
// }

// // 修改文件路径用于加载子目录
// fn modify_directory_with_k(base_dir: &str, k: u32) -> String {
//     format!("{}/{}", base_dir, k)
// }

// // 主函数
// fn main() {
//     // 初始化日志
//     env_logger::init();

//     // 获取快速提取器
//     let extractors = get_fast_extractors();

//     // 解析命令行参数
//     let mut args = pico_args::Arguments::from_env();

//     // 获取提取器名称参数
//     let extractor_name = get_extractor_name(&mut args);
//     if extractor_name == "print" {
//         // 打印提取器名称并退出
//         print_extractor_names(&extractors);
//         return;
//     }

//     // 获取其他参数
//     let cost_function = get_cost_function(&mut args);
//     let out_filename = get_output_filename(&mut args);
//     let filename = get_input_filename(&mut args);
//     let k = get_k_value(&mut args); // 新增 k 参数

//     // 修改目录路径
//     let base_directory = "random_out_dag_son"; // 基础路径
//     let sub_directory = modify_directory_with_k(base_directory, k);

//     // 将文件路径与子目录结合
//     let modified_filename_for_tree_cost = modify_filename(&filename, "input/", "out_json/");
//     let modified_filename_for_dag_cost = modify_filename(&filename, "input/", &sub_directory);

//     let (num_samples, random_prob) = get_random_sampling_settings(&mut args);
//     let rest = args.finish();
//     if !rest.is_empty() {
//         panic!("Unknown arguments: {:?}", rest);
//     }

//     // 创建输出文件
//     let mut out_file = std::fs::File::create(out_filename.clone()).unwrap();

//     // 解析 e-graph
//     let egraph = parse_egraph(&filename);

//     // 可视化 e-graph
//     egraph.to_dot_file("egraph_saturated.dot").unwrap();

//     // 获取提取器
//     let extractor = get_extractor(&extractors, &extractor_name);

//     // 修改文件名
//     let modified_name_for_tree_cost =
//         format_modified_name(&modified_filename_for_tree_cost, &extractor_name);
//     let modified_name_for_dag_cost =
//         format_modified_name(&modified_filename_for_dag_cost, &extractor_name);

//     // 记录开始时间
//     let start_time = std::time::Instant::now();

//     if extractor_name != "random-based-faster-bottom-up" {
//         // 非随机提取器处理逻辑
//         let tree_cost_extraction_result =
//             extract_result(extractor, &egraph, &egraph.root_eclasses, &cost_function);

//         let us = start_time.elapsed().as_micros();
//         let cycles = tree_cost_extraction_result
//             .find_cycles(&egraph, &egraph.root_eclasses);
//         println!("Cycles: {:?}", cycles);
//         assert!(cycles.is_empty());

//         let (dag_cost, dag_cost_extraction_result) = tree_cost_extraction_result
//             .calculate_dag_cost_with_extraction_result(&egraph, &egraph.root_eclasses);
//         print_dag_cost(dag_cost);

//         write_json_result(&modified_name_for_tree_cost, &tree_cost_extraction_result);
//         write_json_result(&modified_name_for_dag_cost, &dag_cost_extraction_result);

//         log_result(&filename, &extractor_name, dag_cost, us);
//         write_output_file(
//             &mut out_file,
//             &filename,
//             &modified_name_for_dag_cost,
//             &extractor_name,
//             dag_cost,
//             us,
//         );

//         println!(
//             "Time consumption of tree-based extraction: {} seconds",
//             us as f64 / 1000000.0
//         );
//     } else {
//         // 随机提取器处理逻辑
//         let extractor: Arc<dyn Extractor + Send + Sync> = Arc::new(FasterBottomUpExtractorRandom);
//         let (result_sender, result_receiver) = channel();
//         let cost_function: Arc<str> = Arc::from(cost_function);

//         run_extract_result_parallel(
//             extractor,
//             Arc::new(egraph.clone()),
//             Arc::from(egraph.root_eclasses.clone()),
//             cost_function,
//             random_prob,
//             num_samples,
//             result_sender,
//         );

//         let mut extraction_results = Vec::new();
//         loop {
//             match result_receiver.recv() {
//                 Ok(extraction_result) => {
//                     extraction_results.push(extraction_result);
//                 }
//                 Err(_) => break,
//             }
//         }

//         let modified_name_for_dag_cost = modify_filename(
//             &modified_name_for_dag_cost,
//             ".json",
//             &format!("_k{}.json", k),
//         );
//         for (i, extraction_result) in extraction_results.iter().enumerate() {
//             let (dag_cost, dag_cost_extraction_result_depth) = extraction_result
//                 .calculate_dag_cost_with_extraction_result(&egraph, &egraph.root_eclasses);
//             let dag_cost_file_name = modify_filename(
//                 &modified_name_for_dag_cost,
//                 ".json",
//                 &format!("_{}.json", i),
//             );
//             write_json_result(&dag_cost_file_name, &dag_cost_extraction_result_depth);
//         }
//     }
// }