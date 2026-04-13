use std::ffi::{c_char, c_int, c_double};
use std::os::raw::c_int as RawCInt;
use std::sync::Mutex;

const MAX_NODES: usize = 100;
const MAX_NAME_LEN: usize = 50;

struct Node {
    id: c_int,
    parent_id: c_int,
    name: [c_char; MAX_NAME_LEN],
    value: c_double,
    active: c_int,
}

static NODE_STORAGE: Mutex<[Option<Node>; MAX_NODES]> = Mutex::new([const { None }; MAX_NODES]);
static NODE_COUNT: Mutex<usize> = Mutex::new(0);

fn add_node_internal(id: c_int, parent_id: c_int, name: &[u8], value: c_double) -> c_int {
    let mut count = NODE_COUNT.lock().unwrap();
    if *count >= MAX_NODES {
        return -1;
    }

    let mut storage = NODE_STORAGE.lock().unwrap();
    let mut name_array = [0 as c_char; MAX_NAME_LEN];
    let copy_len = name.len().min(MAX_NAME_LEN - 1);
    for i in 0..copy_len {
        name_array[i] = name[i] as c_char;
    }

    let new_node = Node {
        id,
        parent_id,
        name: name_array,
        value,
        active: 1,
    };

    storage[*count] = Some(new_node);
    let index = *count as c_int;
    *count += 1;
    index
}

fn find_node_by_id_internal(id: c_int) -> Option<usize> {
    let storage = NODE_STORAGE.lock().unwrap();
    let count = *NODE_COUNT.lock().unwrap();
    for i in 0..count {
        if let Some(ref node) = storage[i] {
            if node.id == id && node.active != 0 {
                return Some(i);
            }
        }
    }
    None
}

fn get_children_count_internal(parent_id: c_int) -> c_int {
    let storage = NODE_STORAGE.lock().unwrap();
    let count = *NODE_COUNT.lock().unwrap();
    let mut result = 0;
    for i in 0..count {
        if let Some(ref node) = storage[i] {
            if node.parent_id == parent_id && node.active != 0 {
                result += 1;
            }
        }
    }
    result
}

fn calculate_subtree_sum_internal(node_id: c_int) -> c_double {
    let idx = match find_node_by_id_internal(node_id) {
        Some(i) => i,
        None => return 0.0,
    };

    let storage = NODE_STORAGE.lock().unwrap();
    let node = storage[idx].as_ref().unwrap();
    let mut sum = node.value;
    let target_id = node.id;
    drop(storage);

    let count = *NODE_COUNT.lock().unwrap();
    let storage = NODE_STORAGE.lock().unwrap();
    for i in 0..count {
        if let Some(ref n) = storage[i] {
            if n.parent_id == target_id && n.active != 0 {
                drop(storage);
                sum += calculate_subtree_sum_internal(n.id);
                let storage = NODE_STORAGE.lock().unwrap();
            }
        }
    }

    sum
}

fn process_string_internal(s: &[u8]) -> c_int {
    let mut result: c_int = 0;
    for &c in s {
        if c == 0 {
            break;
        }
        result += c as c_int;
    }
    result
}

fn safe_double_to_int_internal(d: c_double) -> c_int {
    if d > c_int::MAX as c_double {
        return c_int::MAX;
    }
    if d < c_int::MIN as c_double {
        return c_int::MIN;
    }
    if d.is_nan() {
        return 0;
    }
    d as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn maxnmin(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    {
        let mut count = NODE_COUNT.lock().unwrap();
        *count = 0;
        let mut storage = NODE_STORAGE.lock().unwrap();
        for i in 0..MAX_NODES {
            storage[i] = None;
        }
    }

    add_node_internal(1, -1, b"root", 10.5);
    add_node_internal(2, 1, b"child1", 20.7);
    add_node_internal(3, 1, b"child2", 15.3);
    add_node_internal(4, 2, b"grandchild1", 5.9);
    add_node_internal(5, 2, b"grandchild2", 8.2);
    add_node_internal(6, 3, b"grandchild3", 12.4);

    let node_id = (param1 % 6) + 1;

    if let Some(idx) = find_node_by_id_internal(node_id) {
        let storage = NODE_STORAGE.lock().unwrap();
        let node = storage[idx].as_ref().unwrap();
        let name_bytes: Vec<u8> = node.name.iter()
            .take_while(|&&c| c != 0)
            .map(|&c| c as u8)
            .collect();
        drop(storage);

        if !name_bytes.is_empty() {
            result += process_string_internal(&name_bytes);
        }

        let subtree_sum = calculate_subtree_sum_internal(node_id);
        let sum_as_int = safe_double_to_int_internal(subtree_sum);
        result += sum_as_int;
    }

    let second_node_id = (param2 % 6) + 1;

    if let Some(idx) = find_node_by_id_internal(second_node_id) {
        let storage = NODE_STORAGE.lock().unwrap();
        let node = storage[idx].as_ref().unwrap();
        let value_multiplied = node.value * param3 as c_double;
        drop(storage);

        let converted_value = safe_double_to_int_internal(value_multiplied);
        result += converted_value;
    }

    let parent_id = (param4 % 3) + 1;
    let children = get_children_count_internal(parent_id);
    result += children * 10;

    let calculation = (param1 + param2) as c_double / (param3 + 1) as c_double;
    let calculation = calculation * param4 as c_double;

    let final_calc = safe_double_to_int_internal(calculation);
    result += final_calc;

    result
}
