use std::ffi::c_int;
use std::sync::{Mutex, OnceLock};

const MAX_NODES: usize = 100;
const STATUS_OK: c_int = 0o000;
const STATUS_WARNING: c_int = 0o001;
const STATUS_ERROR: c_int = 0o002;
const STATUS_CRITICAL: c_int = 0o377;

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct Node {
    id: c_int,
    parent_id: c_int,
    value: f64,
    data: [c_int; 4],
}

#[derive(Clone)]
struct State {
    node_storage: [Node; MAX_NODES],
    node_count: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            node_storage: [Node::default(); MAX_NODES],
            node_count: 0,
        }
    }
}

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(State::default()))
}

fn find_node_index_by_id(state: &State, id: c_int) -> Option<usize> {
    let mut i = 0usize;
    while i < state.node_count {
        if state.node_storage[i].id == id {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn add_node(state: &mut State, id: c_int, parent_id: c_int, value: f64) -> c_int {
    if state.node_count >= MAX_NODES {
        return STATUS_ERROR;
    }

    let slot = &mut state.node_storage[state.node_count];
    slot.id = id;
    slot.parent_id = parent_id;
    slot.value = value;
    slot.data[0] = 0o100;
    slot.data[1] = 0o200;
    slot.data[2] = 0o300;
    slot.data[3] = 0o400;

    state.node_count += 1;
    STATUS_OK
}

fn process_backward(array: &[c_int], size: usize, start_offset: c_int) -> c_int {
    let mut sum: c_int = 0;
    let mut ptr = size;
    let start = array.as_ptr().wrapping_offset(start_offset as isize);

    while ptr > 0 && array.as_ptr().wrapping_add(ptr) > start {
        ptr -= 1;
        sum += array[ptr];
    }

    sum
}

fn compute_size_metric(s: &str) -> c_int {
    let len = s.len();
    let mut metric = len as c_int;
    metric = metric * 2 + 0o10;
    metric
}

fn safe_double_to_int(mut value: f64) -> c_int {
    if value > 2147483647.0 {
        value = 2147483647.0;
    }
    if value < -2147483648.0 {
        value = -2147483648.0;
    }
    value as c_int
}

fn initialize_test_data(state: &mut State) {
    state.node_count = 0;

    let _ = add_node(state, 1, -1, 100.5);
    let _ = add_node(state, 2, 1, 50.25);
    let _ = add_node(state, 3, 1, 75.75);
    let _ = add_node(state, 4, 2, 25.125);
    let _ = add_node(state, 5, 2, 30.875);
    let _ = add_node(state, 6, 3, 40.0625);
    let _ = add_node(state, 7, 4, 12.5);
}

#[unsafe(no_mangle)]
pub extern "C" fn jumpnode(
    operation_mode: c_int,
    node_id: c_int,
    depth: c_int,
    flags: c_int,
) -> c_int {
    let mut temp_array = [0 as c_int; 20];
    let array_size: usize;
    let state = state().lock().unwrap_or_else(|poisoned| poisoned.into_inner());

    match operation_mode {
        0o001 => {
            let Some(mut current_index) = find_node_index_by_id(&state, node_id) else {
                return STATUS_ERROR | 0o020;
            };

            let mut accumulated_value = state.node_storage[current_index].value;
            let mut i: c_int = 0;
            while i < depth && state.node_storage[current_index].parent_id != -1 {
                let parent_id = state.node_storage[current_index].parent_id;
                let Some(parent_index) = find_node_index_by_id(&state, parent_id) else {
                    break;
                };

                accumulated_value += state.node_storage[parent_index].value * 1.5;
                current_index = parent_index;
                i += 1;
            }

            safe_double_to_int(accumulated_value)
        }
        0o002 => {
            let Some(current_index) = find_node_index_by_id(&state, node_id) else {
                return STATUS_ERROR | 0o040;
            };

            let mut i = 0usize;
            while i < 4 {
                temp_array[i] = state.node_storage[current_index].data[i];
                i += 1;
            }

            i = 4;
            while i < 0o20 {
                temp_array[i] = (i as c_int) * 0o007;
                i += 1;
            }

            array_size = 0o20;
            let mut result = process_backward(&temp_array, array_size, depth);
            result += (array_size as c_int) * flags;
            result
        }
        0o003 => {
            let buffer = format!("Node_{node_id}_Depth_{depth}");
            let mut result = compute_size_metric(&buffer);
            result += flags & 0o177;
            result
        }
        0o004 => {
            let Some(current_index) = find_node_index_by_id(&state, node_id) else {
                return STATUS_ERROR | 0o100;
            };

            let mut accumulated_value = 0.0f64;
            let mut i = 0usize;
            while i < 4 {
                accumulated_value += (state.node_storage[current_index].data[i] as f64).sqrt()
                    * 2.718281828;
                i += 1;
            }

            accumulated_value *= 1.0 + (depth as f64) * 0.1;
            let mut result = safe_double_to_int(accumulated_value);

            if state.node_count > 2 {
                let mut iter = state.node_count;
                let mut backward_sum: c_int = 0;
                let mut j = 0usize;

                while j < 3 && iter > 0 {
                    iter -= 1;
                    backward_sum += safe_double_to_int(state.node_storage[iter].value);
                    j += 1;
                }

                result += backward_sum;
            }
            result
        }
        _ => STATUS_ERROR | 0o200,
    }
}

#[allow(dead_code)]
fn _keep_c_statics_linked() -> (c_int, c_int) {
    let _ = STATUS_WARNING;
    let _ = STATUS_CRITICAL;
    let _ = initialize_test_data as fn(&mut State);
    (STATUS_WARNING, STATUS_CRITICAL)
}
