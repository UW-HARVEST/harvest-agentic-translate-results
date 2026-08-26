use std::fmt::Write;
use std::os::raw::c_int;

#[repr(C)]
struct Node {
    id: c_int,
    parent_id: c_int,
    value: f64,
    data: [c_int; 4],
}

const MAX_NODES: usize = 100;

static mut NODE_STORAGE: [Node; MAX_NODES] = {
    const INIT: Node = Node { id: 0, parent_id: 0, value: 0.0, data: [0; 4] };
    [INIT; MAX_NODES]
};
static mut NODE_COUNT: usize = 0;

const STATUS_OK: c_int = 0;
const STATUS_ERROR: c_int = 2;

unsafe fn find_node_by_id(id: c_int) -> *mut Node {
    for i in 0..NODE_COUNT {
        if NODE_STORAGE[i].id == id {
            return &mut NODE_STORAGE[i] as *mut Node;
        }
    }
    std::ptr::null_mut()
}

unsafe fn add_node(id: c_int, parent_id: c_int, value: f64) -> c_int {
    if NODE_COUNT >= MAX_NODES {
        return STATUS_ERROR;
    }
    NODE_STORAGE[NODE_COUNT].id = id;
    NODE_STORAGE[NODE_COUNT].parent_id = parent_id;
    NODE_STORAGE[NODE_COUNT].value = value;
    NODE_STORAGE[NODE_COUNT].data[0] = 0o100; // 64
    NODE_STORAGE[NODE_COUNT].data[1] = 0o200; // 128
    NODE_STORAGE[NODE_COUNT].data[2] = 0o300; // 192
    NODE_STORAGE[NODE_COUNT].data[3] = 0o400; // 256
    NODE_COUNT += 1;
    STATUS_OK
}

fn process_backward(array: &[c_int], size: usize, start_offset: usize) -> c_int {
    let mut sum: c_int = 0;
    let mut ptr = size;
    let start = start_offset;
    while ptr > start {
        ptr -= 1;
        sum = sum.wrapping_add(array[ptr]);
    }
    sum
}

fn compute_size_metric(s: &str) -> c_int {
    let len = s.len() as c_int;
    len * 2 + 0o10 // 0o10 = 8
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
        let mut result: c_int = 0;

        match operation_mode {
            1 => {
                let mut current_node = find_node_by_id(node_id);
                if current_node.is_null() {
                    return STATUS_ERROR | 0o20;
                }
                let mut accumulated_value = (*current_node).value;
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
            2 => {
                let current_node = find_node_by_id(node_id);
                if current_node.is_null() {
                    return STATUS_ERROR | 0o40;
                }
                let mut temp_array = [0i32; 20];
                for i in 0..4 {
                    temp_array[i] = (*current_node).data[i];
                }
                for i in 4..16 {
                    temp_array[i] = (i as c_int) * 7;
                }
                let array_size: usize = 16; // 0o20 = 16
                result = process_backward(&temp_array, array_size, depth as usize);
                result += (array_size as c_int) * flags;
            }
            3 => {
                let mut buffer = String::new();
                let _ = write!(buffer, "Node_{}_Depth_{}", node_id, depth);
                result = compute_size_metric(&buffer);
                result += flags & 0o177; // 127
            }
            4 => {
                let current_node = find_node_by_id(node_id);
                if current_node.is_null() {
                    return STATUS_ERROR | 0o100;
                }
                let mut accumulated_value: f64 = 0.0;
                for i in 0..4 {
                    accumulated_value += ((*current_node).data[i] as f64).sqrt() * 2.718281828;
                }
                accumulated_value *= 1.0 + depth as f64 * 0.1;
                result = safe_double_to_int(accumulated_value);

                if NODE_COUNT > 2 {
                    let mut backward_sum: c_int = 0;
                    let mut idx = NODE_COUNT;
                    let mut i = 0;
                    while i < 3 && idx > 0 {
                        idx -= 1;
                        backward_sum += safe_double_to_int(NODE_STORAGE[idx].value);
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
}
