use std::env;
use serde::{Deserialize, Serialize}; // Import Serialize
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::hash::{Hash, Hasher};
use std::io::BufWriter;
use std::fs;
use std::time::Instant;
use egg::*;
use std::collections::HashMap;
use rand::prelude::SliceRandom;
mod parser;
mod preprocess;
mod language;
use crate::preprocess::*;
use crate::parser::*;
use crate::language::*;
use std::process;
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: <program> <input_path> <output_path>");
        return Ok(());
    }
    let current_dir = env::current_dir()?;

    let input_path = &args[1];
    let file_name = Path::new(input_path) // 将 input_path 转换为 Path
    .file_name()
    .and_then(|n| n.to_str()) // 转换为字符串
    .ok_or("Invalid input file name")?;
    let mut file1 = File::open(input_path).expect("Unable to open file1");
    let mut json_str1 = String::new();
    file1.read_to_string(&mut json_str1).expect("Unable to read file1");
    let graph1: Graph = parse_json(&json_str1);
    let root = &graph1.root_eclasses[0];
    let egraph1: egraph_serialize::EGraph=parse_egraph_from_str(&json_str1);
    let traversal_order = dfs_traversal(&graph1, root);
// 打印 traversal_order
    // println!("Traversal order: {:?}", traversal_order);
    let sub_dir = &args[2];
    let output_path = current_dir
    .join("output_file")
    .join(sub_dir)
    .join(file_name);

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
     //   println!("Created output directory: {}", parent.display());
    }
    
    let (root_id, input_id, one_out_sig) =
    process_file_sig_output(
        &graph1,
        traversal_order,
        output_path.to_str().unwrap(), // 转换为 &str
    );
    let mut root_ids: Vec<usize> = vec![root_id.into()];
    let modified_json_file = process_json_prop(output_path.to_str().unwrap());
    let converted_json_data =
        fs::read_to_string(&modified_json_file).expect("Unable to read the JSON file");
    let mut input_egraph: egg::EGraph<Prop,()> = serde_json::from_str(&converted_json_data).unwrap();
    input_egraph.rebuild();//eqn2egraph finished

    let converted_egg = input_egraph.clone();
    // println!("total");
    // println!("input node: {}", converted_egg.total_size());
    // println!("input class: {}", converted_egg.number_of_classes());
    // // Transfer egg::egraph to serialized_egraph and save it into json file
    // current_dir
    //     .join("rewritten_circuit")
    //     .join(sub_dir)
    //     .join(file_name);
    let serialized_input_egraph = egg_to_serialized_egraph(&converted_egg);
    let serialized_input_egraph_json_path = 
    current_dir
    .join("output_file")
    .join(sub_dir)
    .join(format!("serd_{}", file_name));
    //env::current_dir().unwrap().join("rewritten_circuit/egraph2egraph_serd.json"); // egraph to serialized_egraph finished

    save_serialized_egraph_to_json(&serialized_input_egraph, &serialized_input_egraph_json_path, &root_ids)?;
    // Rewrite time!
    
      let runner_iteration_limit = env::args()
          .nth(2)
          .unwrap_or("10".to_string())
          .parse()
          .unwrap_or(20);
      let egraph_node_limit = 200000000;
      let start = Instant::now();
      
      let mut runner = Runner::default()
          .with_explanations_enabled()
          .with_egraph(input_egraph.clone())
          .with_time_limit(std::time::Duration::from_secs(10))
          .with_iter_limit(runner_iteration_limit)
          .with_node_limit(egraph_node_limit);
  
          runner.roots = root_ids.iter().cloned().map(Id::from).collect();
          println!("runner.roots: {:?}", runner.roots);
      let runner_result = runner.run(&make_rules_or_replace());
  
      let duration = start.elapsed();
    //   println!(
    //       "Runner stopped: {:?}. Time taken for runner: {:?}, Classes: {}, Nodes: {}, Size: {} \n\n",
    //       runner_result.stop_reason,
    //       duration,
    //       runner_result.egraph.number_of_classes(),
    //       runner_result.egraph.total_number_of_nodes(),
    //       runner_result.egraph.total_size()
    //   );
     // println!("root{:?}", runner_result.roots);
     // runner_result.print_report();
      let root = runner_result.roots;
    //  println!("root{:?}", root);
      // Save output egraph from runner (input for extraction gym)
    //   let output_egraph_json_path = env::current_dir().unwrap().join("rewritten_circuit/rewritten_egraph_internal.json");
    //   save_egraph_to_json(&runner_result.egraph, &output_egraph_json_path)?;
      let serialized_output_egraph = egg_to_serialized_egraph(&runner_result.egraph);
    //   let serialized_output_egraph_json_path = env::current_dir().unwrap().join("rewritten_circuit/rewritten_egraph_or_serd.json");
    //   save_serialized_egraph_to_json(&serialized_output_egraph, &serialized_output_egraph_json_path, &root_ids)?;

    println!("------------------assign cost of enode-----------------");
    let json_string = serde_json::to_string(&serialized_output_egraph).unwrap();

    let graph2: Graph = parse_json(&json_string);
    let mut root_values: Vec<String> = Vec::new();
    for (node_id, node) in graph2.nodes.iter() {
        if node.op == "@" {
            for child in &node.children {
                // 去掉 .0 部分
                if let Some(new_root) = child.strip_suffix(".0") {
                    root_values.push(new_root.to_string());
                }
            }
        }
    }

    let cost_string = process_json_prop_cost(&json_string);


    let output_egraph_cost_json_path = current_dir
        .join("rewritten_circuit")
        .join(sub_dir)
        .join(file_name);
    
    // 确保路径中的目录存在
    if let Some(parent_dir) = output_egraph_cost_json_path.parent() {
        std::fs::create_dir_all(parent_dir)?; // 递归创建父目录
    }
    
    let mut json_data: serde_json::Value = serde_json::from_str(&cost_string)?;
    // json_data["root_eclasses"] = serde_json::Value::Array(
    //     root_ids.iter().map(|id| serde_json::Value::String(id.to_string())).collect()
    // );
    json_data["root_eclasses"] = serde_json::Value::Array(
        root_values.iter().map(|id| serde_json::Value::String(id.clone())).collect()
    );
    let file = File::create(&output_egraph_cost_json_path)?; // 创建文件
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &json_data)?;
    
    println!("done");
    Ok(())
}
// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let args: Vec<String> = env::args().collect();
//     if args.len() < 3 {
//         eprintln!("Usage: <program> <input_path> <output_path>");
//         return Ok(());
//     }
//     let current_dir = env::current_dir()?;
//     println!("Current working directory: {}", current_dir.display());
//     let input_path = &args[1];
//     let mut file1 = File::open(input_path).expect("Unable to open file1");
//     let mut json_str1 = String::new();
//     file1.read_to_string(&mut json_str1).expect("Unable to read file1");
//     let graph1: Graph = parse_json(&json_str1);
//     let root = &graph1.root_eclasses[0];
//     let traversal_order = dfs_traversal(&graph1, root);
//     let output_path = &args[2];
//     let (root_id, input_id, one_out_sig) =
//         process_file_sig_output(&graph1, traversal_order, output_path);
//     let mut root_ids: Vec<usize> = vec![root_id.into()];
//     let modified_json_file = process_json_prop(&output_path);
//     let converted_json_data =
//         fs::read_to_string(&modified_json_file).expect("Unable to read the JSON file");
//     let mut input_egraph: egg::EGraph<Prop, ConstantFold> = serde_json::from_str(&converted_json_data).unwrap();
//     input_egraph.rebuild();//eqn2egraph finished
//     let converted_egg = input_egraph.clone();
//     println!("total");
//     println!("input node: {}", converted_egg.total_size());
//     println!("input class: {}", converted_egg.number_of_classes());
//     // Transfer egg::egraph to serialized_egraph and save it into json file
//     let serialized_input_egraph = egg_to_serialized_egraph(&converted_egg);
//     let serialized_input_egraph_json_path = env::current_dir().unwrap().join("rewritten_circuit/egraph2egraph_serd.json"); // egraph to serialized_egraph finished
//     save_serialized_egraph_to_json(&serialized_input_egraph, &serialized_input_egraph_json_path, &root_ids)?;
//     // Rewrite time!
    
//       let runner_iteration_limit = env::args()
//           .nth(2)
//           .unwrap_or("10".to_string())
//           .parse()
//           .unwrap_or(20);
//       let egraph_node_limit = 200000000;
//       let start = Instant::now();
      
//       let mut runner = Runner::default()
//           .with_explanations_enabled()
//           .with_egraph(input_egraph.clone())
//           .with_time_limit(std::time::Duration::from_secs(10))
//           .with_iter_limit(runner_iteration_limit)
//           .with_node_limit(egraph_node_limit);
  
//           runner.roots = root_ids.iter().cloned().map(Id::from).collect();
//           println!("runner.roots: {:?}", runner.roots);
//       let runner_result = runner.run(&make_rules_or_replace());
  
//       let duration = start.elapsed();
//       println!(
//           "Runner stopped: {:?}. Time taken for runner: {:?}, Classes: {}, Nodes: {}, Size: {} \n\n",
//           runner_result.stop_reason,
//           duration,
//           runner_result.egraph.number_of_classes(),
//           runner_result.egraph.total_number_of_nodes(),
//           runner_result.egraph.total_size()
//       );
//      // println!("root{:?}", runner_result.roots);
//       runner_result.print_report();
//       let root = runner_result.roots;
//       println!("root{:?}", root);
//       // Save output egraph from runner (input for extraction gym)
//       let output_egraph_json_path = env::current_dir().unwrap().join("rewritten_circuit/rewritten_egraph_internal.json");
//       save_egraph_to_json(&runner_result.egraph, &output_egraph_json_path)?;
  
//       println!("egraph after runner");
//       println!("egraph node: {}", runner_result.egraph.total_size());
//       println!("egraph class: {}", runner_result.egraph.number_of_classes());
//       let serialized_output_egraph = egg_to_serialized_egraph(&runner_result.egraph);
//       let serialized_output_egraph_json_path = env::current_dir().unwrap().join("rewritten_circuit/rewritten_egraph_or_serd.json");
//       save_serialized_egraph_to_json(&serialized_output_egraph, &serialized_output_egraph_json_path, &root_ids)?;

//       println!("------------------assign cost of enode-----------------");
//       let json_string = serde_json::to_string(&serialized_output_egraph).unwrap();
//       let cost_string = process_json_prop_cost(&json_string);

//       let output_egraph_cost_json_path = env::current_dir().unwrap().join("rewritten_circuit/rewritten_egraph_with_weight_cost_serd.json");
//       let mut json_data: serde_json::Value = serde_json::from_str(&cost_string)?;
//       json_data["root_eclasses"] = serde_json::Value::Array(root_ids.iter().map(|id| serde_json::Value::String(id.to_string())).collect());
//       let file = File::create(&output_egraph_cost_json_path)?;
//       let writer = BufWriter::new(file);
//       serde_json::to_writer_pretty(writer, &json_data)?;

//       println!("done");
//       Ok(())
// }