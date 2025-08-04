#!/bin/bash

# INPUT_DIR="/export2/cchen099/choice_graphlearning/benchmarks/benchmark_new"
INPUT_DIR="/data/cchen/choice_revisit/benchmarks/epfl/benchmarks/exp_aig_new"
# OUTPUT_DIR="/export2/cchen099/choice_graphlearning/benchmarks/delay"
OUTPUT_DIR="/data/cchen/choice_revisit/benchmarks/epfl/benchmarks/exp_delay"
ABC_TOOL="../abc/abc"

if [[ ! -x "$ABC_TOOL" ]]; then
    echo "ABC tool not found or not executable at $ABC_TOOL. Please check the path."
    exit 1
fi

mkdir -p "$OUTPUT_DIR"

for file in "$INPUT_DIR"/*.aig; do
    if [[ -f "$file" ]]; then
        filename=$(basename -- "$file")
        filename_no_ext="${filename%.*}"
        output_file="$OUTPUT_DIR/${filename_no_ext}.aig"
        
        echo "Processing $file..."
        
        $ABC_TOOL -c "read $file; if -g -K 6 -C 8; if -g -K 6 -C 8; dch;st; write $output_file"
        # $ABC_TOOL -c "read $file; dch;st; write $output_file"
        
        if [[ $? -eq 0 ]]; then
            echo "Successfully processed $file -> $output_file"
        else
            echo "Failed to process $file"
        fi
    fi
done

echo "All files processed."