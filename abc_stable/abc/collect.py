import os
import re
import csv

# 输入目录路径
# INPUT_DIR = "Out_test_supergate_delay"
INPUT_DIR = "Out_test_supergate_area"
# 输出 CSV 文件路径
# OUTPUT_CSV = "result_delay.csv"
OUTPUT_CSV = "result_area.csv"
# 正则表达式，用于匹配 area、delay 和 runtime
pattern_area_delay = re.compile(r"area\s*=\s*([\d.]+)\s+delay\s*=\s*([\d.]+)")
pattern_runtime = re.compile(r"Total runtime\s*=\s*([\d.]+)\s*sec")

def parse_area_delay_runtime(file_path):
    """解析 txt 文件，提取两组 area、delay 和 runtime 值"""
    with open(file_path, "r", encoding="utf-8") as file:
        content = file.read()
        # 使用正则匹配两组 area 和 delay
        matches_area_delay = pattern_area_delay.findall(content)
        # 使用正则匹配两组 runtime
        matches_runtime = pattern_runtime.findall(content)
        
        if len(matches_area_delay) >= 2 and len(matches_runtime) >= 2:
            # 返回第一组和第二组的 area、delay 和 runtime
            return [
                float(matches_area_delay[0][0]),  # 第一组 area
                float(matches_area_delay[0][1]),  # 第一组 delay
                float(matches_runtime[0]),        # 第一组 runtime
                float(matches_area_delay[1][0]),  # 第二组 area
                float(matches_area_delay[1][1]),  # 第二组 delay
                float(matches_runtime[1])         # 第二组 runtime
            ]
        else:
            return [None, None, None, None, None, None]

def calculate_improvement(first, second):
    """计算第二组相对于第一组的 area、delay 和 runtime 提升百分比"""
    if first and second:
        area_improvement = ((first[0] - second[0]) / first[0]) * 100 if first[0] != 0 else 0
        delay_improvement = ((first[1] - second[1]) / first[1]) * 100 if first[1] != 0 else 0
        runtime_improvement = ((first[2] - second[2]) / first[2]) * 100 if first[2] != 0 else 0
        return round(area_improvement, 2), round(delay_improvement, 2), round(runtime_improvement, 2)
    else:
        return None, None, None

def main():
    # 存储结果
    results = []
    first_totals = [0, 0, 0]  # 累计第一组的 area、delay 和 runtime
    second_totals = [0, 0, 0] # 累计第二组的 area、delay 和 runtime
    file_count = 0

    # 遍历目录中的所有 txt 文件
    for file_name in os.listdir(INPUT_DIR):
        if file_name.endswith(".txt"):
            file_path = os.path.join(INPUT_DIR, file_name)
            file_base_name = os.path.splitext(file_name)[0]  # 去掉 .txt 后缀
            # 解析文件内容
            area_delay_runtime = parse_area_delay_runtime(file_path)
            if area_delay_runtime[0] is not None:
                # 第一组和第二组的 area、delay 和 runtime
                first_area, first_delay, first_runtime = area_delay_runtime[0:3]
                second_area, second_delay, second_runtime = area_delay_runtime[3:6]
                # 计算提升百分比
                area_improvement, delay_improvement, runtime_improvement = calculate_improvement(
                    [first_area, first_delay, first_runtime],
                    [second_area, second_delay, second_runtime]
                )
                # 添加到结果列表
                results.append([
                    file_base_name, first_area, first_delay, first_runtime,
                    second_area, second_delay, second_runtime,
                    area_improvement, delay_improvement, runtime_improvement
                ])
                # 累计统计
                first_totals[0] += first_area
                first_totals[1] += first_delay
                first_totals[2] += first_runtime
                second_totals[0] += second_area
                second_totals[1] += second_delay
                second_totals[2] += second_runtime
                file_count += 1

    # 计算平均值
    first_avg = [first_totals[0] / file_count, first_totals[1] / file_count, first_totals[2] / file_count]
    second_avg = [second_totals[0] / file_count, second_totals[1] / file_count, second_totals[2] / file_count]

    # 计算平均提升百分比
    avg_area_improvement, avg_delay_improvement, avg_runtime_improvement = calculate_improvement(
        first_avg, second_avg
    )

    # 将平均值和提升百分比添加到结果
    results.append([
        "avg", first_avg[0], first_avg[1], first_avg[2],
        second_avg[0], second_avg[1], second_avg[2],
        avg_area_improvement, avg_delay_improvement, avg_runtime_improvement
    ])

    # 写入 CSV 文件
    with open(OUTPUT_CSV, "w", newline="", encoding="utf-8") as csvfile:
        csv_writer = csv.writer(csvfile)
        # 写入表头
        csv_writer.writerow([
            "File Name", "First Area", "First Delay", "First Runtime",
            "Second Area", "Second Delay", "Second Runtime",
            "Area Improvement (%)", "Delay Improvement (%)", "Runtime Improvement (%)"
        ])
        # 写入数据
        csv_writer.writerows(results)

    print(f"结果已保存到 {OUTPUT_CSV}")

if __name__ == "__main__":
    main()