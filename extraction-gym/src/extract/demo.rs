// use std::env;
// use std::process;

// //use abc::Abc;

// use crate::extract::lib::Abc;

// fn main() {
//     let args: Vec<String> = env::args().collect();
//     if args.len() != 2 {
//         eprintln!("Usage: {} <equation_file.eqn>", args[0]);
//         process::exit(1);
//     }

//     let equation_file = &args[1];

//     let mut abc = Abc::new();

//     println!("Reading equation file...");
//     abc.execute_command(&format!("read_eqn {}", equation_file));
//     println!("Reading library...");
//     abc.execute_command("read_lib asap7_clean.lib");
//     println!("Performing structural hashing...");
//     abc.execute_command("strash");
//     println!("Performing technology mapping...");
//     abc.execute_command("map");
//     println!("Performing post-processing...(topo; gate sizing)");
//     abc.execute_command("topo");
//     abc.execute_command("upsize");
//     abc.execute_command("dnsize");
    
//     println!("Executing stime command...");
//     let stime_output = abc.execute_command_with_output("stime -d");
//     //println!("stime output: {}", stime_output);

//     if let Some(delay) = parse_delay(&stime_output) {
//         //println!("Delay: {} ps", delay);
//         let delay_ns = delay / 1000.0;
//         println!("Delay in nanoseconds: {} ns", delay_ns);
//     } else {
//         println!("Failed to parse delay value");
//     }
// }

// fn parse_delay(output: &str) -> Option<f32> {
//     //println!("Parsing delay from output: {}", output);
//     for line in output.lines() {
//         //println!("Checking line: {}", line);
//         if line.contains("Delay") {
//             //println!("Found Delay line: {}", line);
//             let parts: Vec<&str> = line.split_whitespace().collect();
//             if parts.len() >= 3 {
//                 //println!("Attempting to parse: {}", parts[1]);
//                 return parts[1].parse::<f32>().ok();
//             }
//         }
//     }
//     None
// }