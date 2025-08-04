#!/bin/bash
RED="\e[31m"
GREEN="\e[32m"
YELLOW="\e[1;33m"
RESET="\e[0m"

# Utility function for creating directories if they do not exist
ensure_dir() {
    if [ ! -d "$1" ]; then
        mkdir -p "$1" || { echo -e "${RED}Failed to create directory $1${RESET}"; exit 1; }
    fi
}

# Utility function for changing directories safely
change_dir() {
    cd "$1" || { echo -e "${RED}Failed to change directory to $1${RESET}"; exit 1; }
}

# Utility function for copying files safely
copy_file() {
    cp "$1" "$2" || { echo -e "${RED}Failed to copy $1 to $2${RESET}"; exit 1; }
}

# Utility function to execute a command and handle failure
execute_command() {
    eval "$1" || { echo -e "${RED}Failed to execute command: $1${RESET}"; exit 1; }
}

# Function to set up required directories
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

# Function to get user input
get_user_input() {
    read -p "Enter the number of iteration times (optional, default: 1): " iteration_times
    iteration_times=${iteration_times:-30}

    read -p "Enter the cost function for extraction-gym (optional, could be 'area' or 'delay', default: 'area'): " cost_function
    cost_function=${cost_function:-"area"}

    read -p "Enter the extraction pattern for e-rewriter (optional, could be 'faster-bottom-up' or 'random-based-faster-bottom-up', default: 'faster-bottom-up'): " pattern
    pattern=${pattern:-"faster-bottom-up"}

    # if pattern is provided with *random*
    if [[ "$pattern" == *"random"* ]]; then
        read -p "Enter the number of samplings for random pattern (optional, default: 10): " num_samplings
        num_samplings=${num_samplings:-30}

        read -p "Enter the probability of randomization (optional, default: 0.5): " prob_randomization
        prob_randomization=${prob_randomization:-0.1}
    fi

    # if cost_function is 'area', replace it with 'node_sum_cost', if it is 'delay', replace it with 'node_depth_cost'
    if [ "$cost_function" == "area" ]; then
        cost_function="node_sum_cost"
    elif [ "$cost_function" == "delay" ]; then
        cost_function="node_depth_cost"
    fi
}



# Function to extract the DAG

extract_dag() {
    echo -e "${YELLOW}<-----------------------------Process 2: Extract DAG------------------------------>${RESET}"
    start_time_process_extract=$(date +%s.%N)
    echo -e "${YELLOW}Running extraction gym...${RESET}"
    # cp "extract_or_replace/rewritten_circuit/rewritten_egraph_with_weight_cost_serd.json" "extraction-gym/input/"
    local source_data="/export2/cchen099/E-syn2/extract_or_replace/rewritten_circuit/${sub_dir}/rewritten_egraph_with_weight_cost_serd.json"
    change_dir "extraction-gym/"

    # Creating the output directory if it doesn't exist
    OUTPUT_DIR="output_log"
    #ext="faster-bottom-up"
    ext=${pattern}
    mkdir -p ${OUTPUT_DIR}

    # running the extraction process
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


# Function to process JSON
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
        
        # Parallel execution of process_json for each extracted egraph
        ls input_extracted_egraph/* | parallel --eta "target/release/process_json -s ${input_saturacted_egraph_path} -e {} -o out_process_dag_result/{/} -g"
        file_to_delete="/export2/cchen099/E-syn2/process_json/out_process_dag_result/rewritten_egraph_with_weight_cost_serd_faster-bottom-up.json"
        if [ -f "$file_to_delete" ]; then
            echo -e "${YELLOW}File $file_to_delete exists. Deleting it...${RESET}"
            rm "$file_to_delete" || {
                echo -e "${RED}Error: Failed to delete $file_to_delete${RESET}"
                exit 1
            }
        fi
        change_dir ".."

        echo -e "${YELLOW}Copying rewritten and extracted egraph files ... Prepare graph for Equation conversion.${RESET}"
        for file in process_json/out_process_dag_result/*; do
            copy_file "$file" "graph2eqn/${file##*/}"
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
        copy_file "process_json/out_process_dag_result/rewritten_egraph_with_weight_cost_serd_${pattern}.json" "extract_or_replace/out_after_replace"
    fi

    end_time_process_process_json=$(date +%s.%N)
    runtime_process_process_json=$(echo "$end_time_process_process_json - $start_time_process_process_json" | bc)
    echo -e "${GREEN}Process 3 - Extract DAG and Process JSON completed.${RESET}"
}



setup_directories
get_user_input 
extract_dag # extract from saturated egraph, extract dag
process_json # extract from saturated egraph, process json
