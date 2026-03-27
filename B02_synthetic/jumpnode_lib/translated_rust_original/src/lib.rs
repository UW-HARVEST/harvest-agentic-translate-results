use std::fmt::Write as _;

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
static mut NODE_COUNT: i32 = 0;

const STATUS_OK: i32 = 0o0000;
const STATUS_ERROR: i32 = 0o0002;

unsafe fn find_node_by_id(id: i32) -> *mut Node {
    for i in 0..NODE_COUNT as usize {
        if NODE_STORAGE[i].id == id {
            return &mut NODE_STORAGE[i] as *mut Node;
        }
    }
    std::ptr::null_mut()
}

unsafe fn add_node(id: i32, parent_id: i32, value: f64) -> i32 {
    if NODE_COUNT >= MAX_NODES as i32 {
        return STATUS_ERROR;
    }
    let nc = NODE_COUNT as usize;
    NODE_STORAGE[nc].id = id;
    NODE_STORAGE[nc].parent_id = parent_id;
    NODE_STORAGE[nc].value = value;
    NODE_STORAGE[nc].data[0] = 0o100;
    NODE_STORAGE[nc].data[1] = 0o200;
    NODE_STORAGE[nc].data[2] = 0o300;
    NODE_STORAGE[nc].data[3] = 0o400;
    NODE_COUNT += 1;
    STATUS_OK
}

fn process_backward(array: &[i32], size: usize, start_offset: i32) -> i32 {
    let mut sum: i32 = 0;
    let mut ptr = size;
    let start = start_offset as usize;
    while ptr > start {
        ptr -= 1;
        sum += array[ptr];
    }
    sum
}

fn compute_size_metric(s: &str) -> i32 {
    let metric = s.len() as i32;
    metric * 2 + 0o10
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
pub unsafe extern "C" fn jumpnode(
    operation_mode: i32,
    node_id: i32,
    depth: i32,
    flags: i32,
) -> i32 {
    let mut result: i32 = 0;
    let mut accumulated_value: f64;
    let mut temp_array: [i32; 20] = [0; 20];

    match operation_mode {
        0o001 => {
            let mut current_node = find_node_by_id(node_id);
            if current_node.is_null() {
                return STATUS_ERROR | 0o20;
            }
            accumulated_value = (*current_node).value;
            let mut i = 0;
            while i < depth && (*current_node).parent_id != -1 {
                let parent_node = find_node_by_id((*current_node).parent_id);
                if parent_node.is_null() {
                    break;
                }
                accumulated_value += (*parent_node).value * 1.5;
                current_node = parent_node;
                i += 1;
            }
            result = safe_double_to_int(accumulated_value);
        }
        0o002 => {
            let current_node = find_node_by_id(node_id);
            if current_node.is_null() {
                return STATUS_ERROR | 0o40;
            }
            for i in 0..4 {
                temp_array[i] = (*current_node).data[i];
            }
            for i in 4..0o20usize {
                temp_array[i] = i as i32 * 0o7;
            }
            let array_size: usize = 0o20;
            result = process_backward(&temp_array, array_size, depth);
            result += array_size as i32 * flags;
        }
        0o003 => {
            let mut buffer = String::new();
            let _ = write!(buffer, "Node_{}_Depth_{}", node_id, depth);
            result = compute_size_metric(&buffer);
            result += flags & 0o177;
        }
        0o004 => {
            let current_node = find_node_by_id(node_id);
            if current_node.is_null() {
                return STATUS_ERROR | 0o100;
            }
            accumulated_value = 0.0;
            for i in 0..4 {
                accumulated_value +=
                    ((*current_node).data[i] as f64).sqrt() * 2.718281828;
            }
            accumulated_value *= 1.0 + depth as f64 * 0.1;
            result = safe_double_to_int(accumulated_value);

            if NODE_COUNT > 2 {
                let end = NODE_COUNT as usize;
                let mut iter = end;
                let mut backward_sum: i32 = 0;
                let mut i = 0;
                while i < 3 && iter > 0 {
                    iter -= 1;
                    backward_sum += safe_double_to_int(NODE_STORAGE[iter].value);
                    i += 1;
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
