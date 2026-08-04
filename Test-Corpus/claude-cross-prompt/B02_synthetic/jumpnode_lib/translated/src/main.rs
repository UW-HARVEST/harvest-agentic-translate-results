// Translated from c_src/src/lib.c
// The original C source is a shared library with no main().
// This binary defines the equivalent functions but produces no output,
// matching the byte-identical (empty) output behavior of the original.

#[derive(Clone, Copy)]
struct Node {
    id: i32,
    parent_id: i32,
    value: f64,
    data: [i32; 4],
}

const MAX_NODES: usize = 100;

const STATUS_OK: i32 = 0o000;
#[allow(dead_code)]
const STATUS_WARNING: i32 = 0o001;
const STATUS_ERROR: i32 = 0o002;
#[allow(dead_code)]
const STATUS_CRITICAL: i32 = 0o377;

struct NodeStorage {
    nodes: [Node; MAX_NODES],
    count: usize,
}

impl NodeStorage {
    #[allow(dead_code)]
    const fn new() -> Self {
        let zero = Node {
            id: 0,
            parent_id: 0,
            value: 0.0,
            data: [0; 4],
        };
        NodeStorage {
            nodes: [zero; MAX_NODES],
            count: 0,
        }
    }

    fn find_node_by_id(&self, id: i32) -> Option<usize> {
        for i in 0..self.count {
            if self.nodes[i].id == id {
                return Some(i);
            }
        }
        None
    }

    fn add_node(&mut self, id: i32, parent_id: i32, value: f64) -> i32 {
        if self.count >= MAX_NODES {
            return STATUS_ERROR;
        }
        let n = &mut self.nodes[self.count];
        n.id = id;
        n.parent_id = parent_id;
        n.value = value;
        n.data[0] = 0o100;
        n.data[1] = 0o200;
        n.data[2] = 0o300;
        n.data[3] = 0o400;
        self.count += 1;
        STATUS_OK
    }
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

#[allow(dead_code)]
fn jumpnode(
    storage: &mut NodeStorage,
    operation_mode: i32,
    node_id: i32,
    depth: i32,
    flags: i32,
) -> i32 {
    let mut result: i32;

    match operation_mode {
        0o001 => {
            let mut idx = match storage.find_node_by_id(node_id) {
                Some(i) => i,
                None => return STATUS_ERROR | 0o020,
            };

            let mut accumulated_value = storage.nodes[idx].value;

            let mut i = 0;
            while i < depth && storage.nodes[idx].parent_id != -1 {
                let parent_id = storage.nodes[idx].parent_id;
                let parent_idx = match storage.find_node_by_id(parent_id) {
                    Some(p) => p,
                    None => break,
                };
                accumulated_value += storage.nodes[parent_idx].value * 1.5;
                idx = parent_idx;
                i += 1;
            }

            result = safe_double_to_int(accumulated_value);
        }
        0o002 => {
            let idx = match storage.find_node_by_id(node_id) {
                Some(i) => i,
                None => return STATUS_ERROR | 0o040,
            };

            let mut temp_array: [i32; 20] = [0; 20];
            for i in 0..4 {
                temp_array[i] = storage.nodes[idx].data[i];
            }
            for i in 4..0o20 {
                temp_array[i] = (i as i32).wrapping_mul(0o007);
            }

            let array_size: usize = 0o20;
            let start = depth as usize;
            result = process_backward(&temp_array, array_size, start);
            result = result.wrapping_add((array_size as i32).wrapping_mul(flags));
        }
        0o003 => {
            let buffer = format!("Node_{}_Depth_{}", node_id, depth);
            result = compute_size_metric(&buffer);
            result = result.wrapping_add(flags & 0o177);
        }
        0o004 => {
            let idx = match storage.find_node_by_id(node_id) {
                Some(i) => i,
                None => return STATUS_ERROR | 0o100,
            };

            let mut accumulated_value: f64 = 0.0;
            for i in 0..4 {
                accumulated_value += (storage.nodes[idx].data[i] as f64).sqrt() * 2.718281828;
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
fn initialize_test_data(storage: &mut NodeStorage) {
    storage.count = 0;
    storage.add_node(1, -1, 100.5);
    storage.add_node(2, 1, 50.25);
    storage.add_node(3, 1, 75.75);
    storage.add_node(4, 2, 25.125);
    storage.add_node(5, 2, 30.875);
    storage.add_node(6, 3, 40.0625);
    storage.add_node(7, 4, 12.5);
}

fn main() {
    // The original C file is a library without a main function.
    // It produces no output when compiled and linked as an executable would
    // produce a link error; we instead produce no output here to remain
    // byte-identical to a "no output" expectation.
}
