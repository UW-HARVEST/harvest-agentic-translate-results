use std::sync::Mutex;

const MAX_NODES: usize = 100;
const MAX_NAME_LEN: usize = 50;

#[derive(Clone, Copy)]
struct Node {
    id: i32,
    parent_id: i32,
    name: [u8; MAX_NAME_LEN],
    value: f64,
    active: i32,
}

impl Node {
    const fn new() -> Self {
        Node {
            id: 0,
            parent_id: 0,
            name: [0; MAX_NAME_LEN],
            value: 0.0,
            active: 0,
        }
    }
}

static NODE_STORAGE: Mutex<[Node; MAX_NODES]> = Mutex::new([Node::new(); MAX_NODES]);
static NODE_COUNT: Mutex<usize> = Mutex::new(0);

fn add_node(id: i32, parent_id: i32, name: &str, value: f64) -> i32 {
    let mut count = NODE_COUNT.lock().unwrap();
    if *count >= MAX_NODES {
        return -1;
    }

    let mut new_node = Node {
        id,
        parent_id,
        value,
        active: 1,
        name: [0; MAX_NAME_LEN],
    };

    let bytes = name.as_bytes();
    let len = bytes.len().min(MAX_NAME_LEN - 1);
    new_node.name[..len].copy_from_slice(&bytes[..len]);
    new_node.name[MAX_NAME_LEN - 1] = 0;

    let mut storage = NODE_STORAGE.lock().unwrap();
    storage[*count] = new_node;
    *count += 1;
    (*count - 1) as i32
}

fn find_node_by_id(id: i32) -> Option<Node> {
    let count = *NODE_COUNT.lock().unwrap();
    let storage = NODE_STORAGE.lock().unwrap();
    for i in 0..count {
        if storage[i].id == id && storage[i].active != 0 {
            return Some(storage[i]);
        }
    }
    None
}

fn get_children_count(parent_id: i32) -> i32 {
    let mut count = 0;
    let node_count = *NODE_COUNT.lock().unwrap();
    let storage = NODE_STORAGE.lock().unwrap();
    for i in 0..node_count {
        if storage[i].parent_id == parent_id && storage[i].active != 0 {
            count += 1;
        }
    }
    count
}

fn calculate_subtree_sum(node_id: i32) -> f64 {
    let node = match find_node_by_id(node_id) {
        Some(n) => n,
        None => return 0.0,
    };

    let mut sum = node.value;

    let count = *NODE_COUNT.lock().unwrap();
    let mut children = Vec::new();
    {
        let storage = NODE_STORAGE.lock().unwrap();
        for i in 0..count {
            if storage[i].parent_id == node_id && storage[i].active != 0 {
                children.push(storage[i].id);
            }
        }
    }

    for child_id in children {
        sum += calculate_subtree_sum(child_id);
    }

    sum
}

fn process_string(name: &[u8]) -> i32 {
    let mut result = 0;
    if name[0] != 0 {
        for &b in name {
            if b == 0 {
                break;
            }
            result += b as i8 as i32;
        }
    }
    result
}

fn safe_double_to_int(d: f64) -> i32 {
    if d > i32::MAX as f64 {
        return i32::MAX;
    }
    if d < i32::MIN as f64 {
        return i32::MIN;
    }
    if d.is_nan() {
        return 0;
    }
    d as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn maxnmin(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let mut result = 0;

    *NODE_COUNT.lock().unwrap() = 0;

    add_node(1, -1, "root", 10.5);
    add_node(2, 1, "child1", 20.7);
    add_node(3, 1, "child2", 15.3);
    add_node(4, 2, "grandchild1", 5.9);
    add_node(5, 2, "grandchild2", 8.2);
    add_node(6, 3, "grandchild3", 12.4);

    let node_id = (param1 % 6) + 1;
    if let Some(selected_node) = find_node_by_id(node_id) {
        if selected_node.name[0] != 0 {
            result += process_string(&selected_node.name);
        }

        let subtree_sum = calculate_subtree_sum(node_id);
        let sum_as_int = safe_double_to_int(subtree_sum);
        result += sum_as_int;
    }

    let second_node_id = (param2 % 6) + 1;
    if let Some(second_node) = find_node_by_id(second_node_id) {
        let value_multiplied = second_node.value * (param3 as f64);
        let converted_value = safe_double_to_int(value_multiplied);
        result += converted_value;
    }

    let parent_id = (param4 % 3) + 1;
    let children = get_children_count(parent_id);
    result += children * 10;

    let mut calculation = (param1 + param2) as f64 / (param3 + 1) as f64;
    calculation *= param4 as f64;

    let final_calc = safe_double_to_int(calculation);
    result += final_calc;

    result
}
