#!/bin/bash

# Usage: ./preprocess_mffc.sh <mode>
# mode=1 uses if -g; otherwise uses compress2rs twice

BASE_DIR="$(cd "$(dirname "$0")" && pwd)"
MFFC_DIR="$BASE_DIR/mffc"
AIG_DIR="$BASE_DIR/aig"
ABC_CMD="./abc"
MAX_JOBS=96
mode="$1"

if [ -z "$mode" ]; then
    echo "Error: Please provide mode parameter (1 for if -g, otherwise compress2rs)"
    exit 1
fi

if ! [[ "$mode" =~ ^[0-9]+$ ]]; then
    echo "Error: mode parameter must be integer"
    exit 1
fi

preprocess_aig_files() {
    local aig_file="$1"
    local mode="$2"
    if [ -z "$aig_file" ]; then
        echo "Error: file path is empty"
        return
    fi
    if [ "$mode" -ne 1 ]; then
        "$ABC_CMD" -c "read $aig_file; compress2rs; compress2rs; write_aiger -s $aig_file;"
    else
        "$ABC_CMD" -c "read $aig_file; if -g; write_aiger -s $aig_file;"
    fi
}

export -f preprocess_aig_files
export ABC_CMD

find "$AIG_DIR" -type f -name "*.aig" | parallel --jobs "$MAX_JOBS" preprocess_aig_files {} "$mode"

process_one_file() {
    local file_path="$1"
    local mode="$2"
    local filename foldername target_dir target_file target_file1
    filename=$(basename "$file_path")
    foldername="${filename%%.*}"
    target_dir="$AIG_DIR/$foldername"
    target_file="$target_dir/abc.aig"
    target_file1="$target_dir/abc1.aig"
    mkdir -p "$target_dir"
    if [ "$mode" -ne 1 ]; then
        "$ABC_CMD" -c "read $file_path; logic;mfs -a;mfs -a;st; write_aiger -s $target_file;"
        "$ABC_CMD" -c "read $file_path; compress2rs; compress2rs; write_aiger -s $target_file1;"
    else
        "$ABC_CMD" -c "read $file_path; if -g; write_aiger -s $target_file;"
    fi
}

export -f process_one_file
export AIG_DIR ABC_CMD

mffc_list=($(find "$MFFC_DIR" -maxdepth 1 -type f -name "*.aig" | sort -V))
total_mffc_files=${#mffc_list[@]}

if [ "$total_mffc_files" -eq 0 ]; then
    echo "Error: No .aig file found in $MFFC_DIR"
    exit 1
fi

parallel --jobs "$MAX_JOBS" process_one_file {} "$mode" ::: "${mffc_list[@]}"