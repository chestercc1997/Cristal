import os
import re

input_dir = "./benchmarks/exp/benchmarks/exp_aig"
output_dir = "./benchmarks/exp/benchmarks/exp_aig_new"

os.makedirs(output_dir, exist_ok=True)

aig_files = [f for f in os.listdir(input_dir) if f.endswith(".aig")]

for aig_file in aig_files:
    input_path = os.path.join(input_dir, aig_file)
    output_path = os.path.join(output_dir, aig_file)

    with open(input_path, "rb") as file:
        lines = file.readlines()

    max_i = 0
    max_o = 0
    for line in lines:
        try:
            decoded_tail = line[-100:].decode("utf-8", errors="ignore")
            i_matches = re.findall(r"i(\d+)", decoded_tail)
            o_matches = re.findall(r"o(\d+)", decoded_tail)
            max_i = max([max_i] + [int(m) for m in i_matches])
            max_o = max([max_o] + [int(m) for m in o_matches])
        except UnicodeDecodeError:
            continue

    pi_digits = len(str(max_i)) if max_i > 0 else 1
    po_digits = len(str(max_o)) if max_o > 0 else 1

    processed_lines = []
    for line in lines:
        original_line = line

        # 处理输入端口编号（iX）
        new_line = original_line
        for match in re.finditer(rb'i(\d+)(.*?)(\n|$)', new_line):
            num_str = match.group(1)
            index = int(num_str)
            # 在原始的 iX 后追加 piXX
            replacement = f"i{index} pi{index:0{pi_digits}d}".encode("utf-8") + match.group(3)
            start = match.start()
            end = match.end()
            new_line = new_line[:start] + replacement + new_line[end:]

        # 处理输出端口编号（oX）
        for match in re.finditer(rb'o(\d+)(.*?)(\n|$)', new_line):
            num_str = match.group(1)
            index = int(num_str)
            # 在原始的 oX 后追加 poXX
            replacement = f"o{index} po{index:0{po_digits}d}".encode("utf-8") + match.group(3)
            start = match.start()
            end = match.end()
            new_line = new_line[:start] + replacement + new_line[end:]

        processed_lines.append(new_line)

    with open(output_path, "wb") as file:
        file.writelines(processed_lines)

    print(f"Processed {aig_file} -> {output_path}")