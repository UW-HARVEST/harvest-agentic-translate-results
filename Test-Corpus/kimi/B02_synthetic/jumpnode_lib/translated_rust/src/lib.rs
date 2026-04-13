use std::ffi::{c_char, c_int, c_double};
use std::os::raw::c_size_t;
use std::sync::Mutex;

const MAX_NODES: usize = 100;

const STATUS_OK: c_int = 0o0000;
const STATUS_WARNING: c_int = 0o0001;
const STATUS_ERROR: c_int = 0o0002;
const STATUS_CRITICAL: c_int = 0o0377;

struct Node {
    id: c_int,
    parent_id: c_int,
    value: c_double,
    data: [c_int; 4],
}

struct NodeStorage {
    nodes: Vec<Node>,
}

static NODE_STORAGE: Mutex<NodeStorage> = Mutex::new(NodeStorage {
    nodes: Vec::new(),
});

fn find_node_by_id(id: c_int) -> Option<usize> {
    let storage = NODE_STORAGE.lock().unwrap();
    storage.nodes.iter().position(|n| n.id == id)
}

fn add_node(id: c_int, parent_id: c_int, value: c_double) -> c_int {
    let mut storage = NODE_STORAGE.lock().unwrap();
    if storage.nodes.len() >= MAX_NODES {
        return STATUS_ERROR;
    }
    
    let node = Node {
        id,
        parent_id,
        value,
        data: [0o0100, 0o0200, 0o0300, 0o0400],
    };
    storage.nodes.push(node);
    STATUS_OK
}

fn process_backward(array: &[c_int], start_offset: usize) -> c_int {
    let mut sum: c_int = 0;
    let start = start_offset;
    
    for i in (start..array.len()).rev() {
        sum = sum.wrapping_add(array[i]);
    }
    
    sum
}

fn compute_size_metric(s: &str) -> c_int {
    let len = s.len();
    let mut metric = len as c_int;
    metric = metric.wrapping_mul(2).wrapping_add(0o010);
    metric
}

fn safe_double_to_int(value: c_double) -> c_int {
    let clamped = if value > 2147483647.0 {
        2147483647.0
    } else if value < -2147483648.0 {
        -2147483648.0
    } else {
        value
    };
    clamped as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn jumpnode(operation_mode: c_int, node_id: c_int, depth: c_int, flags: c_int) -> c_int {
    let mut result: c_int = 0;
    
    match operation_mode {
        0o0001 => {
            let idx = match find_node_by_id(node_id) {
                Some(i) => i,
                None => return STATUS_ERROR | 0o0020,
            };
            
            let storage = NODE_STORAGE.lock().unwrap();
            let mut current_idx = idx;
            let mut accumulated_value = storage.nodes[current_idx].value;
            
            for _ in 0..depth {
                let parent_id = storage.nodes[current_idx].parent_id;
                if parent_id == -1 {
                    break;
                }
                drop(storage);
                let parent_idx = match find_node_by_id(parent_id) {
                    Some(i) => i,
                    None => break,
                };
                let storage = NODE_STORAGE.lock().unwrap();
                accumulated_value += storage.nodes[parent_idx].value * 1.5;
                current_idx = parent_idx;
            }
            
            result = safe_double_to_int(accumulated_value);
        }
        
        0o0002 => {
            let idx = match find_node_by_id(node_id) {
                Some(i) => i,
                None => return STATUS_ERROR | 0o0040,
            };
            
            let storage = NODE_STORAGE.lock().unwrap();
            let mut temp_array: [c_int; 20] = [0; 20];
            
            for i in 0..4 {
                temp_array[i] = storage.nodes[idx].data[i];
            }
            
            for i in 4..20 {
                temp_array[i] = (i as c_int).wrapping_mul(0o0007);
            }
            
            let array_size: usize = 0o020;
            
            result = process_backward(&temp_array[..array_size], depth as usize);
            result = result.wrapping_add((array_size as c_int).wrapping_mul(flags));
        }
        
        0o0003 => {
            let buffer = format!("Node_{}_Depth_{}", node_id, depth);
            result = compute_size_metric(&buffer);
            result = result.wrapping_add(flags & 0o0177);
        }
        
        0o0004 => {
            let idx = match find_node_by_id(node_id) {
                Some(i) => i,
                None => return STATUS_ERROR | 0o0100,
            };
            
            let storage = NODE_STORAGE.lock().unwrap();
            let mut accumulated_value: c_double = 0.0;
            
            for i in 0..4 {
                let val = storage.nodes[idx].data[i] as c_double;
                accumulated_value += val.sqrt() * 2.718281828;
            }
            
            accumulated_value *= 1.0 + (depth as c_double) * 0.1;
            result = safe_double_to_int(accumulated_value);
            
            let node_count = storage.nodes.len();
            if node_count > 2 {
                let mut backward_sum: c_int = 0;
                let start = if node_count > 3 { node_count - 3 } else { 0 };
                
                for i in start..node_count {
                    backward_sum = backward_sum.wrapping_add(safe_double_to_int(storage.nodes[i].value));
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

fn initialize_test_data() {
    let mut storage = NODE_STORAGE.lock().unwrap();
    storage.nodes.clear();
    drop(storage);
    
    add_node(1, -1, 100.5);
    add_node(2, 1, 50.25);
    add_node(3, 1, 75.75);
    add_node(4, 2, 25.125);
    add_node(5, 2, 30.875);
    add_node(6, 3, 40.0625);
    add_node(7, 4, 12.5);
}
