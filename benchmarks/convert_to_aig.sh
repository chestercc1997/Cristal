#!/bin/bash

# 输入文件目录
INPUT_DIR="./benchmarks/converted_circuit_strash/LGSynth91"
# 输出文件目录
OUTPUT_DIR="./benchmarks/LGSynth91/benchmarks/aig"
# ABC 工具路径
ABC_TOOL="../../abc/abc"

# 检查 ABC 工具是否存在
if [[ ! -x "$ABC_TOOL" ]]; then
    echo "ABC tool not found or not executable at $ABC_TOOL. Please check the path."
    exit 1
fi

# 如果输出目录不存在，则创建
mkdir -p "$OUTPUT_DIR"

# 遍历输入目录中的所有 .eqn 文件
for file in "$INPUT_DIR"/*.eqn; do
    if [[ -f "$file" ]]; then
        # 获取文件名（不含路径）
        filename=$(basename -- "$file")
        # 去掉文件后缀
        filename_no_ext="${filename%.*}"
        # 输出文件路径
        output_file="$OUTPUT_DIR/${filename_no_ext}.aig"
        
        echo "Processing $file..."
        
        # 使用 ABC 工具运行命令并保存输出文件
        $ABC_TOOL -c "read_eqn $file; st; write_aiger -s $output_file"
        
        if [[ $? -eq 0 ]]; then
            echo "Successfully processed $file -> $output_file"
        else
            echo "Failed to process $file"
        fi
    fi
done

echo "All files processed."