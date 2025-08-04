use std::fs::File;
use std::io::Read;
use std::env;
use flussab::DeferredWriter;
use flussab_aiger::{
    aig::{Renumber, RenumberConfig},
    ascii, binary, Error,
};
use flussab_aiger::traversal::*;
use flussab_aiger::aig::Aig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    #[cfg(feature = "egraph2aig")]
    {
        if args.len() != 4 {
            eprintln!("Usage: program <output_file1> <output_file2> <input_egraph_path>");
            std::process::exit(1);
        }
        let output_file_path1 = &args[1];
        let output_file_path2 = &args[2];
        let input_file_path = &args[3];

        let mut file = File::open(input_file_path).expect("Unable to open file");
        let mut json_str = String::new();
        file.read_to_string(&mut json_str).expect("Unable to read file");

        let graph = parse_json_sd(&json_str);

        let input_vec = generate_input_vec(&graph);
        let mut graph_reorder = graph.reorder(input_vec.clone());

        let mut sorted_nodes: Vec<_> = graph_reorder.nodes.iter().collect();
        sorted_nodes.sort_by_key(|(id, _)| id.parse::<usize>().unwrap());

        graph_reorder.filter_nodes_by_op();

        let mut filtered_nodes: Vec<_> = graph_reorder.nodes.iter().collect();
        filtered_nodes.sort_by_key(|(_, node)| node.order.unwrap_or_default());

        let aig: flussab_aiger::aig::Aig<u32> = graph_reorder.to_aig();

        let config = RenumberConfig::default()
            .trim(true)
            .structural_hash(true)
            .const_fold(true);
        let (aig_order, _renumber) = Renumber::renumber_aig(config, &aig)?;

        let output_file = File::create(output_file_path1)?;
        let mut aag_writer = DeferredWriter::from_write(&output_file);
        let writer = ascii::Writer::<u32>::new(&mut aag_writer);
        writer.write_ordered_aig(&aig_order);

        let output_file2 = File::create(output_file_path2)?;
        let aig_writer = DeferredWriter::from_write(&output_file2);
        let mut binary_writer = binary::Writer::<u32>::new(aig_writer);
        binary_writer.write_ordered_aig(&aig_order);
    }

    #[cfg(feature = "extract_cone")]
    {
        if args.len() != 5 {
            eprintln!(
                "Usage: program <output_file1> <output_file2> <input_aig_path> <gate_output>"
            );
            std::process::exit(1);
        }

        let output_file_path1 = &args[1];
        let output_file_path2 = &args[2];
        let input_file_path = &args[3];
        let gate_output: u32 = args[4]
            .parse()
            .expect("Invalid gate_output, must be a valid unsigned integer");

        let file = File::open(input_file_path)?;
        let aig_reader = binary::Parser::<u32>::from_read(file, binary::Config::default())?;
        let ordered_aig = aig_reader.parse()?;
        let aig = Aig::from(ordered_aig);
        let aig: flussab_aiger::aig::Aig<u32> = aig.extract_cone(gate_output);

        let config = RenumberConfig::default()
            .trim(true)
            .structural_hash(true)
            .const_fold(true);
        let (aig_order, _renumber) = Renumber::renumber_aig(config, &aig)?;

        let output_file = File::create(output_file_path1)?;
        let mut aag_writer = DeferredWriter::from_write(&output_file);
        let writer = ascii::Writer::<u32>::new(&mut aag_writer);
        writer.write_ordered_aig(&aig_order);

        let output_file2 = File::create(output_file_path2)?;
        let aig_writer = DeferredWriter::from_write(&output_file2);
        let mut binary_writer = binary::Writer::<u32>::new(aig_writer);
        binary_writer.write_ordered_aig(&aig_order);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use flussab::DeferredWriter;
    use flussab_aiger::{
        aig::{Renumber, RenumberConfig},
        ascii, binary,
    };
    use flussab_aiger::aig::Aig;

    #[test]
    fn test_read_and_write_aig() -> Result<(), Box<dyn std::error::Error>> {
        let input_file_path = "/data/cchen/choice_revisit/flussab/aig_2_egraph/mffc/1.aig";
        let output_file_path = "/data/cchen/choice_revisit/flussab/aig_2_egraph/aag/1.aag";

        let file = File::open(input_file_path)?;
        let aig_reader = binary::Parser::<u32>::from_read(file, binary::Config::default())?;
        let ordered_aig = aig_reader.parse()?;
        let aig = Aig::from(ordered_aig);

        let config = RenumberConfig::default()
            .trim(false)
            .structural_hash(true)
            .const_fold(true);
        let (aig_order, _renumber) = Renumber::renumber_aig(config, &aig)?;

        let output_file = File::create(output_file_path)?;
        let mut aag_writer = DeferredWriter::from_write(&output_file);
        let writer = ascii::Writer::<u32>::new(&mut aag_writer);
        writer.write_ordered_aig(&aig_order);

        assert!(std::path::Path::new(output_file_path).exists());

        Ok(())
    }
}