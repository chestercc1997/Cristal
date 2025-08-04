#!/bin/bash
RED="\e[31m"
GREEN="\e[32m"
YELLOW="\e[1;33m"
RESET="\e[0m"

ensure_dir() {
    if [ ! -d "$1" ]; then
        mkdir -p "$1" || { echo -e "${RED}Failed to create directory $1${RESET}"; exit 1; }
    fi
}

change_dir() {
    cd "$1" || { echo -e "${RED}Failed to change directory to $1${RESET}"; exit 1; }
}

copy_file() {
    cp "$1" "$2" || { echo -e "${RED}Failed to copy $1 to $2${RESET}"; exit 1; }
}

execute_command() {
    eval "$1" || { echo -e "${RED}Failed to execute command: $1${RESET}"; exit 1; }
}

setup_directories() {
    echo -e "${GREEN}Setting up required directories...${RESET}"
    ensure_dir "e-rewriter/rewritten_circuit"
    ensure_dir "e-rewriter/random_graph"
    ensure_dir "extraction-gym/input"
    ensure_dir "extraction-gym/out_dag_json"
    ensure_dir "extraction-gym/out_json"
    ensure_dir "extraction-gym/output_log"
    ensure_dir "process_json/input_saturacted_egraph"
    ensure_dir "process_json/input_extracted_egraph"
    ensure_dir "process_json/out_process_dag_result"
    ensure_dir "extraction-gym/random_out_dag_json/"
    echo -e "${GREEN}Setup complete.${RESET}\n"
}

get_user_input() {
    read -p "Enter the number of iteration times (optional, default: 1): " iteration_times
    iteration_times=${iteration_times:-30}
    read -p "Enter the cost function for extraction-gym (optional, could be 'area' or 'delay', default: 'area'): " cost_function
    cost_function=${cost_function:-"area"}
    read -p "Enter the extraction pattern for e-rewriter (optional, could be 'faster-bottom-up' or 'random-based-faster-bottom-up', default: 'faster-bottom-up'): " pattern
    pattern=${pattern:-"faster-bottom-up"}
    if [[ "$pattern" == *"random"* ]]; then
        read -p "Enter the number of samplings for random pattern (optional, default: 10): " num_samplings
        num_samplings=${num_samplings:-30}
        read -p "Enter the probability of randomization (optional, default: 0.5): " prob_randomization
        prob_randomization=${prob_randomization:-0.1}
    fi
    if [ "$cost_function" == "area" ]; then
        cost_function="node_sum_cost"
    elif [ "$cost_function" == "delay" ]; then
        cost_function="node_depth_cost"
    fi
}

extract_dag() {
    echo -e "${YELLOW}<-----------------------------Process 2: Extract DAG------------------------------>${RESET}"
    start_time_process_extract=$(date +%s.%N)
    echo -e "${YELLOW}Running extraction gym...${RESET}"
    change_dir "extraction-gym/"
    OUTPUT_DIR="output_log"
    ext=${pattern}
    mkdir -p ${OUTPUT_DIR}
    data="input/rewritten_egraph_with_weight_cost_serd.json"
    base_name=$(basename "${data}" .json)
    out_file="${OUTPUT_DIR}/log-${base_name}-${ext}.json"
    echo "Running extractor for ${data} with ${ext}"
    if [[ "$pattern" == *"random"* ]]; then
        target/release/extraction-gym "${data}" --cost-function="${cost_function}" --extractor="${pattern}" --out="${out_file}" --num-samples="${num_samplings}" --random-prob="${prob_randomization}"
    else
        target/release/extraction-gym "${data}" --cost-function="${cost_function}" --extractor="${pattern}" --out="${out_file}"
    fi
    change_dir ".."
    end_time_process_extract=$(date +%s.%N)
    runtime_process_extract=$(echo "$end_time_process_extract - $start_time_process_extract" | bc)
    echo -e "${GREEN}Process 2 - Extract DAG completed.${RESET}"
}

process_json() {
    echo -e "${YELLOW}<-----------------------------Process 3: Process JSON----------------------------->${RESET}"
    start_time_process_process_json=$(date +%s.%N)
    copy_file "extraction-gym/input/rewritten_egraph_with_weight_cost_serd.json" "process_json/input_saturacted_egraph/"
    if [[ "$pattern" == *"random"* ]]; then
        for file in extraction-gym/random_out_dag_json/*; do
            copy_file "$file" "process_json/input_extracted_egraph/"
        done
        change_dir "process_json/"
        input_saturacted_egraph_path="input_saturacted_egraph/rewritten_egraph_with_weight_cost_serd.json"
        ls input_extracted_egraph/* | parallel --eta "target/release/process_json -s ${input_saturacted_egraph_path} -e {} -o out_process_dag_result/{/} -g"
        change_dir ".."
        echo -e "${YELLOW}Copying rewritten and extracted egraph files ... Prepare graph for Equation conversion.${RESET}"
        for file in process_json/out_process_dag_result/*; do
            copy_file "$file" "choose_net_build/choose_result"
        done
    else
        copy_file "extraction-gym/out_dag_json/rewritten_egraph_with_weight_cost_serd_${pattern}.json" "process_json/input_extracted_egraph/"
        change_dir "process_json/"
        input_saturacted_egraph_path="input_saturacted_egraph/rewritten_egraph_with_weight_cost_serd.json"
        input_extracted_egraph_path="input_extracted_egraph/rewritten_egraph_with_weight_cost_serd_${pattern}.json"
        output_path="out_process_dag_result/rewritten_egraph_with_weight_cost_serd_${pattern}.json"
        execute_command "target/release/process_json -s ${input_saturacted_egraph_path} -e ${input_extracted_egraph_path} -o ${output_path} -g"
        change_dir ".."
        echo -e "${YELLOW}Copying rewritten and extracted egraph files ... Prepare graph for Equation conversion.${RESET}"
    fi
    end_time_process_process_json=$(date +%s.%N)
    runtime_process_process_json=$(echo "$end_time_process_process_json - $start_time_process_process_json" | bc)
    echo -e "${GREEN}Process 3 - Extract DAG and Process JSON completed.${RESET}"
}

setup_directories
get_user_input
extract_dag
process_json