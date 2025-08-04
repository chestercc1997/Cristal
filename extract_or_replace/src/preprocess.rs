
use std::fs;
use std::path::PathBuf;
use serde_json::Value;

pub fn process_data(data: &mut serde_json::Value) {
    if let Some(entries) = data.as_array_mut() {
        for entry in entries.iter_mut() {
            if let Some(entry_obj) = entry.as_object_mut() {
                process_entry(entry_obj);
            }
        }
    } else if let Some(obj) = data.as_object_mut() {
        process_entry(obj);
        for (_, value) in obj.iter_mut() {
            process_data(value);
        }
    }
}

//sub function for process_json_prop
pub fn process_entry(entry_obj: &mut serde_json::Map<String, serde_json::Value>) {
    // handle "op" and "children" keys,values pair
    if let Some(op_value) = entry_obj.remove("op") {
        if let Some(children_value) = entry_obj.remove("children") {
            entry_obj.insert(op_value.as_str().unwrap().to_owned(), children_value);
        }
    }

    for (key, value) in entry_obj.clone().into_iter() {
        if let serde_json::Value::Array(ref arr) = value {
            if arr.is_empty() {
                entry_obj.remove(&key);
                entry_obj.insert("Symbol".to_owned(), serde_json::Value::String(key.clone()));
            }
        }
        if key == "Not" || key == "Root" {
            if let serde_json::Value::Array(ref arr1) = value {
                if arr1.len() == 1 {
                    let new_value = arr1[0].clone();
                    entry_obj.insert(key, new_value);
                }
            }
        }
    }
}
pub fn process_json_prop(json_file: &str) -> String {
    let json_str = fs::read_to_string(json_file).expect("Failed to read JSON file");
    let mut data: Value = serde_json::from_str(&json_str).unwrap();

    // handle "memo"
    if let Some(classes) = data
        .get_mut("classes")
        .and_then(|classes| classes.as_object_mut())
    {
        for class in classes.values_mut() {
            if let Some(nodes) = class
                .get_mut("nodes")
                .and_then(|nodes| nodes.as_array_mut())
            {
                for node in nodes.iter_mut() {
                    //    println!("Processed node: {:?}", node);
                    process_data(node);
                }
            }
            if let Some(parents) = class
                .get_mut("parents")
                .and_then(|parents| parents.as_array_mut())
            {
                for parent in parents.iter_mut() {
                    //    println!("Processed parent: {:?}", parent);
                    process_data(parent);
                }
            }
        }
    }

    if let Some(memo) = data.get_mut("memo").and_then(|memo| memo.as_array_mut()) {
        for entry in memo.iter_mut() {
            process_data(entry);
        }
    }

    // converted the modified data into a json string
    let modified_json_str = serde_json::to_string_pretty(&data).unwrap();
    // make the modified json file name
    let json_file_path = PathBuf::from(json_file);
    let modified_json_file = json_file_path.with_file_name(format!(
        "modified_{}",
        json_file_path.file_name().unwrap().to_str().unwrap()
    ));
    // write the modified json file
    fs::write(&modified_json_file, modified_json_str).expect("Failed to write modified JSON file");
    modified_json_file.to_str().unwrap().to_owned()
}
