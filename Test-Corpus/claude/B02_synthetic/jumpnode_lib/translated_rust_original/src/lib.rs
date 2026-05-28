// Rust translation of c_src/src/lib.c
// Preserves exact C semantics including byte-identical output.

use std::ffi::c_int;

#[derive(Copy, Clone)]
#[repr(C)]
struct Node {
    id: c_int,
    parent_id: c_int,
    value: f64,
    data: [c_int; 4],
}

const MAX_NODES: usize = 100;

const STATUS_OK: c_int = 0o0000;
#[allow(dead_code)]
const STATUS_WARNING: c_int = 0o0001;
const STATUS_ERROR: c_int = 0o0002;
#[allow(dead_code)]
const STATUS_CRITICAL: c_int = 0o0377;

static mut NODE_STORAGE: [Node; MAX_NODES] = [Node {
    id: 0,
    parent_id: 0,
    value: 0.0,
    data: [0; 4],
}; MAX_NODES];

static mut NODE_COUNT: c_int = 0;

fn find_node_by_id(id: c_int) -> *mut Node {
    unsafe {
        let count = NODE_COUNT;
        let storage_ptr = std::ptr::addr_of_mut!(NODE_STORAGE) as *mut Node;
        let mut i: c_int = 0;
        while i < count {
            let node_ptr = storage_ptr.add(i as usize);
            if (*node_ptr).id == id {
                return node_ptr;
            }
            i += 1;
        }
        std::ptr::null_mut()
    }
}

#[allow(dead_code)]
fn add_node(id: c_int, parent_id: c_int, value: f64) -> c_int {
    unsafe {
        if NODE_COUNT >= MAX_NODES as c_int {
            return STATUS_ERROR;
        }

        let idx = NODE_COUNT as usize;
        let storage_ptr = std::ptr::addr_of_mut!(NODE_STORAGE) as *mut Node;
        let node_ptr = storage_ptr.add(idx);

        (*node_ptr).id = id;
        (*node_ptr).parent_id = parent_id;
        (*node_ptr).value = value;

        (*node_ptr).data[0] = 0o0100;
        (*node_ptr).data[1] = 0o0200;
        (*node_ptr).data[2] = 0o0300;
        (*node_ptr).data[3] = 0o0400;

        NODE_COUNT += 1;
        STATUS_OK
    }
}

fn process_backward(array: *mut c_int, size: usize, start_offset: c_int) -> c_int {
    let mut sum: c_int = 0;
    unsafe {
        let mut ptr = array.add(size);
        let start = array.offset(start_offset as isize);

        while ptr > start {
            ptr = ptr.offset(-1);
            sum = sum.wrapping_add(*ptr);
        }
    }
    sum
}

fn compute_size_metric(s: &[u8]) -> c_int {
    // strlen: bytes up to (but not including) the first NUL.
    let mut len: usize = 0;
    while len < s.len() && s[len] != 0 {
        len += 1;
    }
    let mut metric = len as c_int;
    metric = metric.wrapping_mul(2).wrapping_add(0o010);
    metric
}

fn safe_double_to_int(mut value: f64) -> c_int {
    if value > 2147483647.0 {
        value = 2147483647.0;
    }
    if value < -2147483648.0 {
        value = -2147483648.0;
    }
    // Match C semantics of (int)value: truncation toward zero.
    value as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn jumpnode(
    operation_mode: c_int,
    node_id: c_int,
    depth: c_int,
    flags: c_int,
) -> c_int {
    let mut result: c_int = 0;
    let mut temp_array: [c_int; 20] = [0; 20];
    let mut buffer: [u8; 50] = [0; 50];

    match operation_mode {
        0o0001 => {
            let mut current_node = find_node_by_id(node_id);
            if current_node.is_null() {
                return STATUS_ERROR | 0o0020;
            }

            unsafe {
                let mut accumulated_value: f64 = (*current_node).value;

                let mut i: c_int = 0;
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
        }
        0o0002 => {
            let current_node = find_node_by_id(node_id);
            if current_node.is_null() {
                return STATUS_ERROR | 0o0040;
            }

            unsafe {
                for i in 0..4 {
                    temp_array[i] = (*current_node).data[i];
                }
            }

            for i in 4..0o020 {
                temp_array[i] = (i as c_int) * 0o0007;
            }

            let array_size: usize = 0o020;

            result = process_backward(temp_array.as_mut_ptr(), array_size, depth);

            result = result.wrapping_add((array_size as c_int).wrapping_mul(flags));
        }
        0o0003 => {
            // sprintf(buffer, "Node_%d_Depth_%d", node_id, depth);
            // Format manually to match C's "%d" (decimal int).
            let formatted = format!("Node_{}_Depth_{}", node_id, depth);
            let bytes = formatted.as_bytes();
            // Copy into buffer (assumes fits within 50 bytes; matches C's buffer size).
            let copy_len = bytes.len().min(buffer.len() - 1);
            buffer[..copy_len].copy_from_slice(&bytes[..copy_len]);
            buffer[copy_len] = 0; // NUL terminator like sprintf

            result = compute_size_metric(&buffer);

            result = result.wrapping_add(flags & 0o0177);
        }
        0o0004 => {
            let current_node = find_node_by_id(node_id);
            if current_node.is_null() {
                return STATUS_ERROR | 0o0100;
            }

            let mut accumulated_value: f64 = 0.0;
            unsafe {
                for i in 0..4 {
                    accumulated_value += ((*current_node).data[i] as f64).sqrt() * 2.718281828;
                }
            }

            accumulated_value *= 1.0 + (depth as f64) * 0.1;

            result = safe_double_to_int(accumulated_value);

            unsafe {
                if NODE_COUNT > 2 {
                    let storage_ptr = std::ptr::addr_of_mut!(NODE_STORAGE) as *mut Node;
                    let end_ptr = storage_ptr.add(NODE_COUNT as usize);
                    let mut iter = end_ptr;
                    let mut backward_sum: c_int = 0;

                    let mut i: c_int = 0;
                    while i < 3 && iter > storage_ptr {
                        iter = iter.offset(-1);
                        backward_sum =
                            backward_sum.wrapping_add(safe_double_to_int((*iter).value));
                        i += 1;
                    }

                    result = result.wrapping_add(backward_sum);
                }
            }
        }
        _ => {
            result = STATUS_ERROR | 0o0200;
        }
    }

    result
}

#[allow(dead_code)]
fn initialize_test_data() {
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
