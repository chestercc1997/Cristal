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
    input_file="$1"
    iteration_times="$2"
    cost_function="${3:-area}"
    pattern="${4:-faster-bottom-up}"

    if [ -z "$input_file" ]; then
        read -p "Enter the input file path (required): " input_file
        if [ -z "$input_file" ]; then
            echo "Error: input file is required!"
            exit 1
        fi
    fi

    if [ -z "$iteration_times" ]; then
        read -p "Enter the number of iteration times (optional, default: 30): " iteration_times
        iteration_times=${iteration_times:-30}
    fi

    if [ -z "$cost_function" ]; then
        read -p "Enter the cost function for extraction-gym (optional, could be 'area' or 'delay', default: 'area'): " cost_function
        cost_function=${cost_function:-"area"}
    fi

    if [ -z "$pattern" ]; then
        read -p "Enter the extraction pattern for e-rewriter (optional, could be 'faster-bottom-up' or 'random-based-faster-bottom-up', default: 'faster-bottom-up'): " pattern
        pattern=${pattern:-"faster-bottom-up"}
    fi

    if [[ "$pattern" == *"random"* ]]; then
        read -p "Enter the number of samplings for random pattern (optional, default: 10): " num_samplings
        num_samplings=${num_samplings:-10}
        read -p "Enter the probability of randomization (optional, default: 0.5): " prob_randomization
        prob_randomization=${prob_randomization:-0.5}
        read -p "Enter the sub directory name (optional, default: 1): " sub_dir
        sub_dir=${sub_dir:-1}
    else
        sub_dir=$(dirname "$input_file" | xargs basename)
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
    local source_data="$input_file"
    printf "${CYAN}[DEBUG] Source data absolute path: ${source_data}${RESET}\n"
    change_dir "extraction-gym/"
    local OUTPUT_DIR="output_log/${sub_dir}"
    mkdir -p "${OUTPUT_DIR}"
    local base_name=$(basename "${source_data}" .json)
    local out_file="${OUTPUT_DIR}/log-${base_name}-${pattern}.json"
    if [[ "$pattern" == *"random"* ]]; then
        target/release/extraction-gym \
            "${source_data}" \
            --cost-function="${cost_function}" \
            --extractor="${pattern}" \
            --out="${out_file}" \
            --num-samples="${num_samplings}" \
            --random-prob="${prob_randomization}" \
            --sub-dir="${sub_dir}"
    else
        target/release/extraction-gym \
            "${source_data}" \
            --cost-function="${cost_function}" \
            --extractor="${pattern}" \
            --out="${out_file}" \
            --sub-dir="${sub_dir}"
    fi
    change_dir ".."
    end_time_process_extract=$(date +%s.%N)
    runtime_process_extract=$(echo "$end_time_process_extract - $start_time_process_extract" | bc)
    echo -e "${GREEN}Process 2 - Extract DAG completed.${RESET}"
}

process_json() {
    echo -e "${YELLOW}<-----------------------------Process 3: Process JSON----------------------------->${RESET}"
    start_time_process_process_json=$(date +%s.%N)
    BASE_DIR="$(cd "$(dirname "$0")" && pwd)"
    local file_name=$(basename "$input_file")
    local source_data="$BASE_DIR/extract_or_replace/rewritten_circuit/${sub_dir}/${file_name}"
    local extracted_dir="$BASE_DIR/extraction-gym/out_dag_json/${sub_dir}"
    local output_base_dir="$BASE_DIR/process_json/out_process_or_result/${sub_dir}"
    local input_extracted="${extracted_dir}/${file_name}"
    local output_path="${output_base_dir}/${file_name}"
    mkdir -p "${output_base_dir}" || error "Failed to create base dir: ${output_base_dir}"
    change_dir "process_json/"
    local output_dir=$(dirname "${output_path}")
    mkdir -p "${output_dir}" || error "Failed to create output dir: ${output_dir}"
    execute_command "target/release/process_json -s '${source_data}' -e '${input_extracted}' -o '${output_path}' -g"
    end_time_process_process_json=$(date +%s.%N)
    runtime_process_process_json=$(echo "$end_time_process_process_json - $start_time_process_process_json" | bc)
    echo -e "${GREEN}Process 3 - Extract DAG and Process JSON completed.${RESET}"
    change_dir ".."
}

setup_directories
get_user_input
extract_dag
process_json