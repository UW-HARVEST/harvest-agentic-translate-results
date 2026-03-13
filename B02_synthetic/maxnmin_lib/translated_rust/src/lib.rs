use std::ffi::c_int;

const MAX_NODES: usize = 100;
const MAX_NAME_LEN: usize = 50;

#[derive(Clone)]
struct Node {
    id: c_int,
    parent_id: c_int,
    name: [u8; MAX_NAME_LEN],
    value: f64,
    active: c_int,
}

impl Node {
    fn zeroed() -> Self {
        Node {
            id: 0,
            parent_id: 0,
            name: [0u8; MAX_NAME_LEN],
            value: 0.0,
            active: 0,
        }
    }
}

static mut NODE_STORAGE: Option<Vec<Node>> = None;
static mut NODE_COUNT: c_int = 0;

fn storage() -> &'static mut Vec<Node> {
    unsafe {
        if NODE_STORAGE.is_none() {
            NODE_STORAGE = Some(vec![Node::zeroed(); MAX_NODES]);
        }
        NODE_STORAGE.as_mut().unwrap()
    }
}

fn add_node(id: c_int, parent_id: c_int, name: &[u8], value: f64) -> c_int {
    unsafe {
        if NODE_COUNT >= MAX_NODES as c_int {
            return -1;
        }
        let mut new_node = Node::zeroed();
        new_node.id = id;
        new_node.parent_id = parent_id;
        new_node.value = value;
        new_node.active = 1;

        // strncpy behavior: copy up to MAX_NAME_LEN-1 bytes
        let copy_len = name.len().min(MAX_NAME_LEN - 1);
        new_node.name[..copy_len].copy_from_slice(&name[..copy_len]);
        new_node.name[MAX_NAME_LEN - 1] = 0;

        let idx = NODE_COUNT as usize;
        storage()[idx] = new_node;
        NODE_COUNT += 1;
        NODE_COUNT - 1
    }
}

fn find_node_by_id(id: c_int) -> Option<usize> {
    unsafe {
        for i in 0..NODE_COUNT as usize {
            if storage()[i].id == id && storage()[i].active != 0 {
                return Some(i);
            }
        }
        None
    }
}

fn get_children_count(parent_id: c_int) -> c_int {
    unsafe {
        let mut count = 0;
        for i in 0..NODE_COUNT as usize {
            if storage()[i].parent_id == parent_id && storage()[i].active != 0 {
                count += 1;
            }
        }
        count
    }
}

fn calculate_subtree_sum(node_id: c_int) -> f64 {
    let idx = match find_node_by_id(node_id) {
        Some(i) => i,
        None => return 0.0,
    };

    let mut sum = storage()[idx].value;

    unsafe {
        for i in 0..NODE_COUNT as usize {
            if storage()[i].parent_id == node_id && storage()[i].active != 0 {
                let child_id = storage()[i].id;
                sum += calculate_subtree_sum(child_id);
            }
        }
    }

    sum
}

fn process_string(s: &[u8]) -> c_int {
    let mut result: c_int = 0;
    for &b in s {
        if b == 0 {
            break;
        }
        result += b as c_int;
    }
    result
}

fn safe_double_to_int(d: f64) -> c_int {
    if d > c_int::MAX as f64 {
        return c_int::MAX;
    }
    if d < c_int::MIN as f64 {
        return c_int::MIN;
    }
    if d != d {
        return 0;
    }
    d as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn maxnmin(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    unsafe { NODE_COUNT = 0; }

    add_node(1, -1, b"root", 10.5);
    add_node(2, 1, b"child1", 20.7);
    add_node(3, 1, b"child2", 15.3);
    add_node(4, 2, b"grandchild1", 5.9);
    add_node(5, 2, b"grandchild2", 8.2);
    add_node(6, 3, b"grandchild3", 12.4);

    let node_id = (param1 % 6) + 1;
    if let Some(idx) = find_node_by_id(node_id) {
        let name = storage()[idx].name;
        if name[0] != 0 {
            result += process_string(&name);
        }

        let subtree_sum = calculate_subtree_sum(node_id);
        let sum_as_int = safe_double_to_int(subtree_sum);
        result += sum_as_int;
    }

    let second_node_id = (param2 % 6) + 1;
    if let Some(idx) = find_node_by_id(second_node_id) {
        let value_multiplied = storage()[idx].value * param3 as f64;
        let converted_value = safe_double_to_int(value_multiplied);
        result += converted_value;
    }

    let parent_id = (param4 % 3) + 1;
    let children = get_children_count(parent_id);
    result += children * 10;

    let calculation = (param1 + param2) as f64 / (param3 + 1) as f64 * param4 as f64;
    let final_calc = safe_double_to_int(calculation);
    result += final_calc;

    result
}
