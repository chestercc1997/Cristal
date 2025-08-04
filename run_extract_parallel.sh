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
        read -p "Enter the sub directory name (optional, default: 1): " sub_dir
        sub_dir=${sub_dir:-1}
    fi
    if [ "$cost_function" == "area" ]; then
        cost_function="node_sum_cost"
    elif [ "$cost_function" == "delay" ]; then
        cost_function="node_depth_cost"
    fi
    echo "[DEBUG] User input sub_dir: $sub_dir"
}

extract_dag() {
    echo -e "${YELLOW}<-----------------------------Process 2: Extract DAG------------------------------>${RESET}"
    start_time_process_extract=$(date +%s.%N)
    echo -e "${YELLOW}Running extraction gym...${RESET}"
    local source_data="$(pwd)/flussab/aig_2_egraph/rewritten_circuits/${sub_dir}/rewritten_egraph_with_weight_cost_serd.json"
    echo -e "${CYAN}[DEBUG] Source data absolute path: ${source_data}${RESET}"
    change_dir "extraction-gym/"
    OUTPUT_DIR="output_log"
    ext=${pattern}
    mkdir -p ${OUTPUT_DIR}
    base_name=$(basename "${source_data}" .json)
    out_file="${OUTPUT_DIR}/log-${base_name}-${ext}.json"
    if [[ "$pattern" == *"random"* ]]; then
        target/release/extraction-gym "${source_data}" --cost-function="${cost_function}" --extractor="${pattern}" --out="${out_file}" --num-samples="${num_samplings}" --random-prob="${prob_randomization}" --sub-dir="${sub_dir}"
    else
        target/release/extraction-gym "${source_data}" --cost-function="${cost_function}" --extractor="${pattern}" --out="${out_file}"
    fi
    change_dir ".."
    end_time_process_extract=$(date +%s.%N)
    runtime_process_extract=$(echo "$end_time_process_extract - $start_time_process_extract" | bc)
    echo -e "${GREEN}Process 2 - Extract DAG completed.${RESET}"
}

process_json() {
    echo -e "${YELLOW}<-----------------------------Process 3: Process JSON----------------------------->${RESET}"
    start_time_process_process_json=$(date +%s.%N)
    local script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
    local source_data="${script_dir}/flussab/aig_2_egraph/rewritten_circuits/${sub_dir}/rewritten_egraph_with_weight_cost_serd.json"
    local extracted_dir="${script_dir}/extraction-gym/random_out_dag_json/${sub_dir}"
    local output_base_dir="${script_dir}/process_json/out_process_dag_result/${sub_dir}"
    echo "[DEBUG] Script Directory: ${script_dir}"
    echo "[DEBUG] Source Data Path: ${source_data}"
    echo "[DEBUG] Extracted Dir: ${extracted_dir}"
    echo "[DEBUG] Output Dir: ${output_base_dir}"
    if [[ ! -f "${source_data}" ]]; then
        echo -e "${RED}Error: Source data file not found: ${source_data}${RESET}"
        return 1
    fi
    if [[ ! -d "${extracted_dir}" ]]; then
        echo -e "${RED}Error: Extracted directory not found: ${extracted_dir}${RESET}"
        return 1
    fi
    change_dir "${script_dir}/process_json/"
    mkdir -p "${output_base_dir}"
    if [[ "$pattern" == *"random"* ]]; then
        find "${extracted_dir}" -maxdepth 1 -type f -name "*.json" | parallel --eta \
            "target/release/process_json \
            -s '${source_data}' \
            -e '{}' \
            -o '${output_base_dir}/{/}' \
            -g"
    fi
    change_dir "${script_dir}"
    end_time_process_process_json=$(date +%s.%N)
    runtime_process_process_json=$(echo "$end_time_process_process_json - $start_time_process_process_json" | bc)
    echo -e "${GREEN}Process 3 - Extract DAG and Process JSON completed.${RESET}"
}

setup_directories
get_user_input
extract_dag
process_json