#!/bin/bash

RED="\e[31m"
GREEN="\e[32m"
YELLOW="\e[1;33m"
RESET="\e[0m"

# 提示用户警告信息
echo -e "${RED}WARNING: This script will permanently delete all files and subfolders in the specified directories. Proceed with caution!${RESET}"

# 定义需要清理的目录
directories=(
  "extraction-gym/output_log"
  "extraction-gym/random_result"
  "extraction-gym/random_out_dag_json"
  "extraction-gym/out_dag_json"
  "extract_or_replace/output_file"
  "extract_or_replace/rewritten_circuit"
  "process_json/out_process_dag_result"
  "process_json/out_process_or_result"
  "flussab/flussab-aiger/aig"
  "flussab/flussab-aiger/aig"
  "flussab/aig_2_egraph/rewritten_circuits"
  "flussab/aig_2_egraph/map"
  "flussab/aig_2_egraph/aag"
  "flussab/aig_2_egraph/mffc"
  "abc/aig"
  "choose_net_build/root_md_egraph"
  "abc/dangle_aig"
  "abc/cec"
  "choose_net_build/output_cp_file"
)

# 遍历目录并清理
for dir in "${directories[@]}"; do
  if [ -d "$dir" ]; then
    echo -e "${YELLOW}Cleaning directory: $dir${RESET}"
    rm -rf "$dir"/* 2>/dev/null
    rm -rf "$dir"/.* 2>/dev/null  # 删除隐藏文件
    echo -e "${GREEN}Deleted all contents of $dir.${RESET}"
  else
    echo -e "${YELLOW}Directory $dir does not exist. Skipping.${RESET}"
  fi
done

# 提示清理完成
echo -e "${GREEN}Cleaning complete.${RESET}"