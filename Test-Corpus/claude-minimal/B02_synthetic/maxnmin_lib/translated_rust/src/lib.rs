// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust from c_src/src/lib.c

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

struct NodeStorage {
    nodes: [Node; MAX_NODES],
    count: usize,
}

impl NodeStorage {
    const fn new() -> Self {
        NodeStorage {
            nodes: [Node::new(); MAX_NODES],
            count: 0,
        }
    }

    fn add_node(&mut self, id: i32, parent_id: i32, name: &str, value: f64) -> i32 {
        if self.count >= MAX_NODES {
            return -1;
        }

        let mut new_node = Node {
            id,
            parent_id,
            name: [0; MAX_NAME_LEN],
            value,
            active: 1,
        };

        // Mimic strncpy(new_node.name, name, MAX_NAME_LEN - 1)
        // and explicit null termination at MAX_NAME_LEN - 1.
        let bytes = name.as_bytes();
        let copy_len = bytes.len().min(MAX_NAME_LEN - 1);
        new_node.name[..copy_len].copy_from_slice(&bytes[..copy_len]);
        new_node.name[MAX_NAME_LEN - 1] = 0;

        self.nodes[self.count] = new_node;
        self.count += 1;
        (self.count - 1) as i32
    }

    fn find_node_by_id(&self, id: i32) -> Option<usize> {
        for i in 0..self.count {
            if self.nodes[i].id == id && self.nodes[i].active != 0 {
                return Some(i);
            }
        }
        None
    }

    fn get_children_count(&self, parent_id: i32) -> i32 {
        let mut count = 0;
        for i in 0..self.count {
            if self.nodes[i].parent_id == parent_id && self.nodes[i].active != 0 {
                count += 1;
            }
        }
        count
    }

    fn calculate_subtree_sum(&self, node_id: i32) -> f64 {
        let node_idx = match self.find_node_by_id(node_id) {
            Some(idx) => idx,
            None => return 0.0,
        };

        let mut sum = self.nodes[node_idx].value;

        for i in 0..self.count {
            if self.nodes[i].parent_id == node_id && self.nodes[i].active != 0 {
                sum += self.calculate_subtree_sum(self.nodes[i].id);
            }
        }

        sum
    }
}

fn process_string(name: &[u8]) -> i32 {
    let mut result: i32 = 0;
    // Treat as a null-terminated C string: stop at first NUL.
    for &b in name.iter() {
        if b == 0 {
            break;
        }
        // Mimic C: result += (int)(*str), where char may be signed.
        // C's `(int)(*str)` for a signed char interprets bytes >= 128 as negatives.
        // To match typical signed-char behavior, sign-extend.
        let signed = b as i8 as i32;
        result = result.wrapping_add(signed);
    }
    result
}

fn safe_double_to_int(d: f64) -> i32 {
    if d.is_nan() {
        return 0;
    }
    if d > i32::MAX as f64 {
        return i32::MAX;
    }
    if d < i32::MIN as f64 {
        return i32::MIN;
    }
    d as i32
}

pub fn maxnmin(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let mut result: i32 = 0;

    let mut storage = NodeStorage::new();

    storage.add_node(1, -1, "root", 10.5);
    storage.add_node(2, 1, "child1", 20.7);
    storage.add_node(3, 1, "child2", 15.3);
    storage.add_node(4, 2, "grandchild1", 5.9);
    storage.add_node(5, 2, "grandchild2", 8.2);
    storage.add_node(6, 3, "grandchild3", 12.4);

    let node_id = (param1 % 6) + 1;
    if let Some(idx) = storage.find_node_by_id(node_id) {
        let name = storage.nodes[idx].name;
        if name[0] != 0 {
            result = result.wrapping_add(process_string(&name));
        }

        let subtree_sum = storage.calculate_subtree_sum(node_id);
        let sum_as_int = safe_double_to_int(subtree_sum);
        result = result.wrapping_add(sum_as_int);
    }

    let second_node_id = (param2 % 6) + 1;
    if let Some(idx) = storage.find_node_by_id(second_node_id) {
        let value_multiplied = storage.nodes[idx].value * (param3 as f64);
        let converted_value = safe_double_to_int(value_multiplied);
        result = result.wrapping_add(converted_value);
    }

    let parent_id = (param4 % 3) + 1;
    let children = storage.get_children_count(parent_id);
    result = result.wrapping_add(children.wrapping_mul(10));

    let calculation = ((param1 as f64) + (param2 as f64)) / ((param3 as f64) + 1.0);
    let calculation = calculation * (param4 as f64);

    let final_calc = safe_double_to_int(calculation);
    result = result.wrapping_add(final_calc);

    result
}

#[no_mangle]
pub extern "C" fn maxnmin_c(a: i32, b: i32, c: i32, d: i32) -> i32 {
    maxnmin(a, b, c, d)
}
