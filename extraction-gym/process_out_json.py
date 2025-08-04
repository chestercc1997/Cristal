import os
import json

def process_json(input_file, a):
    # 读取输入文件
    with open(input_file, 'r') as f:
        data = json.load(f)

    choices = data['choices']
    values = list(choices.values())

    # 读取 graph_internal_serd.json 文件
    input_dir = os.path.join(os.getcwd(), 'data', 'my_data')
    print(input_dir)
    files = os.listdir(input_dir)
    json_files = [file for file in files if file.endswith('.json')]
    graph_file = os.path.join(input_dir, json_files[0])
    with open(graph_file, 'r') as f:
         graph_data = json.load(f)

    # 构建新的结果字典，只保留存在于values列表中的键
    new_nodes = {key: value for key, value in graph_data['nodes'].items() if key in values}

    # 构建最终的结果字典
    result = {'nodes': new_nodes}
   
    # 获取输入文件的子文件名
    file_name = os.path.basename(input_file)

    # 构建输出文件路径
    if(a==1):
      output_file = os.path.join('out_process_result1', file_name)
    else:
      output_file = os.path.join('out_process_dag_result1', file_name)
        
    # 输出结果
    with open(output_file, 'w') as f:
        json.dump(result, f, indent=2)

    #print(f'处理完成，结果保存在文件: {output_file}')

def process_json1(input_file,a):
    # 读取输入文件
    with open(input_file, 'r') as f:
        data = json.load(f)

    # 处理键中的小数点和小数部分
    new_nodes = {}
    for key, value in data['nodes'].items():
        new_key = key.split('.')[0]  # 去除小数点和小数部分
        value['children'] = [child.split('.')[0] for child in value['children']]  # 处理 "children" 中的数字
        new_nodes[new_key] = value

    # 构建最终的结果字典
    result = {'nodes': new_nodes}

    # 获取输入文件的子文件名
    file_name = os.path.basename(input_file)

    # 构建输出文件路径
    if(a==1):
       output_dir = 'out_process_result1'
    else :
       output_dir = 'out_process_dag_result1'
    os.makedirs(output_dir, exist_ok=True)
    output_file = os.path.join(output_dir, file_name)

    # 输出结果
    with open(output_file, 'w') as f:
        json.dump(result, f, indent=2)

    #print(f'处理完成，结果保存在文件: {output_file}')

# 遍历目录下的所有JSON文件
output_dir = 'out_process_result1'
os.makedirs(output_dir, exist_ok=True)
input_dir = os.path.join(os.getcwd(), 'out_json', 'my_data')
#input_dir = '/data/cchen/extraction-gym-new/extraction-gym/out_json/'
files = [file for file in os.listdir(input_dir)]

for file in files:
    input_file = os.path.join(input_dir, file)

    if os.path.isfile(input_file):
        #print(f'处理文件: {input_file}')
        process_json(input_file,1)
    else:
        print(f'文件不存在: {input_file}')

output_dir = 'out_process_dag_result1'
os.makedirs(output_dir, exist_ok=True)
#input_dir = '/data/cchen/extraction-gym-new/extraction-gym/out_dag_json/'
input_dir = os.path.join(os.getcwd(), 'out_dag_json', 'my_data')
files = [file for file in os.listdir(input_dir)]

for file in files:
    input_file = os.path.join(input_dir, file)

    if os.path.isfile(input_file):
       # print(f'处理文件: {input_file}')
        process_json(input_file,0)
    else:
        print(f'文件不存在: {input_file}')






# 遍历目录下的所有文件
input_dir = os.path.join(os.getcwd(), 'out_process_result1')
#input_dir = '/data/cchen/extraction-gym-new/extraction-gym/out_process_result/'
files = [file for file in os.listdir(input_dir)]

for file in files:
    input_file = os.path.join(input_dir, file)

    if os.path.isfile(input_file):
      #  print(f'处理文件: {input_file}')
        process_json1(input_file,1)
    else:
        print(f'文件不存在: {input_file}')

input_dir = os.path.join(os.getcwd(), 'out_process_dag_result1') 
#input_dir = '/data/cchen/extraction-gym-new/extraction-gym/out_process_dag_result/'
files = [file for file in os.listdir(input_dir)]

for file in files:
    input_file = os.path.join(input_dir, file)

    if os.path.isfile(input_file):
     #   print(f'处理文件: {input_file}')
        process_json1(input_file,0)
    else:
        print(f'文件不存在: {input_file}')


input_dir = os.path.join(os.getcwd(), 'out_process_dag_result1')
# input_dir = '/data/cchen/extraction-gym-new/extraction-gym/out_process_dag_result/'

files = os.listdir(input_dir)

for file in files:
    input_file = os.path.join(input_dir, file)

    if os.path.isfile(input_file):
        filename, extension = os.path.splitext(input_file)
        
        if extension != '.json':
            new_file = input_file + '.json'
            os.rename(input_file, new_file)



input_dir = os.path.join(os.getcwd(), 'data', 'my_data')
print(input_dir)
files = os.listdir(input_dir)
json_files = [file for file in files if file.endswith('.json')]
graph_file = os.path.join(input_dir, json_files[0])
with open(graph_file, 'r') as f:
    source_data = json.load(f)
    root_eclasses = source_data.get("root_eclasses", [])


output_dir = os.path.join(os.getcwd(), 'out_process_dag_result1')

for filename in os.listdir(output_dir):
    if filename.endswith(".json"):
        target_file_path = os.path.join(output_dir, filename)

        # 读取目标文件内容
        with open(target_file_path, "r") as target_file:
            target_data = json.load(target_file)

        # 向数据添加键值对
        target_data["root_eclasses"] = root_eclasses

        # 将更新后的数据写入目标文件
        with open(target_file_path, "w") as target_file:
            json.dump(target_data, target_file, indent=4)



