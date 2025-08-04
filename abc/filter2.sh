#!/bin/bash

# ====================== 配置区块 ======================
BASE_DIR="$(cd "$(dirname "$0")" && pwd)"
MFFC_DIR="$BASE_DIR/mffc"
AIG_DIR="$BASE_DIR/aig"
ABC_CMD="../abc_stable/abc/abc"
MAX_JOBS=96  # 并行任务数，可根据环境调整

# 参数1：保留的 .aig 文件数量 K
K=${1:-1}
# 参数2：Mode，1 表示使用delay排序；其他值表示使用area排序
MODE=${2:-1}

echo "Starting script with K=$K, MODE=$MODE"
echo "BASE_DIR=$BASE_DIR"
echo "MFFC_DIR=$MFFC_DIR"
echo "AIG_DIR=$AIG_DIR"

# ====================== 核心处理逻辑 ======================
process_aig_files() {
    local mffc_file="$1"
    local mffc_basename
    mffc_basename=$(basename "$mffc_file" .aig)
    local aig_subdir="$AIG_DIR/$mffc_basename"

    echo "Processing $mffc_file"
    echo "AIG subdir: $aig_subdir"

    # 如果子目录不存在，跳过
    if [[ ! -d "$aig_subdir" ]]; then
        echo "Subdir $aig_subdir does not exist, skipping"
        return
    fi

    declare -A area_map=()
    declare -A delay_map=()
    local all_files=()

    # 检查子目录中的文件数量
    local file_count=$(ls -1 "$aig_subdir"/*.aig 2>/dev/null | wc -l)
    echo "Found $file_count .aig files in $aig_subdir"

    # 遍历所有 .aig 文件
    for aig_file in "$aig_subdir"/*.aig; do
        echo "Processing file $aig_file"
        
        # 调用 ABC 提取数据
        local output metrics area delay
        echo "Running ABC command: $ABC_CMD -c \"read asap7_clean.lib;read $aig_file;map;ps\""
        output=$("$ABC_CMD" -c "read asap7_clean.lib;read $aig_file;map;ps" 2>/dev/null)
        metrics=$(extract_metrics "$output")
        echo "Extracted metrics: $metrics"
        IFS=, read -r area delay <<< "$metrics"

        # 如果指标无效，设置默认值（这些默认值会导致得分较低）
        [[ -z "$area" || "$area" == "-1" ]] && area="999.99"
        [[ -z "$delay" || "$delay" == "-1" ]] && delay="999.99"
        
        all_files+=("$aig_file")
        area_map["$aig_file"]=$area
        delay_map["$aig_file"]=$delay
        
        echo "Using metrics: area=$area, delay=$delay"
    done

    # 如果没有文件，跳过
    if [[ ${#all_files[@]} -eq 0 ]]; then
        echo "No files found, skipping"
        return
    fi

    echo "Total files: ${#all_files[@]}"

    # 初始化得分映射
    declare -A score_map=()
    local total_files=${#all_files[@]}

    # 根据MODE选择排序方式
    if [[ "$MODE" -eq 1 ]]; then
        # 使用delay排序（小的得分高）
        local score=$total_files
        # 值到文件的反向映射
        declare -A val2files=()
        for f in "${!delay_map[@]}"; do
            val2files["${delay_map[$f]}"]+="$f "
        done
        
        # 排序值列表 (升序)
        local vals=( $(printf "%s\n" "${!val2files[@]}" | sort -n) )
        
        # 依次打分
        for v in "${vals[@]}"; do
            echo "Assigning score $score to files with delay=$v"
            for f in ${val2files[$v]}; do
                score_map["$f"]=$score
                echo "  File: $(basename "$f"), delay=$v, score=$score"
            done
            ((score--))
        done
    else
        # 使用area排序（小的得分高）
        local score=$total_files
        # 值到文件的反向映射
        declare -A val2files=()
        for f in "${!area_map[@]}"; do
            val2files["${area_map[$f]}"]+="$f "
        done
        
        # 排序值列表 (升序)
        local vals=( $(printf "%s\n" "${!val2files[@]}" | sort -n) )
        
        # 依次打分
        for v in "${vals[@]}"; do
            echo "Assigning score $score to files with area=$v"
            for f in ${val2files[$v]}; do
                score_map["$f"]=$score
                echo "  File: $(basename "$f"), area=$v, score=$score"
            done
            ((score--))
        done
    fi

    # 按得分排序，保留前 K，其它删除
    echo "Sorting files by score"
    local sorted=( $(for f in "${all_files[@]}"; do
        printf "%s:%s\n" "${score_map[$f]}" "$f"
    done | sort -t: -k1 -rn | cut -d: -f2) )

    echo "Sorted list (top $K to keep):"
    for i in "${!sorted[@]}"; do
        if [[ $i -lt $K ]]; then
            if [[ "$MODE" -eq 1 ]]; then
                echo "  KEEP: ${sorted[$i]} (delay: ${delay_map[${sorted[$i]}]}, score: ${score_map[${sorted[$i]}]})"
            else
                echo "  KEEP: ${sorted[$i]} (area: ${area_map[${sorted[$i]}]}, score: ${score_map[${sorted[$i]}]})"
            fi
        else
            if [[ "$MODE" -eq 1 ]]; then
                echo "  REMOVE: ${sorted[$i]} (delay: ${delay_map[${sorted[$i]}]}, score: ${score_map[${sorted[$i]}]})"
            else
                echo "  REMOVE: ${sorted[$i]} (area: ${area_map[${sorted[$i]}]}, score: ${score_map[${sorted[$i]}]})"
            fi
        fi
    done

    echo "Files to be removed:"
    for f in "${all_files[@]}"; do
        if ! [[ " ${sorted[@]:0:$K} " =~ " $f " ]]; then
            echo "Will remove: $f"
            rm -f "$f"
            # 检查文件是否被删除
            if [[ -f "$f" ]]; then
                echo "ERROR: File still exists after removal attempt!"
            else
                echo "SUCCESS: File was removed"
            fi
        fi
    done
}

# ====================== 辅助函数 ======================
extract_metrics() {
    # 从输出中提取area和delay，修复为正确提取纯数值
    local area=$(echo "$1" | grep -o "area =[0-9.]*" | sed 's/area =//g')
    local delay=$(echo "$1" | grep -o "delay =[0-9.]*" | sed 's/delay =//g')
    
    # 如果未找到，返回-1
    [[ -z "$area" ]] && area="-1"
    [[ -z "$delay" ]] && delay="-1"
    
    echo "$area,$delay"
}

# ====================== 主执行流程 ======================
main() {
    export ABC_CMD AIG_DIR MFFC_DIR K MODE
    export -f process_aig_files extract_metrics

    echo "Starting parallel processing with K=$K MODE=$MODE"
    echo "Looking for files in $MFFC_DIR/*.aig"
    local mffc_count=$(ls -1 "$MFFC_DIR"/*.aig 2>/dev/null | wc -l)
    echo "Found $mffc_count MFFC files to process"

    parallel --jobs "$MAX_JOBS" process_aig_files {} ::: "$MFFC_DIR"/*.aig
    
    echo "Processing complete!"
}

main "$@"