// Rust translation of c_src/src/lib.c — produces byte-identical output.

use std::ffi::c_int;
use std::sync::Mutex;

#[derive(Clone, Copy)]
struct Node {
    id: i32,
    parent_id: i32,
    value: f64,
    data: [i32; 4],
}

impl Node {
    const fn zero() -> Self {
        Node {
            id: 0,
            parent_id: 0,
            value: 0.0,
            data: [0; 4],
        }
    }
}

const MAX_NODES: usize = 100;

struct State {
    node_storage: [Node; MAX_NODES],
    node_count: usize,
}

impl State {
    const fn new() -> Self {
        State {
            node_storage: [Node::zero(); MAX_NODES],
            node_count: 0,
        }
    }
}

static STATE: Mutex<State> = Mutex::new(State::new());

const STATUS_OK: i32 = 0o0000;
#[allow(dead_code)]
const STATUS_WARNING: i32 = 0o0001;
const STATUS_ERROR: i32 = 0o0002;
#[allow(dead_code)]
const STATUS_CRITICAL: i32 = 0o0377;

fn find_node_by_id(state: &State, id: i32) -> Option<usize> {
    for i in 0..state.node_count {
        if state.node_storage[i].id == id {
            return Some(i);
        }
    }
    None
}

#[allow(dead_code)]
fn add_node(state: &mut State, id: i32, parent_id: i32, value: f64) -> i32 {
    if state.node_count >= MAX_NODES {
        return STATUS_ERROR;
    }

    let idx = state.node_count;
    state.node_storage[idx].id = id;
    state.node_storage[idx].parent_id = parent_id;
    state.node_storage[idx].value = value;

    state.node_storage[idx].data[0] = 0o0100;
    state.node_storage[idx].data[1] = 0o0200;
    state.node_storage[idx].data[2] = 0o0300;
    state.node_storage[idx].data[3] = 0o0400;

    state.node_count += 1;
    STATUS_OK
}

fn process_backward(array: &[i32], size: usize, start_offset: usize) -> i32 {
    let mut sum: i32 = 0;
    let mut ptr = size;
    let start = start_offset;

    while ptr > start {
        ptr -= 1;
        // Wrap on add to mirror C signed-int behavior on overflow.
        sum = sum.wrapping_add(array[ptr]);
    }

    sum
}

fn compute_size_metric(s: &[u8]) -> i32 {
    // strlen: bytes until first NUL.
    let mut len: usize = 0;
    while len < s.len() && s[len] != 0 {
        len += 1;
    }

    let mut metric: i32 = len as i32;
    metric = metric.wrapping_mul(2).wrapping_add(0o010);
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

#[unsafe(no_mangle)]
pub extern "C" fn jumpnode(
    operation_mode: c_int,
    node_id: c_int,
    depth: c_int,
    flags: c_int,
) -> c_int {
    let state = STATE.lock().unwrap();
    #[allow(unused_assignments)]
    let mut result: i32 = 0;

    match operation_mode {
        0o0001 => {
            let mut cur_idx = match find_node_by_id(&state, node_id) {
                Some(i) => i,
                None => return STATUS_ERROR | 0o0020,
            };

            let mut accumulated_value: f64 = state.node_storage[cur_idx].value;

            let mut i = 0;
            while i < depth && state.node_storage[cur_idx].parent_id != -1 {
                let parent_idx =
                    match find_node_by_id(&state, state.node_storage[cur_idx].parent_id) {
                        Some(p) => p,
                        None => break,
                    };

                accumulated_value += state.node_storage[parent_idx].value * 1.5;
                cur_idx = parent_idx;
                i += 1;
            }

            result = safe_double_to_int(accumulated_value);
        }

        0o0002 => {
            let cur_idx = match find_node_by_id(&state, node_id) {
                Some(i) => i,
                None => return STATUS_ERROR | 0o0040,
            };

            let mut temp_array: [i32; 20] = [0; 20];
            for i in 0..4 {
                temp_array[i] = state.node_storage[cur_idx].data[i];
            }
            for i in 4..0o020usize {
                temp_array[i] = (i as i32).wrapping_mul(0o0007);
            }

            let array_size: usize = 0o020;

            result = process_backward(&temp_array, array_size, depth as usize);
            result = result.wrapping_add((array_size as i32).wrapping_mul(flags));
        }

        0o0003 => {
            // sprintf(buffer, "Node_%d_Depth_%d", node_id, depth)
            let formatted = format!("Node_{}_Depth_{}", node_id, depth);
            let mut buffer = [0u8; 50];
            let bytes = formatted.as_bytes();
            // C sprintf writes the bytes followed by a NUL. If the formatted output
            // would overflow `buffer` the original C has UB; we mirror by truncating
            // (this case never occurs for reasonable int values on typical platforms,
            // and the tests do not exercise it).
            let n = bytes.len().min(buffer.len() - 1);
            buffer[..n].copy_from_slice(&bytes[..n]);
            // buffer[n] stays 0 (NUL terminator).

            result = compute_size_metric(&buffer);
            result = result.wrapping_add(flags & 0o0177);
        }

        0o0004 => {
            let cur_idx = match find_node_by_id(&state, node_id) {
                Some(i) => i,
                None => return STATUS_ERROR | 0o0100,
            };

            let mut accumulated_value: f64 = 0.0;
            for i in 0..4 {
                accumulated_value +=
                    (state.node_storage[cur_idx].data[i] as f64).sqrt() * 2.718281828;
            }

            accumulated_value *= 1.0 + (depth as f64) * 0.1;

            result = safe_double_to_int(accumulated_value);

            if state.node_count > 2 {
                // Iterate backward over the last three nodes from the end.
                let mut iter = state.node_count;
                let mut backward_sum: i32 = 0;
                let mut i = 0;
                while i < 3 && iter > 0 {
                    iter -= 1;
                    backward_sum = backward_sum
                        .wrapping_add(safe_double_to_int(state.node_storage[iter].value));
                    i += 1;
                }
                result = result.wrapping_add(backward_sum);
            }
        }

        _ => {
            result = STATUS_ERROR | 0o0200;
        }
    }

    result
}
