use std::ffi::c_int;
use std::sync::{Mutex, MutexGuard};

const MAX_NODES: usize = 100;

const STATUS_OK: c_int = 0o0000;
const STATUS_ERROR: c_int = 0o0002;

#[derive(Clone, Copy)]
struct Node {
    id: c_int,
    parent_id: c_int,
    value: f64,
    data: [c_int; 4],
}

const EMPTY_NODE: Node = Node {
    id: 0,
    parent_id: 0,
    value: 0.0,
    data: [0; 4],
};

struct NodeState {
    storage: [Node; MAX_NODES],
    count: usize,
}

impl NodeState {
    const fn new() -> Self {
        Self {
            storage: [EMPTY_NODE; MAX_NODES],
            count: 0,
        }
    }

    fn find_node_by_id(&self, id: c_int) -> Option<&Node> {
        self.storage[..self.count].iter().find(|node| node.id == id)
    }

    #[allow(dead_code)]
    fn add_node(&mut self, id: c_int, parent_id: c_int, value: f64) -> c_int {
        if self.count >= MAX_NODES {
            return STATUS_ERROR;
        }

        self.storage[self.count] = Node {
            id,
            parent_id,
            value,
            data: [0o100, 0o200, 0o300, 0o400],
        };
        self.count += 1;
        STATUS_OK
    }
}

static NODE_STATE: Mutex<NodeState> = Mutex::new(NodeState::new());

fn node_state() -> MutexGuard<'static, NodeState> {
    NODE_STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn process_backward(array: &[c_int], start_offset: c_int) -> c_int {
    let start = start_offset as usize;
    array[start..]
        .iter()
        .rev()
        .fold(0, |sum, value| sum.wrapping_add(*value))
}

fn compute_size_metric(value: &str) -> c_int {
    (value.len() as c_int).wrapping_mul(2).wrapping_add(0o10)
}

fn safe_double_to_int(mut value: f64) -> c_int {
    if value > 2_147_483_647.0 {
        value = 2_147_483_647.0;
    }
    if value < -2_147_483_648.0 {
        value = -2_147_483_648.0;
    }

    value as c_int
}

#[allow(dead_code)]
fn initialize_test_data(state: &mut NodeState) {
    state.count = 0;

    state.add_node(1, -1, 100.5);
    state.add_node(2, 1, 50.25);
    state.add_node(3, 1, 75.75);
    state.add_node(4, 2, 25.125);
    state.add_node(5, 2, 30.875);
    state.add_node(6, 3, 40.0625);
    state.add_node(7, 4, 12.5);
}

#[unsafe(no_mangle)]
pub extern "C" fn jumpnode(
    operation_mode: c_int,
    node_id: c_int,
    depth: c_int,
    flags: c_int,
) -> c_int {
    match operation_mode {
        0o0001 => {
            let state = node_state();
            let Some(mut current_node) = state.find_node_by_id(node_id) else {
                return STATUS_ERROR | 0o0020;
            };

            let mut accumulated_value = current_node.value;
            let mut i = 0;
            while i < depth && current_node.parent_id != -1 {
                let Some(parent_node) = state.find_node_by_id(current_node.parent_id) else {
                    break;
                };

                accumulated_value += parent_node.value * 1.5;
                current_node = parent_node;
                i += 1;
            }

            safe_double_to_int(accumulated_value)
        }
        0o0002 => {
            let state = node_state();
            let Some(current_node) = state.find_node_by_id(node_id) else {
                return STATUS_ERROR | 0o0040;
            };

            let mut temp_array = [0; 0o20];
            temp_array[..4].copy_from_slice(&current_node.data);
            for (i, value) in temp_array.iter_mut().enumerate().skip(4) {
                *value = (i as c_int).wrapping_mul(0o0007);
            }

            process_backward(&temp_array, depth)
                .wrapping_add((temp_array.len() as c_int).wrapping_mul(flags))
        }
        0o0003 => {
            let buffer = format!("Node_{node_id}_Depth_{depth}");
            compute_size_metric(&buffer).wrapping_add(flags & 0o0177)
        }
        0o0004 => {
            let state = node_state();
            let Some(current_node) = state.find_node_by_id(node_id) else {
                return STATUS_ERROR | 0o0100;
            };

            let mut accumulated_value = 0.0;
            for value in current_node.data {
                accumulated_value += f64::from(value).sqrt() * 2.718281828;
            }
            accumulated_value *= 1.0 + f64::from(depth) * 0.1;

            let mut result = safe_double_to_int(accumulated_value);
            if state.count > 2 {
                let backward_sum = state.storage[..state.count]
                    .iter()
                    .rev()
                    .take(3)
                    .fold(0 as c_int, |sum, node| {
                        sum.wrapping_add(safe_double_to_int(node.value))
                    });
                result = result.wrapping_add(backward_sum);
            }

            result
        }
        _ => STATUS_ERROR | 0o0200,
    }
}
