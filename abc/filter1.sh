#!/bin/bash

BASE_DIR="$(cd "$(dirname "$0")" && pwd)"
MFFC_DIR="$BASE_DIR/mffc"
AIG_DIR="$BASE_DIR/aig"
ABC_CMD="./abc"
MAX_JOBS=64

K=${1:-1}
MODE=${2:-1}

process_aig_files() {
    local mffc_file="$1"
    local mffc_basename
    mffc_basename=$(basename "$mffc_file" .aig)
    local aig_subdir="$AIG_DIR/$mffc_basename"

    [[ ! -d "$aig_subdir" ]] && return

    declare -A mismatch_map=()
    declare -A depthcorr_map=()
    declare -A lev_map=()
    declare -A and_map=()
    local all_files=()

    for aig_file in "$aig_subdir"/*.aig; do
        local output metrics mismatch_pct depth_corr lev and_count
        output=$("$ABC_CMD" -c "&r $mffc_file; &reveng $aig_file" 2>/dev/null)
        metrics=$(extract_metrics "$output")
        IFS=, read -r mismatch_pct depth_corr lev and_count <<< "$metrics"
        if [[ -z "$mismatch_pct" || "$mismatch_pct" == "-1" || \
              -z "$depth_corr"   || "$depth_corr"   == "-1" || \
              -z "$lev"          || "$lev"          == "-1" || \
              -z "$and_count"    || "$and_count"    == "-1" ]]; then
            continue
        fi
        all_files+=("$aig_file")
        mismatch_map["$aig_file"]=$mismatch_pct
        depthcorr_map["$aig_file"]=$depth_corr
        lev_map["$aig_file"]=$lev
        and_map["$aig_file"]=$and_count
    done

    [[ ${#all_files[@]} -eq 0 ]] && return

    declare -A lev_score_map=()
    declare -A mismatch_score_map=()
    declare -A depth_score_map=()
    declare -A and_score_map=()
    declare -A final_score_map=()

    local total_files=${#all_files[@]}

    assign_scores() {
        local map_name_vals=$1
        local map_name_scores=$2
        local ascending=$3

        local -n in_map="$map_name_vals"
        local -n out_map="$map_name_scores"

        declare -A val2files=()
        for f in "${!in_map[@]}"; do
            val2files["${in_map[$f]}"]+="$f "
        done

        local vals=( $(printf "%s\n" "${!val2files[@]}" | sort -n) )
        [[ $ascending -eq 0 ]] && vals=( $(printf "%s\n" "${vals[@]}" | tac) )

        local score=$total_files
        for v in "${vals[@]}"; do
            for f in ${val2files[$v]}; do
                out_map["$f"]=$score
            done
            ((score--))
        done
    }

    assign_scores lev_map           lev_score_map      1
    assign_scores mismatch_map      mismatch_score_map 0
    assign_scores depthcorr_map     depth_score_map    0
    if [[ "$MODE" -eq 1 ]]; then
        assign_scores and_map       and_score_map      0
    else
        assign_scores and_map       and_score_map      1
    fi

    for f in "${all_files[@]}"; do
        local ms=${mismatch_score_map["$f"]}
        local ds=${depth_score_map["$f"]}
        local as=${and_score_map["$f"]}
        local ls=${lev_score_map["$f"]}
        local struct_score
        struct_score=$(echo "scale=4; (6*$ms + 2*$ds + 2*$as)/3" | bc)
        local final_score
        if [[ "$MODE" -eq 1 ]]; then
            final_score=$(echo "scale=4; $ls + $struct_score" | bc)
        else
            final_score=$(echo "scale=4; $as + $struct_score" | bc)
        fi
        final_score_map["$f"]=$final_score
    done

    local sorted=( $(for f in "${all_files[@]}"; do
        printf "%s:%s\n" "${final_score_map[$f]}" "$f"
    done | sort -t: -k1 -rn | cut -d: -f2) )

    for f in "${all_files[@]}"; do
        if ! [[ " ${sorted[@]:0:$K} " =~ " $f " ]]; then
            rm -f "$f"
        fi
    done
}

extract_metrics() {
    echo "$1" | awk '
        BEGIN { mismatch_pct=-1; depth_corr=-1; lev=-1; and_count=-1 }
        /\[DATA\] matches=/ {
            match($0,/mismatches=[0-9]+\(([0-9.]+)%\)/,a)
            if (a[1]!="") mismatch_pct=a[1]
        }
        /\[STRUCTURE\]/ {
            match($0,/DepthCorr=([0-9.]+)/,a)
            if (a[1]!="") depth_corr=a[1]
        }
        /Flat:.*lev =[[:space:]]*[0-9]+/ {
            match($0,/lev =[[:space:]]*([0-9]+)/,a)
            if (a[1]!="") lev=a[1]
            match($0,/and =[[:space:]]*([0-9]+)/,b)
            if (b[1]!="") and_count=b[1]
        }
        END { print mismatch_pct","depth_corr","lev","and_count }'
}

main() {
    export ABC_CMD AIG_DIR MFFC_DIR K MODE
    export -f process_aig_files extract_metrics
    parallel --jobs "$MAX_JOBS" process_aig_files {} ::: "$MFFC_DIR"/*.aig
}

main "$@"