use std::fmt::Write;

#[repr(C)]
#[derive(Clone, Copy)]
struct Node {
    id: i32,
    parent_id: i32,
    value: f64,
    data: [i32; 4],
}

const MAX_NODES: usize = 100;

static mut NODE_STORAGE: [Node; MAX_NODES] = [Node {
    id: 0,
    parent_id: 0,
    value: 0.0,
    data: [0; 4],
}; MAX_NODES];
static mut NODE_COUNT: usize = 0;

const STATUS_OK: i32 = 0o0000;
const STATUS_ERROR: i32 = 0o0002;

fn find_node_by_id(id: i32) -> Option<usize> {
    unsafe {
        for i in 0..NODE_COUNT {
            if NODE_STORAGE[i].id == id {
                return Some(i);
            }
        }
    }
    None
}

fn add_node(id: i32, parent_id: i32, value: f64) -> i32 {
    unsafe {
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
    }
    STATUS_OK
}

fn process_backward(array: &[i32], start_offset: usize) -> i32 {
    let mut sum: i32 = 0;
    let size = array.len();
    let mut ptr = size;
    let start = start_offset;
    while ptr > start {
        ptr -= 1;
        sum += array[ptr];
    }
    sum
}

fn compute_size_metric(s: &str) -> i32 {
    let len = s.len() as i32;
    let metric = len * 2 + 0o10;
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
pub extern "C" fn jumpnode(operation_mode: i32, node_id: i32, depth: i32, flags: i32) -> i32 {
    let result: i32;

    match operation_mode {
        0o0001 => {
            let idx = match find_node_by_id(node_id) {
                Some(i) => i,
                None => return STATUS_ERROR | 0o0020,
            };
            unsafe {
                let mut accumulated_value = NODE_STORAGE[idx].value;
                let mut cur = idx;
                let mut i = 0;
                while i < depth && NODE_STORAGE[cur].parent_id != -1 {
                    match find_node_by_id(NODE_STORAGE[cur].parent_id) {
                        Some(p) => {
                            accumulated_value += NODE_STORAGE[p].value * 1.5;
                            cur = p;
                        }
                        None => break,
                    }
                    i += 1;
                }
                result = safe_double_to_int(accumulated_value);
            }
        }
        0o0002 => {
            let idx = match find_node_by_id(node_id) {
                Some(i) => i,
                None => return STATUS_ERROR | 0o0040,
            };
            unsafe {
                let mut temp_array = [0i32; 20];
                for i in 0..4 {
                    temp_array[i] = NODE_STORAGE[idx].data[i];
                }
                for i in 4..0o020 {
                    temp_array[i] = (i as i32) * 0o0007;
                }
                let array_size: usize = 0o020;
                let mut r = process_backward(&temp_array[..array_size], depth as usize);
                r += (array_size as i32) * flags;
                result = r;
            }
        }
        0o0003 => {
            let mut buffer = String::new();
            let _ = write!(buffer, "Node_{}_Depth_{}", node_id, depth);
            let mut r = compute_size_metric(&buffer);
            r += flags & 0o0177;
            result = r;
        }
        0o0004 => {
            let idx = match find_node_by_id(node_id) {
                Some(i) => i,
                None => return STATUS_ERROR | 0o0100,
            };
            unsafe {
                let mut accumulated_value: f64 = 0.0;
                for i in 0..4 {
                    accumulated_value +=
                        (NODE_STORAGE[idx].data[i] as f64).sqrt() * 2.718281828;
                }
                accumulated_value *= 1.0 + depth as f64 * 0.1;
                let mut r = safe_double_to_int(accumulated_value);

                if NODE_COUNT > 2 {
                    let mut backward_sum: i32 = 0;
                    let mut iter = NODE_COUNT;
                    let mut i = 0;
                    while i < 3 && iter > 0 {
                        iter -= 1;
                        backward_sum += safe_double_to_int(NODE_STORAGE[iter].value);
                        i += 1;
                    }
                    r += backward_sum;
                }
                result = r;
            }
        }
        _ => {
            result = STATUS_ERROR | 0o0200;
        }
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn initialize_test_data() {
    unsafe {
        NODE_COUNT = 0;
    }
    add_node(1, -1, 100.5);
    add_node(2, 1, 50.25);
    add_node(3, 1, 75.75);
    add_node(4, 2, 25.125);
    add_node(5, 2, 30.875);
    add_node(6, 3, 40.0625);
    add_node(7, 4, 12.5);
}
