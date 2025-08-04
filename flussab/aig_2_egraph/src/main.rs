use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::hash::{Hash, Hasher};
use std::io::BufWriter;
use std::fs;
use std::time::Instant;
use egg::*;
mod parser;
mod language;
mod preprocess;
use crate::preprocess::*;
use crate::parser::*;
use std::path::Path;
use crate::language::*;
use flussab::DeferredWriter;
use flussab_aiger::{
    aig::{Renumber, RenumberConfig},
    ascii, binary, Error,
};
use flussab_aiger::aig::Aig;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 7 {
        println!("Usage: <program> <path_to_input_json_file> <path_to_output_circuit_file> <output_file_path> <aig_2_egraph_nodemap_path> <iteration> <rewritten_path>");
        std::process::exit(1);
    }

    let file_path = &args[1];
    let output_path = &args[2];
    let output_file_path = &args[3];
    let aig_2_egraph_nodemap_path = &args[4];
    let iteration = &args[5];
    let rewritten_path = &args[6]; // 新增的路径参数

    let mut file = File::open(file_path).expect("Unable to open file");
    let aig_reader = binary::Parser::<u32>::from_read(file, binary::Config::default())?;
    let ordered_aig = aig_reader.parse()?;
    // Parse the AIG
    // let aig: flussab_aiger::aig::OrderedAig<u32> = aig_reader.parse()?;
    let aig = Aig::from(ordered_aig);
    let config = RenumberConfig::default()
    .trim(false)
    .structural_hash(true)
    .const_fold(true);
    let (aig_order, _renumber) = Renumber::renumber_aig(config, &aig)?;

    let output_file = File::create(output_file_path)?;
    let output_file1 = File::create(aig_2_egraph_nodemap_path)?;

    let mut aag_writer = DeferredWriter::from_write(&output_file);
    let writer = ascii::Writer::<u32>::new(&mut aag_writer);
    writer.write_ordered_aig(&aig_order);
    let (input_vec, output_vec) = writer.collect_symbol(&aig_order);
    let (root_id, input_ids, one_out_sig, node_id_to_egraph_id, outputmap) =
        process_aig_to_egraph(&aig_order, input_vec, output_vec, output_path);

    // 将 HashMap 转换为 Vec，并按键的数字大小排序
    let mut sorted_node_ids: Vec<(&String, &Id)> = node_id_to_egraph_id.iter().collect();
    sorted_node_ids.sort_by(|a, b| {
        a.0.parse::<u32>().unwrap().cmp(&b.0.parse::<u32>().unwrap())
    });

    // 打印排序后的 outputmap
    let mut sorted_outputmap: Vec<(&Id, &String)> = outputmap.iter().collect();
    sorted_outputmap.sort_by(|a, b| a.0.cmp(b.0));
    for (id, op_name) in &sorted_outputmap {
        println!("{:?}: {}", id, op_name);
    }

    let mut writer1 = BufWriter::new(output_file1);
    for (key, value) in &sorted_node_ids {
        writeln!(writer1, "{} {}", key, value)?;
    }

    // Process JSON
    let modified_json_file = process_json_prop(&output_path);
    let converted_json_data =
        fs::read_to_string(&modified_json_file).expect("Unable to read the JSON file");

    // Read egraph from JSON file
    let mut input_egraph: egg::EGraph<Prop, ()> =
        serde_json::from_str(&converted_json_data).unwrap();
    input_egraph.rebuild(); // eqn2egraph finished
    println!("input egraph");
    println!("input node: {}", input_egraph.total_size());
    println!("input class: {}", input_egraph.number_of_classes());

    // 使用动态路径替换所有 rewritten_circuit/
    let serialized_input_egraph_json_path =
        Path::new(rewritten_path).join("rewritten_egraph_with_weight_cost_serd_base.json");
    save_serialized_egraph_to_json(
        &egg_to_serialized_egraph(&input_egraph),
        &serialized_input_egraph_json_path,
        &[root_id.into()],
    )?;

    let runner_iteration_limit = iteration.parse().unwrap_or_else(|_| {
        eprintln!("Invalid iteration value: {}", iteration);
        std::process::exit(1);
    });

    let egraph_node_limit = 200000000;
    let start = Instant::now();
    let mut runner = Runner::default()
        .with_explanations_enabled()
        .with_egraph(input_egraph.clone())
        .with_time_limit(std::time::Duration::from_secs(10))
        .with_iter_limit(runner_iteration_limit)
        .with_node_limit(egraph_node_limit);

    runner.roots = vec![root_id];
    let runner_result = runner.run(&make_rules());

    let duration = start.elapsed();
    println!(
        "Runner stopped: {:?}. Time taken for runner: {:?}, Classes: {}, Nodes: {}, Size: {} \n\n",
        runner_result.stop_reason,
        duration,
        runner_result.egraph.number_of_classes(),
        runner_result.egraph.total_number_of_nodes(),
        runner_result.egraph.total_size()
    );

    let root = runner_result.roots[0];

    // Save output egraph from runner (input for extraction gym)
    let output_egraph_json_path = Path::new(rewritten_path).join("rewritten_egraph_internal.json");
    save_egraph_to_json(&runner_result.egraph, &output_egraph_json_path)?;

    println!("egraph after runner");
    println!("egraph node: {}", runner_result.egraph.total_size());
    println!("egraph class: {}", runner_result.egraph.number_of_classes());

    // Save serialized output egraph to json with root nodes
    let serialized_output_egraph =
        egg_to_serialized_egraph(&runner_result.egraph);
    let serialized_output_egraph_json_path =
        Path::new(rewritten_path).join("rewritten_egraph_internal_serd.json");
    save_serialized_egraph_to_json(
        &serialized_output_egraph,
        &serialized_output_egraph_json_path,
        &[root_id.into()],
    )?;

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

    let output_egraph_cost_json_path =
        Path::new(rewritten_path).join("rewritten_egraph_with_weight_cost_serd.json");
    let mut json_data: serde_json::Value =
        serde_json::from_str(&cost_string)?;
    json_data["root_eclasses"] = serde_json::Value::Array(
            root_values.iter().map(|id| serde_json::Value::String(id.clone())).collect()
        );
    let file = File::create(&output_egraph_cost_json_path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &json_data)?;

    println!("done");
    Ok(())
}