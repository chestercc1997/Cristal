#!/bin/bash

BASE_DIR="$(cd "$(dirname "$0")" && pwd)"
BASE_PATH="$BASE_DIR/aig"
OUTPUT_AIG="$BASE_DIR/dangle_aig/dangling_all.aig"
ABC_CMD="./abc"

if [ -z "$1" ]; then
    echo "Error: Please provide the input file path as the first argument"
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

dir_list=($(find "$BASE_PATH" -maxdepth 1 -type d -name "[0-9]*" | sort -V))
total_dirs=${#dir_list[@]}

if [ $total_dirs -eq 0 ]; then
    echo "Error: No subdirectory found under $BASE_PATH"
    exit 1
fi

start_time=$(date +%s)

{
    echo "read $INPUT_AIG; aig_store;"
    count=0
    for dir_path in "${dir_list[@]}"; do
        dir_num=$(basename "$dir_path")
        file_list=($(find "$dir_path" -maxdepth 1 -type f -name "*.aig" | sort -V))
        if [ ${#file_list[@]} -eq 0 ]; then
            echo "Warning: No AIG file in $dir_num" >&2
            continue
        fi
        for file_path in "${file_list[@]}"; do
            if [ "$mode" -ne 1 ]; then
                echo "read $file_path; compress2rs;compress2rs; aig_store;"
            else
                echo "read $file_path; aig_store;"
            fi
            count=$((count + 1))
        done
    done
    echo "appendall -s $num_files_extraction;"
    echo "write_aiger -s $OUTPUT_AIG;"
} | $ABC_CMD

end_time=$(date +%s)
runtime=$((end_time - start_time))

echo "================ Statistics ================"
echo "Number of directories: $total_dirs"
total_files=$(find "$BASE_PATH" -type f -name "*.aig" | wc -l)
echo "Total files processed: $total_files"
echo "Output file: $OUTPUT_AIG"
echo "Total runtime: ${runtime} seconds"