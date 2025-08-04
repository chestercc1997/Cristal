import os
import json
import concurrent.futures
from pathlib import Path

def is_int(s: str) -> bool:
    try:
        int(s)
        return True
    except ValueError:
        return False

def read_json_file(filename: str):
    try:
        with open(filename, 'r', encoding='utf-8') as f:
            data = json.load(f)
        return data
    except Exception as e:
        print(f"Error reading JSON file: {e}")
        return None

def write_json_file(filename: str, data: dict):
    try:
        file_path = Path(filename)
        new_filename = file_path.with_name(file_path.stem + "_new" + file_path.suffix)
        with open(new_filename, 'w', encoding='utf-8') as f:
            json.dump(data, f, indent=4)
        print(f"New file written: {new_filename}")
    except Exception as e:
        print(f"Error writing JSON file: {e}")

def process_json_file(json_file: str):
    try:
        json_data = read_json_file(json_file)
        if json_data is None:
            return

        new_json_data = json_data

        for i in new_json_data['nodes']:
            child = []
            for j in json_data['nodes'][i]['children']:
                assert ("." in j)
                cid = j.split(".")
                assert (len(cid) == 2)
                ccid = cid[0]
                cnid = cid[1]
                assert (is_int(ccid) and is_int(cnid))
                cnum_cid = int(ccid)
                child.append(cnum_cid)

            new_json_data['nodes'][i]['children'] = child
            new_json_data['nodes'][i]['eclass'] = int(json_data['nodes'][i]['eclass'])
            assert ("." in i)
            id = i.split(".")
            assert (len(id) == 2)
            cid = id[0]
            nid = id[1]
            assert (is_int(cid) and is_int(nid))
            num_cid = int(cid)
            num_nid = int(nid)
            new_json_data['nodes'][i]['id'] = str(num_cid) + "." + str(num_nid)

        for i in new_json_data['root_eclasses']:
            assert (is_int(i))
            new_json_data['root_eclasses'][new_json_data['root_eclasses'].index(i)] = int(i)

        write_json_file(json_file, new_json_data)

    except Exception as e:
        print(f"Error processing file {json_file}: {e}")

def find_json_files(base_path: str):
    json_files = []
    for root, dirs, files in os.walk(base_path):
        for file in files:
            if file == "rewritten_egraph_with_weight_cost_serd.json":
                json_files.append(os.path.join(root, file))
    return json_files

if __name__ == "__main__":
    base_dir = "./flussab/aig_2_egraph/rewritten_circuits"
    json_files = find_json_files(base_dir)
    with concurrent.futures.ProcessPoolExecutor() as executor:
        executor.map(process_json_file, json_files)
    print("All files processed!")