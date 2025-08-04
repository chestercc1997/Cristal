#!/bin/bash

BASE_DIR="$(cd "$(dirname "$0")" && pwd)"
MFFC_DIR="$BASE_DIR/mffc"
OUTPUT_AIG="$BASE_DIR/dangle_aig/dangling_all.aig"
ABC_CMD="./abc"

if [ -z "$1" ]; then
    echo "Error: Please provide input file path as the first argument"
    exit 1
fi

INPUT_AIG="$1"

if [ -z "$2" ]; then
    echo "Error: Please provide num_files_extraction as the second argument"
    exit 1
fi

num_files_extraction="$2"
if ! [[ "$num_files_extraction" =~ ^[1-9][0-9]*$ ]]; then
    echo "Error: num_files_extraction must be a positive integer"
    exit 1
fi

mode="$3"
if [ -z "$mode" ]; then
    echo "Error: Please provide mode as the third argument"
    exit 1
fi

mffc_list=($(find "$MFFC_DIR" -maxdepth 1 -type f -name "*.aig" | sort -V))
total_mffc_files=${#mffc_list[@]}

if [ $total_mffc_files -eq 0 ]; then
    echo "Error: No .aig file found in $MFFC_DIR"
    exit 1
fi

start_time=$(date +%s)

{
    echo "read $INPUT_AIG; aig_store;"
    count=0
    for file_path in "${mffc_list[@]}"; do
        if [ "$mode" -ne 1 ]; then
            echo "read $file_path; compress2rs;aig_store;"
        else
            echo "read $file_path;if -g;aig_store;"
        fi
        count=$((count + 1))
    done
    echo "appendall -s $num_files_extraction;"
    echo "write_aiger -s $OUTPUT_AIG;"
} | $ABC_CMD

end_time=$(date +%s)
runtime=$((end_time - start_time))

echo "================ Statistics ================"
echo "Number of mffc files: $total_mffc_files"
echo "Output file: $OUTPUT_AIG"
echo "Total runtime: ${runtime} seconds"