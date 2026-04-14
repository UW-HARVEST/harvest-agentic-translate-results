use std::os::raw::c_int;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy)]
struct Node {
    id: c_int,
    parent_id: c_int,
    value: f64,
    data: [c_int; 4],
}

const MAX_NODES: usize = 100;
const STATUS_OK: c_int = 0o000;
const STATUS_WARNING: c_int = 0o001;
const STATUS_ERROR: c_int = 0o002;
const STATUS_CRITICAL: c_int = 0o377;

struct State {
    node_storage: Vec<Node>,
}

impl State {
    fn new() -> Self {
        Self { node_storage: Vec::with_capacity(MAX_NODES) }
    }

    fn node_count(&self) -> usize {
        self.node_storage.len()
    }

    fn find_node_index_by_id(&self, id: c_int) -> Option<usize> {
        self.node_storage.iter().position(|n| n.id == id)
    }

    fn add_node(&mut self, id: c_int, parent_id: c_int, value: f64) -> c_int {
        if self.node_storage.len() >= MAX_NODES {
            return STATUS_ERROR;
        }

        self.node_storage.push(Node {
            id,
            parent_id,
            value,
            data: [0o100, 0o200, 0o300, 0o400],
        });

        STATUS_OK
    }

    fn initialize_test_data(&mut self) {
        self.node_storage.clear();
        self.add_node(1, -1, 100.5);
        self.add_node(2, 1, 50.25);
        self.add_node(3, 1, 75.75);
        self.add_node(4, 2, 25.125);
        self.add_node(5, 2, 30.875);
        self.add_node(6, 3, 40.0625);
        self.add_node(7, 4, 12.5);
    }
}

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| {
        let mut s = State::new();
        s.initialize_test_data();
        Mutex::new(s)
    })
}

fn process_backward(array: &[c_int], start_offset: c_int) -> c_int {
    let start = if start_offset <= 0 {
        0
    } else {
        usize::try_from(start_offset).unwrap_or(usize::MAX).min(array.len())
    };

    array[start..].iter().rev().copied().sum()
}

fn compute_size_metric(s: &str) -> c_int {
    let len = s.len();
    let mut metric = len as c_int;
    metric = metric * 2 + 0o10;
    metric
}

fn safe_double_to_int(value: f64) -> c_int {
    let clamped = value.clamp(i32::MIN as f64, i32::MAX as f64);
    clamped as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn jumpnode(operation_mode: c_int, node_id: c_int, depth: c_int, flags: c_int) -> c_int {
    let _ = STATUS_WARNING;
    let _ = STATUS_CRITICAL;

    let mut result: c_int = 0;
    let mut state = state().lock().unwrap();

    match operation_mode {
        0o001 => {
            let Some(mut current_index) = state.find_node_index_by_id(node_id) else {
                return STATUS_ERROR | 0o020;
            };

            let mut accumulated_value = state.node_storage[current_index].value;

            let mut i = 0;
            while i < depth && state.node_storage[current_index].parent_id != -1 {
                let parent_id = state.node_storage[current_index].parent_id;
                let Some(parent_index) = state.find_node_index_by_id(parent_id) else {
                    break;
                };
                accumulated_value += state.node_storage[parent_index].value * 1.5;
                current_index = parent_index;
                i += 1;
            }

            result = safe_double_to_int(accumulated_value);
        }
        0o002 => {
            let Some(current_index) = state.find_node_index_by_id(node_id) else {
                return STATUS_ERROR | 0o040;
            };

            let mut temp_array = [0i32; 20];
            temp_array[..4].copy_from_slice(&state.node_storage[current_index].data);
            for (i, item) in temp_array.iter_mut().enumerate().skip(4) {
                *item = (i as c_int) * 0o007;
            }

            let array_size: usize = 0o20;
            result = process_backward(&temp_array[..array_size], depth);
            result += (array_size as c_int) * flags;
        }
        0o003 => {
            let buffer = format!("Node_{}_Depth_{}", node_id, depth);
            result = compute_size_metric(&buffer);
            result += flags & 0o177;
        }
        0o004 => {
            let Some(current_index) = state.find_node_index_by_id(node_id) else {
                return STATUS_ERROR | 0o100;
            };

            let mut accumulated_value = 0.0f64;
            for value in state.node_storage[current_index].data {
                accumulated_value += (value as f64).sqrt() * 2.718281828f64;
            }

            accumulated_value *= 1.0 + (depth as f64) * 0.1;
            result = safe_double_to_int(accumulated_value);

            if state.node_count() > 2 {
                let mut backward_sum = 0;
                for node in state.node_storage.iter().rev().take(3) {
                    backward_sum += safe_double_to_int(node.value);
                }
                result += backward_sum;
            }
        }
        _ => {
            result = STATUS_ERROR | 0o200;
        }
    }

    result
}
