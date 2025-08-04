#!/bin/bash

# ====================== 配置区块 ======================
BASE_DIR="$(cd "$(dirname "$0")" && pwd)"
MFFC_DIR="$BASE_DIR/mffc"
AIG_DIR="$BASE_DIR/aig"
ABC_CMD="./abc"
MAX_JOBS=96  # 可根据需要调整并行任务数，用于控制同时处理的 MFFC 文件个数

# ====================== 核心处理逻辑 ======================
process_aig_files() {
    local mffc_file="$1"
    local mffc_basename
    mffc_basename=$(basename "$mffc_file" .aig)
    local aig_subdir="$AIG_DIR/$mffc_basename"

    # 如果子目录不存在，跳过
    [[ ! -d "$aig_subdir" ]] && return

    declare -A lev_map=()
    declare -A mismatch_map=()
    declare -A score_map=()
    local all_files=()

    # 提取子文件夹内所有文件的 lev 和 mismatch 数据
    for aig_file in "$aig_subdir"/*.aig; do
        # 执行 abc 命令，提取信息
        local output
        output=$("$ABC_CMD" -c "&r $aig_file; &reveng $mffc_file" 2>/dev/null)

        # 调用 extract_metrics 提取数据
        local metrics
        metrics=$(extract_metrics "$output")
        
        # 解析指标数据
        local mismatch_pct
        mismatch_pct=$(echo "$metrics" | cut -d',' -f1)
        local flat_lev
        flat_lev=$(echo "$metrics" | cut -d',' -f2)

        # 验证数据合法性
        if [[ -z "$flat_lev" || "$flat_lev" == "-1" || -z "$mismatch_pct" || "$mismatch_pct" == "-1" ]]; then
            continue
        fi

        # 保存数据到数组和映射
        all_files+=("$aig_file")
        lev_map["$aig_file"]=$flat_lev
        mismatch_map["$aig_file"]=$mismatch_pct
    done

    # 如果子文件夹内没有有效文件，跳过
    [[ ${#all_files[@]} -eq 0 ]] && return

    # 初始化所有文件的得分为0
    for file in "${all_files[@]}"; do
        score_map["$file"]=0
    done

    # 计算得分
    calculate_scores all_files[@] lev_map mismatch_map score_map

    # 找出得分最高的文件
    local best_file=""
    local best_score=-1
    for file in "${all_files[@]}"; do
        local score=${score_map["$file"]}
        if [[ -n "$score" && "$score" != "-1" ]]; then
            if (( $(echo "$score > $best_score" | bc -l) )); then
                best_score=$score
                best_file="$file"
            fi
        fi
    done

    # 删除子文件夹内其他文件，仅保留得分最高的文件
    [[ -n "$best_file" ]] && cleanup_files "$aig_subdir" "$best_file"
}

# ====================== 辅助函数 ======================
# 提取 Mismatch Percentage 和 Flat Lev
extract_metrics() {
    echo "$1" | awk '
        BEGIN {
            mismatch_pct = -1
            flat_lev = -1
        }
        /Flat:.*lev =[[:space:]]*[0-9]+/ {
            match($0, /lev =[[:space:]]*([0-9]+)/, arr)
            if (arr[1] != "") {
                flat_lev = arr[1]
            }
        }
        /\[DATA\] matches=/ {
            match($0, /mismatches=[0-9]+\(([0-9.]+)%\)/, arr)
            if (arr[1] != "") {
                mismatch_pct = arr[1]
            }
        }
        END {
            print mismatch_pct "," flat_lev
        }'
}

# 计算文件得分
calculate_scores() {
    local files_ref=$1
    local -n lev_ref=$2
    local -n mismatch_ref=$3
    local -n score_ref=$4

    # 从引用中获取文件列表
    local files=("${!files_ref}")

    # 创建用于排序的临时数组 - lev
    local -a lev_data=()
    for file in "${files[@]}"; do
        local lev=${lev_ref["$file"]}
        [[ "$lev" =~ ^[0-9]+$ ]] && lev_data+=("$file:$lev")
    done

    # 排序 lev 数据 (按数值升序)
    IFS=$'\n'
    lev_data=($(printf "%s\n" "${lev_data[@]}" | sort -t: -k2 -n))
    unset IFS

    # 赋予 lev 分数 (70% 权重)
    local lev_count=${#lev_data[@]}
    if [[ $lev_count -gt 0 ]]; then
        for i in "${!lev_data[@]}"; do
            local entry="${lev_data[$i]}"
            local file="${entry%%:*}"
            local rank=$((i + 1))
            local lev_score
            lev_score=$(echo "scale=4; ($lev_count - $rank + 1) / $lev_count * 1" | bc)
            score_ref["$file"]=$(echo "scale=4; ${score_ref["$file"]} + $lev_score *2" | bc)
        done
    fi

    # 创建用于排序的临时数组 - mismatch
    local -a mismatch_data=()
    for file in "${files[@]}"; do
        local mismatch=${mismatch_ref["$file"]}
        [[ "$mismatch" =~ ^[0-9]+(\.[0-9]+)?$ ]] && mismatch_data+=("$file:$mismatch")
    done

    # 排序 mismatch 数据 (按数值升序)
    IFS=$'\n'
    mismatch_data=($(printf "%s\n" "${mismatch_data[@]}" | sort -t: -k2 -n))
    unset IFS

    # 赋予 mismatch 分数 (30% 权重)
    local mismatch_count=${#mismatch_data[@]}
    if [[ $mismatch_count -gt 0 ]]; then
        for i in "${!mismatch_data[@]}"; do
            local entry="${mismatch_data[$i]}"
            local file="${entry%%:*}"
            local rank=$((i + 1))
            local mismatch_score
            mismatch_score=$(echo "scale=4; ($mismatch_count - $rank + 1) / $mismatch_count * 1" | bc)
            score_ref["$file"]=$(echo "scale=4; ${score_ref["$file"]} + $mismatch_score" | bc)
        done
    fi
}

# 删除子文件夹内其他文件，仅保留得分最高的文件
cleanup_files() {
    local dir="$1" best="$2"

    for file in "$dir"/*.aig; do
        [[ "$file" != "$best" ]] && rm -f "$file"
    done

    # echo "保留文件：$best"
}

# ====================== 主执行流程 ======================
main() {
    # 导出需要在并行进程中使用的函数和变量
    export ABC_CMD AIG_DIR MFFC_DIR
    export -f process_aig_files extract_metrics calculate_scores cleanup_files

    # 使用 GNU Parallel 针对每个 MFFC 文件进行并行处理
    parallel --jobs "$MAX_JOBS" process_aig_files {} ::: "$MFFC_DIR"/*.aig

    # echo "所有操作完成！"
}

main "$@"