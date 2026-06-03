// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::sync::Mutex;

#[derive(Clone, Copy)]
struct Node {
    id: i32,
    parent_id: i32,
    value: f64,
    data: [i32; 4],
}

impl Node {
    const fn new() -> Self {
        Node {
            id: 0,
            parent_id: 0,
            value: 0.0,
            data: [0; 4],
        }
    }
}

const MAX_NODES: usize = 100;

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
}

static NODE_STORAGE: Mutex<NodeStorage> = Mutex::new(NodeStorage::new());

const STATUS_OK: i32 = 0o0;
#[allow(dead_code)]
const STATUS_WARNING: i32 = 0o1;
const STATUS_ERROR: i32 = 0o2;
#[allow(dead_code)]
const STATUS_CRITICAL: i32 = 0o377;

fn find_node_index_by_id(storage: &NodeStorage, id: i32) -> Option<usize> {
    for i in 0..storage.count {
        if storage.nodes[i].id == id {
            return Some(i);
        }
    }
    None
}

fn add_node(storage: &mut NodeStorage, id: i32, parent_id: i32, value: f64) -> i32 {
    if storage.count >= MAX_NODES {
        return STATUS_ERROR;
    }

    let idx = storage.count;
    storage.nodes[idx].id = id;
    storage.nodes[idx].parent_id = parent_id;
    storage.nodes[idx].value = value;

    storage.nodes[idx].data[0] = 0o100;
    storage.nodes[idx].data[1] = 0o200;
    storage.nodes[idx].data[2] = 0o300;
    storage.nodes[idx].data[3] = 0o400;

    storage.count += 1;
    STATUS_OK
}

fn process_backward(array: &[i32], size: usize, start_offset: usize) -> i32 {
    let mut sum: i32 = 0;
    let mut ptr = size;
    let start = start_offset;

    while ptr > start {
        ptr -= 1;
        sum = sum.wrapping_add(array[ptr]);
    }

    sum
}

fn compute_size_metric(s: &str) -> i32 {
    let len = s.len();
    let mut metric = len as i32;
    metric = metric.wrapping_mul(2).wrapping_add(0o10);
    metric
}

fn safe_double_to_int(mut value: f64) -> i32 {
    if value > 2147483647.0 {
        value = 2147483647.0;
    }
    if value < -2147483648.0 {
        value = -2147483648.0;
    }

    value as i32
}

pub fn jumpnode(operation_mode: i32, node_id: i32, depth: i32, flags: i32) -> i32 {
    let storage = NODE_STORAGE.lock().unwrap();
    #[allow(unused_assignments)]
    let mut result: i32 = 0;

    match operation_mode {
        0o1 => {
            let mut current_idx = match find_node_index_by_id(&storage, node_id) {
                Some(i) => i,
                None => return STATUS_ERROR | 0o20,
            };

            let mut accumulated_value = storage.nodes[current_idx].value;

            let mut i = 0;
            while i < depth && storage.nodes[current_idx].parent_id != -1 {
                let parent_id = storage.nodes[current_idx].parent_id;
                let parent_idx = match find_node_index_by_id(&storage, parent_id) {
                    Some(p) => p,
                    None => break,
                };

                accumulated_value += storage.nodes[parent_idx].value * 1.5;
                current_idx = parent_idx;
                i += 1;
            }

            result = safe_double_to_int(accumulated_value);
        }

        0o2 => {
            let current_idx = match find_node_index_by_id(&storage, node_id) {
                Some(i) => i,
                None => return STATUS_ERROR | 0o40,
            };

            let mut temp_array: [i32; 20] = [0; 20];

            for i in 0..4 {
                temp_array[i] = storage.nodes[current_idx].data[i];
            }

            for i in 4..0o20 {
                temp_array[i] = (i as i32) * 0o7;
            }

            let array_size: usize = 0o20;

            let start_offset = if depth < 0 {
                0
            } else if (depth as usize) > array_size {
                array_size
            } else {
                depth as usize
            };

            result = process_backward(&temp_array, array_size, start_offset);

            result = result.wrapping_add((array_size as i32).wrapping_mul(flags));
        }

        0o3 => {
            let buffer = format!("Node_{}_Depth_{}", node_id, depth);

            result = compute_size_metric(&buffer);

            result = result.wrapping_add(flags & 0o177);
        }

        0o4 => {
            let current_idx = match find_node_index_by_id(&storage, node_id) {
                Some(i) => i,
                None => return STATUS_ERROR | 0o100,
            };

            let mut accumulated_value: f64 = 0.0;
            for i in 0..4 {
                accumulated_value +=
                    (storage.nodes[current_idx].data[i] as f64).sqrt() * 2.718281828;
            }

            accumulated_value *= 1.0 + (depth as f64) * 0.1;

            result = safe_double_to_int(accumulated_value);

            if storage.count > 2 {
                let mut iter = storage.count;
                let mut backward_sum: i32 = 0;

                let mut i = 0;
                while i < 3 && iter > 0 {
                    iter -= 1;
                    backward_sum =
                        backward_sum.wrapping_add(safe_double_to_int(storage.nodes[iter].value));
                    i += 1;
                }

                result = result.wrapping_add(backward_sum);
            }
        }

        _ => {
            result = STATUS_ERROR | 0o200;
        }
    }

    result
}

#[allow(dead_code)]
fn initialize_test_data() {
    let mut storage = NODE_STORAGE.lock().unwrap();
    storage.count = 0;

    add_node(&mut storage, 1, -1, 100.5);
    add_node(&mut storage, 2, 1, 50.25);
    add_node(&mut storage, 3, 1, 75.75);
    add_node(&mut storage, 4, 2, 25.125);
    add_node(&mut storage, 5, 2, 30.875);
    add_node(&mut storage, 6, 3, 40.0625);
    add_node(&mut storage, 7, 4, 12.5);
}
