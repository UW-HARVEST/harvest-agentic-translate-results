use std::os::raw::c_int;

#[derive(Copy, Clone)]
struct Node {
    id: c_int,
    parent_id: c_int,
    value: f64,
    data: [c_int; 4],
}

impl Node {
    const fn empty() -> Self {
        Self {
            id: 0,
            parent_id: 0,
            value: 0.0,
            data: [0; 4],
        }
    }
}

const MAX_NODES: usize = 100;
static mut NODE_STORAGE: [Node; MAX_NODES] = [Node::empty(); MAX_NODES];
static mut NODE_COUNT: usize = 0;

const STATUS_OK: c_int = 0o000;
#[allow(dead_code)]
const STATUS_WARNING: c_int = 0o001;
const STATUS_ERROR: c_int = 0o002;
#[allow(dead_code)]
const STATUS_CRITICAL: c_int = 0o377;

unsafe fn find_node_by_id(id: c_int) -> Option<*mut Node> {
    for i in 0..NODE_COUNT {
        if NODE_STORAGE[i].id == id {
            return Some(&mut NODE_STORAGE[i] as *mut Node);
        }
    }
    None
}

#[allow(dead_code)]
unsafe fn add_node(id: c_int, parent_id: c_int, value: f64) -> c_int {
    if NODE_COUNT >= MAX_NODES {
        return STATUS_ERROR;
    }

    NODE_STORAGE[NODE_COUNT].id = id;
    NODE_STORAGE[NODE_COUNT].parent_id = parent_id;
    NODE_STORAGE[NODE_COUNT].value = value;

    NODE_STORAGE[NODE_COUNT].data[0] = 0o100;
    NODE_STORAGE[NODE_COUNT].data[1] = 0o200;
    NODE_STORAGE[NODE_COUNT].data[2] = 0o300;
    NODE_STORAGE[NODE_COUNT].data[3] = 0o400;

    NODE_COUNT += 1;
    STATUS_OK
}

fn process_backward(array: &[c_int], start_offset: c_int) -> c_int {
    let mut sum = 0;
    let mut ptr = array.len() as isize;
    let start = start_offset as isize;

    while ptr > start {
        ptr -= 1;
        if ptr >= 0 && ptr < array.len() as isize {
            sum += array[ptr as usize];
        }
    }

    sum
}

fn compute_size_metric(str: &str) -> c_int {
    let len = str.len();
    let mut metric = len as c_int;
    metric = metric * 2 + 0o010;
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

#[unsafe(no_mangle)]
pub extern "C" fn jumpnode(operation_mode: c_int, node_id: c_int, depth: c_int, flags: c_int) -> c_int {
    unsafe {
        let mut result = 0;
        let mut accumulated_value: f64;
        let mut temp_array = [0; 20];
        let array_size: usize;

        match operation_mode {
            0o001 => {
                let current_node_ptr = find_node_by_id(node_id);
                if current_node_ptr.is_none() {
                    return STATUS_ERROR | 0o020;
                }
                let mut current_node = current_node_ptr.unwrap();

                accumulated_value = (*current_node).value;

                for _ in 0..depth {
                    if (*current_node).parent_id == -1 {
                        break;
                    }
                    let parent_node_ptr = find_node_by_id((*current_node).parent_id);
                    if parent_node_ptr.is_none() {
                        break;
                    }
                    let parent_node = parent_node_ptr.unwrap();
                    accumulated_value += (*parent_node).value * 1.5;
                    current_node = parent_node;
                }

                result = safe_double_to_int(accumulated_value);
            }
            0o002 => {
                let current_node_ptr = find_node_by_id(node_id);
                if current_node_ptr.is_none() {
                    return STATUS_ERROR | 0o040;
                }
                let current_node = current_node_ptr.unwrap();

                for i in 0..4 {
                    temp_array[i] = (*current_node).data[i];
                }

                for i in 4..0o020 {
                    temp_array[i] = (i as c_int) * 0o007;
                }

                array_size = 0o020;

                result = process_backward(&temp_array[..array_size], depth);

                result += (array_size as c_int) * flags;
            }
            0o003 => {
                let buffer = format!("Node_{}_Depth_{}", node_id, depth);
                result = compute_size_metric(&buffer);
                result += flags & 0o177;
            }
            0o004 => {
                let current_node_ptr = find_node_by_id(node_id);
                if current_node_ptr.is_none() {
                    return STATUS_ERROR | 0o100;
                }
                let current_node = current_node_ptr.unwrap();

                accumulated_value = 0.0;
                for i in 0..4 {
                    accumulated_value += ((*current_node).data[i] as f64).sqrt() * 2.718281828;
                }

                accumulated_value *= 1.0 + (depth as f64) * 0.1;

                result = safe_double_to_int(accumulated_value);

                if NODE_COUNT > 2 {
                    let mut backward_sum = 0;
                    let mut iter_idx = NODE_COUNT;

                    for _ in 0..3 {
                        if iter_idx > 0 {
                            iter_idx -= 1;
                            backward_sum += safe_double_to_int(NODE_STORAGE[iter_idx].value);
                        } else {
                            break;
                        }
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
}

#[allow(dead_code)]
unsafe fn initialize_test_data() {
    NODE_COUNT = 0;

    add_node(1, -1, 100.5);
    add_node(2, 1, 50.25);
    add_node(3, 1, 75.75);
    add_node(4, 2, 25.125);
    add_node(5, 2, 30.875);
    add_node(6, 3, 40.0625);
    add_node(7, 4, 12.5);
}
