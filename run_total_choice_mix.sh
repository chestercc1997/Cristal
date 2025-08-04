#!/bin/bash
set -e
iteration_rewrite="5"
num_files_extraction="10"
num_save0="10"
num_save1="3"
filter_by_simulation="0"
random_prob_set="0.2"
delay="1"  #
critical_nodes="1"
BASE_DIR="$(cd "$(dirname "$0")" && pwd)"
# mode="0"
mode="1"
# mode="3"
if [[ "$mode" == "3" ]]; then
    filter_by_simulation="0"
else
    filter_by_simulation="1"
fi

if [[ "$filter_by_simulation" == "1" ]]; then
    num_files_save="$num_save0"
else
    num_files_save="$num_files_extraction"
fi
echo "filter_by_simulation=$filter_by_simulation, num_files_save=$num_files_save"

if [ "$delay" -eq 1 ]; then
    benchmark_subdir="exp_delay"
else
    benchmark_subdir="exp_area"
fi
# aig path
SCRIPT_DIR="$BASE_DIR"  
# parameter settings
while [[ $# -gt 0 ]]; do
    key="$1"
    case $key in
        --iteration)
            iteration_rewrite="$2"
            shift; shift ;;
        --num_files)
            num_files_extraction="$2"
            shift; shift ;;
        --random_prob)
            random_prob_set="$2"
            shift; shift ;;
        --aig_file)
            AIG_FILE="$2"
            shift; shift ;;
        --aig_file_init)
            AIG_FILE1="$2"
            shift; shift ;;    
        --delay)
            delay="$2"
            shift; shift ;;
        --case)
            aig_name="$2"
            shift; shift ;;
        *)
            echo "Unknown parameter: $1"
            exit 1 ;;
    esac
done

AIG_FILE="benchmarks/exp/benchmarks/${benchmark_subdir}/${aig_name}"
AIG_FILE1="benchmarks/exp/benchmarks/exp_aig/${aig_name}"

AIG_FILE_real="$BASE_DIR/$AIG_FILE"
AIG_FILE_real1="$BASE_DIR/$AIG_FILE1"
echo "parameter settings:"
echo "  iteration_rewrite=$iteration_rewrite"
echo "  AIG_FILE=$AIG_FILE"
echo "  num_files_extraction=$num_files_extraction"
echo "  random_prob_set=$random_prob_set"
echo "  AIG_FILE_real=$AIG_FILE_real"
echo "  delay_mode=$delay"

RED="\e[31m"; GREEN="\e[32m"; YELLOW="\e[33m"; CYAN="\e[36m"; RESET="\e[0m"
AIG_2_EGRAPH_DIR="flussab/aig_2_egraph"
CHOOSE_NET_BUILD_DIR="$BASE_DIR/choose_net_build"
INPUT_FILE_DIR="$CHOOSE_NET_BUILD_DIR/input_file"
OUTPUT_FILE_DIR="$CHOOSE_NET_BUILD_DIR/output_file"
ROOT_MD_EGRAPH_DIR="$CHOOSE_NET_BUILD_DIR/root_md_egraph"
PROCESS_SCRIPT="$BASE_DIR/process_choice.py"
PROCESSED_DIR="$BASE_DIR/process_json/out_process_dag_result" 
FLUSSAB_AIGER_BIN="flussab/target/release/flussab-aiger"
AAG_BASE_DIR="flussab/flussab-aiger/aag"
AIG_BASE_DIR="flussab/flussab-aiger/aig"
PROCESSED_OR_DIR="$BASE_DIR/process_json/out_process_or_result" 
#ABC
ADD_ABC="add_abc.sh"
ABC_DIR="$BASE_DIR/abc"
ABC_Stable_DIR="$BASE_DIR/abc_stable/abc"
MAP_AIG_SOURCE="choose_net_build/output_file/map_aig.txt"
MAP_AIG_TARGET="$ABC_DIR/dangle_aig/map_aig.txt"
CEC_DIR="$ABC_DIR/cec"
# ABC_SCRIPT="test2.sh"
ABC_SCRIPT="test.sh"
ABC_FILTER_SCRIPT1="filter1.sh"
ABC_FILTER_SCRIPT2="filter2.sh"
ASAP7_LIB="$ABC_DIR/asap7_clean.lib"
DANGLE_AIG="$ABC_DIR/dangle_aig/dangling_all.aig"
PROCESSED_LEN="$ABC_DIR/dangle_aig/processed_len.txt"
REFERENCE_AIG="$ABC_DIR/adder_new.aig"
AIG_2_EGRAPH_BIN="flussab/target/release/aig_2_egraph"
AIG_2_EGRAPH_BIN1="flussab/target/release/aig_2_egraph_a"
E_REWRITER_BIN="$CHOOSE_NET_BUILD_DIR/target/release/e-rewriter"
EXTRACT_SCRIPT="$SCRIPT_DIR/run_extract_parallel.sh" 
EXTRACT_SCRIPT_AREA="$SCRIPT_DIR/run_extract_dag_parallel.sh"  
EXTRACT_SCRIPT_DAG_PROCESS_JSON="$SCRIPT_DIR/run_process_dag_json.sh"  
EXTRACT_OR_REPLACE_BIN="$BASE_DIR/extract_or_replace/target/release/extract_or_replace"  
preprocess_file="$BASE_DIR/pre_process_4_rand_extraction.py"
# ====================== Timer ======================
declare -A RUNTIME=(
    [step1]=0
    [step2]=0
    [step3]=0
    [step4]=0
    [step5]=0
    [step6]=0
    [step7]=0
    [step8]=0
    [total]=0
)
# ====================== Functions ======================
info() { echo -e "${GREEN}[INFO]${RESET} $1"; }
warn() { echo -e "${YELLOW}[WARN]${RESET} $1"; }
error() { echo -e "${RED}[ERROR]${RESET} $1"; exit 1; }

format_time() {
    local seconds=$1
    printf "%02d:%02d:%02d" $((seconds/3600)) $(( (seconds%3600)/60 )) $((seconds%60))
}
# Safe directory switching
safe_cd() {
    if ! cd "$1"; then
        error "error: $1"
    fi
    info "Current working directory: $(pwd)"
}

# ====================== Core Functions ======================
clean_up(){
    safe_cd "$BASE_DIR"
    bash clean1.sh
}

generate_mffc() {
    # Start time
    local start=$(date +%s)
    info "Step 1: Generate MFFC Network"
    safe_cd "$ABC_DIR"

    # Print debug info
    echo "ABC directory switch successful: $(pwd)"
    echo "Check if AIG_FILE exists: ../$AIG_FILE"

    # Check if AIG_FILE exists
    if [[ ! -f "../$AIG_FILE" ]]; then
        error "AIG file doesn't exist: $AIG_FILE"
        return 1
    fi

    # Execute ABC command
    echo "Running ABC command: ./abc -c \"read $AIG_FILE_real; write_mffc\""
    local n_values=(800 85  30 20  15 10) # N parameter values to try in sequence
    local mffc_generated=false # Flag to mark if mffc directory was successfully generated

if [ "$delay" -eq 1 ]; then
    for n in "${n_values[@]}"; do
        mffc_cmd="write_mffc -N $n"
        if [ "$critical_nodes" -eq 1 ]; then
            mffc_cmd="$mffc_cmd -C" # If critical node optimization is enabled, add -C parameter
        fi

        # Execute ABC command
        echo "Trying to run ABC command: ./abc -c \"read $AIG_FILE_real; $mffc_cmd\""
        ./abc -c "read $AIG_FILE_real; $mffc_cmd"

        # Check if mffc directory was generated and is not empty
        if [[ -d "mffc" && "$(ls -A mffc)" ]]; then
            echo "Successfully generated mffc directory with files, using -N $n parameter: $ABC_DIR/mffc"
            mffc_generated=true
            break
        else
            echo "No valid mffc directory generated (directory doesn't exist or is empty), trying lower -N value..."
        fi
    done
    else
        # Get network info and parse AND count (with error handling)
    echo "Getting network statistics..."
    if ! ps_output=$(./abc -c "read $AIG_FILE_real; ps" 2>&1); then
        error "Failed to execute ps command"
        return 1
    fi

    # Show original output for debugging
    echo "----- PS Command Original Output -----"
    echo "$ps_output"
    echo "---------------------------"

    # Enhanced regex to match multiple spaces and file paths
    if [[ "$ps_output" =~ [[:space:]]and[[:space:]]+=[[:space:]]+([0-9]+) ]]; then
        and_count="${BASH_REMATCH[1]}"
        echo "Successfully parsed and count: $and_count"
    else
        error "Cannot parse and count from ps output"
        echo "Last captured output content:"
        echo "$ps_output"
        return 1
    fi

        # Set K based on and count (add default value)
        declare -i K=1  # Set default value
        if (( and_count <= 500 )); then
            K=2
        elif (( and_count >= 500 && and_count < 5000 )); then
            K=3
        # elif (( and_count >= 2000 && and_count < 5000 )); then
        #     K=5
        elif (( and_count >= 5000 && and_count < 10000 )); then
            K=10
        else
            K=15
        fi
        echo "Final calculation parameters: and_count=$and_count → K=$K"

            echo "Setting K = $K based on and count."
mffc_generated=false
local n_values=(85  30 20 15 10 5) # N parameter values to try in sequence
# local n_values=(30 20 15 10 5) # N parameter values to try in sequence
for n in "${n_values[@]}"; do
    echo "Trying to run ABC command: ./abc -c \"read $AIG_FILE_real; write_mffc -N $n\""
    ./abc -c "read $AIG_FILE_real; write_mffc -N $n"

    # Check if mffc directory was generated and is not empty
    if [[ -d "mffc" && "$(ls -A mffc)" ]]; then
        echo "Successfully generated mffc directory with files, using -N $n parameter: $ABC_DIR/mffc"
        mffc_generated=true
        break
    else
        echo "No valid mffc directory generated (directory doesn't exist or is empty), trying lower -N value..."
    fi
done

# If all parameter attempts failed, execute backup command
if [[ "$mffc_generated" == false ]]; then
    echo "All -N parameter attempts failed, trying backup command write_mffc_a..."
    # ./abc -c "read $AIG_FILE_real; write_cpcone -N 30 -M 20 -S 1000 -K 15"
     ./abc -c "read $AIG_FILE_real; write_mffc_a -N 30 -k 2 -L 50 -S 30 "
    fi
        fi

    # Check if mffc directory was generated
    if [[ ! -d "mffc" ]]; then
        error "mffc directory not generated: $ABC_DIR/mffc"
        return 1
    else
        echo "Successfully generated mffc directory: $ABC_DIR/mffc"
    fi

    # Return to original directory
    safe_cd "$BASE_DIR"

    # Define source and target directories (based on BASE_DIR)
    SOURCE_DIR="$BASE_DIR/abc/mffc"
    TARGET_DIR="$BASE_DIR/flussab/aig_2_egraph/mffc"

    # Print debug info
    echo "Check source directory: $SOURCE_DIR"
    echo "Target directory: $TARGET_DIR"
    echo "Exists: $(ls -d $SOURCE_DIR 2>/dev/null)"

    # Check if source directory exists
    if [[ ! -d "$SOURCE_DIR" ]]; then
        error "Source directory doesn't exist: $SOURCE_DIR"
        return 1
    fi

    # If target directory doesn't exist, create it
    if [[ ! -d "$TARGET_DIR" ]]; then
        echo "Target directory doesn't exist, creating: $TARGET_DIR"
        mkdir -p "$TARGET_DIR"
    fi

    # Copy files
    echo "Copying files from $SOURCE_DIR to $TARGET_DIR"
    cp -r "$SOURCE_DIR/"* "$TARGET_DIR/"

    # Check if copy was successful
    if [[ $? -eq 0 ]]; then
        echo "File copy complete!"
    else
        error "File copy failed!"
        return 1
    fi

    # Record runtime
    local end=$(date +%s)
    RUNTIME[step1]=$((end - start))
    info "Step 1 complete, time taken: $(format_time ${RUNTIME[step1]})"
}

aig_to_egraph_parallel() {
    info "Step 2: Parallel processing of all AIG → Egraph conversions in mffc directory"
    local start=$(date +%s)

    # Define input and output directories
    local mffc_dir="$BASE_DIR/abc/mffc"                # mffc directory
    local aag_dir="$AIG_2_EGRAPH_DIR/aag"             # aag output directory
    local map_dir="$AIG_2_EGRAPH_DIR/map"             # map output directory
    local conversion_data_dir="$AIG_2_EGRAPH_DIR/conversion_data" # dynamic JSON output directory
    local rewritten_circuits_dir="$AIG_2_EGRAPH_DIR/rewritten_circuits" # rewritten circuits output directory

    # Ensure output directories exist
    mkdir -p "$aag_dir" "$map_dir" "$conversion_data_dir" "$rewritten_circuits_dir"

    # Collect all .aig files
    local aig_files=($(find "$mffc_dir" -type f -name "*.aig"))
    local total=${#aig_files[@]}
    [ $total -eq 0 ] && warn "No AIG files found" && return
    info "Detected ${CYAN}${total}${RESET} AIG files to convert"

    # Export environment variables
    export iteration_rewrite
    export AIG_2_EGRAPH_BIN AIG_2_EGRAPH_DIR conversion_data_dir rewritten_circuits_dir aag_dir map_dir

    # Define single file processing function
    process_single_aig() {
        local aig_file="$1"

        # Extract filename (without path and extension)
        local basename=$(basename "$aig_file")
        local filename="${basename%.*}"  # Remove extension

        # Dynamically generate JSON output path (matching filename)
        local output_json="$conversion_data_dir/$filename.json"
        local rewritten_circuit_path="$rewritten_circuits_dir/$filename"

        # Ensure JSON and rewritten_circuits directories exist
        mkdir -p "$(dirname "$output_json")"
        mkdir -p "$rewritten_circuit_path"

        # Dynamically generate output_aag and map_file paths
        local output_aag="$aag_dir/$filename.aag"
        local map_file="$map_dir/$filename.txt"

        # Print debug info
        echo "DEBUG: Processing AIG file: $aig_file"
        echo "DEBUG: Output JSON file: $output_json"
        echo "DEBUG: Output AAG file: $output_aag"
        echo "DEBUG: Output MAP file: $map_file"
        echo "DEBUG: Rewritten circuit directory: $rewritten_circuit_path"

        # Execute AIG → Egraph conversion
        if ! "$AIG_2_EGRAPH_BIN" "$aig_file" "$output_json" "$output_aag" "$map_file" "$iteration_rewrite" "$rewritten_circuit_path"; then
            echo "Warning: AIG conversion failed: $aig_file" >&2
            return 1
        fi

        return 0
    }

    # Export necessary functions
    export -f process_single_aig

    # Process AIG files in parallel, with progress bar and error control
    printf "%s\n" "${aig_files[@]}" | parallel \
        --jobs 64 \
        --bar \
        --progress \
        --eta \
        --halt soon,fail=1 \
        "process_single_aig {}"

    # Check parallel task result
    if [ $? -ne 0 ]; then
        error "Error occurred during AIG conversion"
    fi

    local end=$(date +%s)
    RUNTIME[step2]=$((end - start))
    info "Step 2 complete, time taken: $(format_time ${RUNTIME[step2]})"

    # If delay mode is not 1, run preprocessing script
    if [ "$delay" -ne 1 ]; then
        local preprocess_file="$BASE_DIR/pre_process_4_rand_extraction.py"
        echo "Delay mode is not 1, running preprocessing script: $preprocess_file"
        python "$preprocess_file"
    fi
}

parallel_extraction() {
    local start=$(date +%s)
    info "Step 3: Parallel Random Extraction"
    safe_cd "$BASE_DIR"
    
    # Use rewritten_circuits_dir as base_path
    local base_path="$AIG_2_EGRAPH_DIR/rewritten_circuits"  # Dynamically reference rewritten_circuits_dir
    local dirs=($(find "$base_path" -maxdepth 1 -type d -name "[0-9]*" | sort -V))
    local total=${#dirs[@]}
    info "Detected ${CYAN}${total}${RESET} subdirectories to process"

    # Export necessary functions and variables
    export -f process_subdir process_subdir_dag info warn error 
    export EXTRACT_SCRIPT EXTRACT_SCRIPT_AREA  EXTRACT_SCRIPT_DAG_PROCESS_JSON RED GREEN YELLOW RESET 

    # Parallel execution with precise progress display
    printf "%s\n" "${dirs[@]##*/}" | parallel \
        --jobs 200% \
        --bar \
        --tagstring "[{#}/${total}]" \
        --progress \
        --eta \
        --total $total \
        --halt soon,fail=1 \
        "process_subdir {} $num_files_extraction $random_prob_set $delay"
    
    [ $? -eq 0 ] || error "Error during parallel processing"
    
    local end=$(date +%s)
    RUNTIME[step3]=$((end - start))

    info "Step 3 complete, time taken: $(format_time ${RUNTIME[step3]})"
}

process_subdir_dag() {
    local sub_dir="$1"
    local num_files="$2"
    local delay="$3"

    # Debug output
    echo "[DEBUG] Processing sub_dir: $sub_dir"

    # Check if sub_dir is empty
    if [ -z "$sub_dir" ]; then
        echo "[ERROR] sub_dir is empty!" >&2
        exit 1
    fi

    # Set script path
    script_to_run="$EXTRACT_SCRIPT_DAG_PROCESS_JSON"
    optimization_mode="area"

    # Check if script_to_run is empty
    if [ -z "$script_to_run" ]; then
        echo "[ERROR] EXTRACT_SCRIPT_DAG_PROCESS_JSON is not set or empty!" >&2
        exit 1
    fi

    # Debug output
    echo "[DEBUG] Running script: $script_to_run with sub_dir: $sub_dir"

    # Create temp file to pass parameters
    local tmp_input
    tmp_input=$(mktemp) || { echo "[ERROR] Failed to create temp file" >&2; exit 1; }

    cat <<EOF >"$tmp_input"
$num_files
$sub_dir
EOF

    # Execute script
    if ! bash "$script_to_run" <"$tmp_input"; then
        rm -f "$tmp_input"
        echo "[ERROR] Failed to process sub_dir: $sub_dir" >&2
        exit 1
    fi

    rm -f "$tmp_input"
}

process_subdir() {
    local sub_dir="$1"
    local num_files="$2"
    local random_prob="$3"
    local delay="$4"

    # Debug output
    echo "[DEBUG] Processing sub_dir: $sub_dir"

    # Check if sub_dir is empty
    if [ -z "$sub_dir" ]; then
        echo "[ERROR] sub_dir is empty!" >&2
        exit 1
    fi

    # Set optimization mode based on delay
    if [[ "$delay" == "1" ]]; then
        script_to_run="$EXTRACT_SCRIPT"
        optimization_mode="delay"
    else
        # script_to_run="$EXTRACT_SCRIPT_AREA"
        script_to_run="$EXTRACT_SCRIPT"
        optimization_mode="area"
    fi

    # Debug output
    echo "[DEBUG] Running script: $script_to_run with sub_dir: $sub_dir"

    # Create temp file to pass parameters
    local tmp_input
    tmp_input=$(mktemp) || { echo "[ERROR] Failed to create temp file" >&2; exit 1; }

        cat <<EOF >"$tmp_input"
10
$optimization_mode
random-based-faster-bottom-up
$num_files
$random_prob
$sub_dir
EOF

    # Execute script
    if ! bash "$script_to_run" <"$tmp_input"; then
        rm -f "$tmp_input"
        echo "[ERROR] Failed to process sub_dir: $sub_dir" >&2
        exit 1
    fi

    rm -f "$tmp_input"
}

parallel_json_processing() {
    local start=$(date +%s)
    info "Step 4: Parallel JSON Processing"
    safe_cd "$BASE_DIR"

    # Use relative paths
    local processed_dir="$BASE_DIR/process_json/out_process_dag_result"
    local extract_or_replace_dir="$BASE_DIR/extract_or_replace"
    local rewritten_circuit_dir="$BASE_DIR/process_json/out_process_dag_result"

    # Collect all JSON files
    local json_files=($(find "$processed_dir" -type f -name "*.json"))
    local total=${#json_files[@]}
    [ $total -eq 0 ] && warn "No JSON files found" && return
    info "Detected ${CYAN}${total}${RESET} JSON files to process"

    # Process single file function
    process_single_json() {
        local json_file="$1"
        local sub_dir_name=$(basename "$(dirname "$json_file")")
        local filename=$(basename "$json_file")
        local rewritten_file="$BASE_DIR/extract_or_replace/rewritten_circuit/$sub_dir_name/$filename"

        # Switch to extract_or_replace directory
        safe_cd "$extract_or_replace_dir"

        # Call executable to process JSON
        "$EXTRACT_OR_REPLACE_BIN" "$json_file" "$sub_dir_name" 
        cd ..
        # Create temp file matching run_or_replace_parallel.sh input format
        local tmp_input
        tmp_input=$(mktemp) || error "Cannot create temp file"

        # Write parameters in the order required by script (consistent with run_extract.sh style)
        cat <<EOF >"$tmp_input"
${rewritten_file}
5             
area           
faster-bottom-up 
EOF

        #Execute script through input redirection
        if ! bash run_or_replace_parallel.sh < "$tmp_input"; then
            rm -f "$tmp_input"
            error "Replacement processing failed: $json_file"
        fi
        rm -f "$tmp_input"

        # Return to BASE_DIR
        safe_cd "$BASE_DIR"
    }

    # Export necessary functions and variables
    export -f process_single_json info warn error safe_cd
    export EXTRACT_OR_REPLACE_BIN processed_dir BASE_DIR extract_or_replace_dir

    # Parallel processing with progress bar
    printf "%s\n" "${json_files[@]}" | parallel \
        --jobs 100% \
        --bar \
        --progress \
        --eta \
        --halt soon,fail=1 \
        "process_single_json {}"

    [ $? -eq 0 ] || error "JSON file processing failed"
    
    local end=$(date +%s)
    RUNTIME[step4]=$((end - start))
    info "Step 4 complete, time taken: $(format_time ${RUNTIME[step4]})"
}

egraph_aig_conversion() {
    local start=$(date +%s)
    info "Step 5: Parallel AIG Conversion"
    
    # Collect all JSON files to process
    local json_files=($(find "$PROCESSED_OR_DIR" -type f -name "*.json"))
    local total=${#json_files[@]}
    [ $total -eq 0 ] && warn "No JSON files found" && return
    info "Detected ${CYAN}${total}${RESET} JSON files to convert"

    # Process single file function
    process_aig_conversion() {
        local json_file="$1"
        local sub_dir=$(basename $(dirname "$json_file"))
        local file_name=$(basename "$json_file" .json)
        
        # Create output directories
        local aag_dir="$AAG_BASE_DIR/$sub_dir"
        local aig_dir="$AIG_BASE_DIR/$sub_dir"
        mkdir -p "$aag_dir" "$aig_dir" || {
            error "Cannot create output directories: $aag_dir or $aig_dir"
            return 1
        }

        # Build output paths
        local aag_file="$aag_dir/$file_name.aag"
        local aig_file="$aig_dir/$file_name.aig"
        
        # Execute conversion command
        if ! "$FLUSSAB_AIGER_BIN" "$aag_file" "$aig_file" "$json_file"; then
            error "Conversion failed: $json_file"
            return 2
        fi
        
        # Verify output files
        if [[ ! -f "$aag_file" || ! -f "$aig_file" ]]; then
            warn "Output files not generated: $file_name"
            return 3
        fi
        
        return 0
    }

    # Export necessary functions and variables
    export -f process_aig_conversion info warn error
    export FLUSSAB_AIGER_BIN AAG_BASE_DIR AIG_BASE_DIR

    # Parallel processing with progress bar
    printf "%s\n" "${json_files[@]}" | parallel \
        --jobs 100% \
        --bar \
        --progress \
        --eta \
        --halt soon,fail=1 \
        "process_aig_conversion {}"

    [ $? -eq 0 ] || error "Error during AIG conversion process"
    
    local end=$(date +%s)
    RUNTIME[step5]=$((end - start))
    info "Step 5 complete, time taken: $(format_time ${RUNTIME[step5]})"
}

merged_processing() {
    # Step 6: Result Processing
    local step6_start=$(date +%s)
    local TARGET_AIG_DIR="$ABC_DIR/aig"  # Clear target path
    info "Step 6: Result Processing"

    # ==== Original Step 6: Result File Processing ====
    info "---- File Preparation Phase ----"
    mkdir -p "$ABC_DIR" "$CEC_DIR" || error "Directory creation failed"

    # Delete old AIG target directory (if exists)
    if [ -d "$TARGET_AIG_DIR" ]; then
        if ! rm -rf "$TARGET_AIG_DIR"; then
            error "Old directory deletion failed: $TARGET_AIG_DIR"
        fi
    fi

    # Copy AIG file directory to relative path
    if ! cp -r "$AIG_BASE_DIR" "$ABC_DIR/"; then
        error "AIG directory copy failed: $AIG_BASE_DIR → $ABC_DIR/"
    fi

    info "---- ABC Processing Phase ----"
    safe_cd "$ABC_DIR"

    # If filter_by_simulation is enabled, run filter.sh script first
    if [[ "$filter_by_simulation" == "1" ]]; then
    info "filter_by_simulation enabled, running $ABC_FILTER_SCRIPT1..."

    # Run additional ADD_ABC script logic
    bash "$ADD_ABC" "$delay"

    # Execute different logic based on mode value
    if [ "$mode" == "0" ]; then
        # mode = 0: Run all three scripts
        if ! bash "$ABC_FILTER_SCRIPT1" "$num_save0" "$delay"; then
            error "Filter script execution failed: $ABC_FILTER_SCRIPT1"
        fi

        if ! bash "$ABC_FILTER_SCRIPT2" "$num_save1" "$delay"; then
            error "Filter script execution failed: $ABC_FILTER_SCRIPT2"
        fi

        info "Running test.sh..."
        if ! bash "$ABC_SCRIPT" "$AIG_FILE_real" "$num_save1" "$delay"; then
            error "test.sh execution failed"
        fi

    elif [ "$mode" == "1" ]; then
        # mode = 1: Run ABC_FILTER_SCRIPT1 and ABC_SCRIPT
        if ! bash "$ABC_FILTER_SCRIPT1" "$num_save1" "$delay"; then
            error "Filter script execution failed: $ABC_FILTER_SCRIPT1"
        fi

        if ! bash "$ABC_SCRIPT" "$AIG_FILE_real" "$num_save1" "$delay"; then
            error "test.sh execution failed"
        fi

    elif [ "$mode" == "2" ]; then
        # mode = 2: Run ABC_FILTER_SCRIPT2 and ABC_SCRIPT
        if ! bash "$ABC_FILTER_SCRIPT2" "$num_save1" "$delay"; then
            error "Filter script execution failed: $ABC_FILTER_SCRIPT2"
        fi

        if ! bash "$ABC_SCRIPT" "$AIG_FILE_real" "$num_save1" "$delay"; then
            error "test.sh execution failed"
        fi
    fi

else
    # If filter_by_simulation is not enabled, run script directly
    info "filter_by_simulation not enabled, running $ABC_SCRIPT directly..."

    if ! bash "$ABC_SCRIPT" "$AIG_FILE_real" "$num_files_save" "$delay"; then
        error "test.sh execution failed"
    fi
fi
    # End step6 timing
    info "Step 6: Result processing complete, time taken: $(format_time ${RUNTIME[step6]})"

    # Execute ABC synthesis command
    local abc_cmd="&r -r $DANGLE_AIG $MAP_AIG_TARGET $PROCESSED_LEN;&ps;&write $CEC_DIR/b.aig"
    info "Executing ABC synthesis..."
    if ! ./abc -c "$abc_cmd"; then
        safe_cd "$ABC_Stable_DIR"
        ./abc -c "read $AIG_FILE_real1;if -g -K 6 -C 8; if -g -K 6 -C 8;ps;read_lib $ASAP7_LIB;map -v;ps;topo;upsize;dnsize;stime;" || error "Final synthesis command execution failed"
      
    fi

    # Step 7: MAP Processing
    local step6_end=$(date +%s)
    RUNTIME[step6]=$((step6_end - step6_start))
    local step7_start=$(date +%s)

    info "Step 7: MAP Processing"
    safe_cd "$ABC_Stable_DIR"

    # Execute subsequent synthesis commands
     if [[ "$delay" == "1" ]]; then
        info "Using delay optimization mode"
        ./abc -c "&r $CEC_DIR/b.aig;&put;ps;read_lib $ASAP7_LIB;map -v;ps;topo;upsize;dnsize;stime;st;write $CEC_DIR/a.aig" || error "ABC subsequent command execution failed"
        local step7_end=$(date +%s)
        local step8_start=$(date +%s)
        ./abc -c "read $AIG_FILE_real1;if -g -K 6 -C 8; if -g -K 6 -C 8;dch;ps;read_lib $ASAP7_LIB;map -v;ps;topo;upsize;dnsize;stime;" || error "Final synthesis command execution failed"
        local step8_end=$(date +%s)
    else
        info "Using area optimization mode"
        ./abc -c "&r $CEC_DIR/b.aig;&put;ps;read_lib $ASAP7_LIB;map -a -v;ps;topo;upsize;dnsize;stime;st;write $CEC_DIR/a.aig" || error "ABC subsequent command execution failed"
        local step7_end=$(date +%s)
        local step8_start=$(date +%s)
        ./abc -c "read $AIG_FILE_real1;compress2rs;compress2rs;dch;ps;read_lib $ASAP7_LIB;map -a -v;ps;topo;upsize;dnsize;stime;" || error "Final synthesis command execution failed"
        local step8_end=$(date +%s)
    fi

    #Execute equivalence verification
    info "Executing equivalence verification..."
    cec_output=$(./abc -c "cec $CEC_DIR/a.aig $AIG_FILE_real")
    echo "$cec_output"
    if echo "$cec_output" | grep -q "Networks are equivalent"; then
        info "${GREEN}Verification successful: Circuits are equivalent${RESET}"
    else
        warn "${YELLOW}Verification failed: Circuits are not equivalent${RESET}"
    fi

    RUNTIME[step7]=$((step7_end - step7_start))
    RUNTIME[step8]=$((step8_end - step8_start))
}

# ====================== Example Call ======================
main() {
    local total_start=$(date +%s)
    local start_time_str=$(date -d @$total_start '+%Y-%m-%d %H:%M:%S')
    
    echo -e "${CYAN}======= Process Started =======${RESET}"
    echo -e "Start time:         ${start_time_str}"
    clean_up
    generate_mffc
    aig_to_egraph_parallel
    parallel_extraction
    parallel_json_processing
    egraph_aig_conversion
    merged_processing
    RUNTIME[total]=$(( $(date +%s) - total_start ))
    
    echo -e "\n${CYAN}======= Runtime Statistics111 =======${RESET}"

echo -e "Start time:         ${start_time_str}"
echo -e "Step 1: AIG Conversion    $(format_time ${RUNTIME[step1]})"
echo -e "Step 2: Network Construction   $(format_time ${RUNTIME[step2]})"
echo -e "Step 3: Parallel Extraction   $(format_time ${RUNTIME[step3]})"
echo -e "Step 4: JSON Processing   $(format_time ${RUNTIME[step4]})"
echo -e "Step 5: AIG Generation    $(format_time ${RUNTIME[step5]})"
echo -e "Step 6: Result Processing   $(format_time ${RUNTIME[step6]})"
echo -e "Step 7: Mapping    $(format_time ${RUNTIME[step7]})"
steps_total_time=$((RUNTIME[step1] + RUNTIME[step2] + RUNTIME[step3] + RUNTIME[step4] + RUNTIME[step5] + RUNTIME[step6] + RUNTIME[step7]))
echo -e "-------------------------------"
echo -e "Steps 1-7 Total Time:   $(format_time ${steps_total_time})"
echo -e "-------------------------------"
echo -e "dch runtime      $(format_time ${RUNTIME[step8]})"

# Print total runtime and completion time
echo -e "Total runtime:        $(format_time ${RUNTIME[total]})"
echo -e "Completion time:          $(date '+%Y-%m-%d %H:%M:%S')"
}

# Execute main function
main "$@"