#!/bin/bash

cd "$(dirname "$0")"）
INPUT_DIR="./exp_aig"
OUTPUT_DIR="./exp_area"
ABC_TOOL="../../../abc/abc"

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
        "$ABC_TOOL" -c "read $file; compress2rs; compress2rs; dch; st; write $output_file"
        
        if [[ $? -eq 0 ]]; then
            echo "Successfully processed $file -> $output_file"
        else
            echo "Failed to process $file"
        fi
    fi
done

echo "All files processed."