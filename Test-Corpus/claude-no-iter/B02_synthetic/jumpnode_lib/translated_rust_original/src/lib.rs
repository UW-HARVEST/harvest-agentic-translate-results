// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Behavior is intended to be byte-identical to the
// original C library.

use std::ffi::c_int;

#[repr(C)]
#[derive(Copy, Clone)]
struct Node {
    id: c_int,
    parent_id: c_int,
    value: f64,
    data: [c_int; 4],
}

const MAX_NODES: usize = 100;

// Octal constants from the C source.
const STATUS_OK: c_int = 0o000;
#[allow(dead_code)]
const STATUS_WARNING: c_int = 0o001;
const STATUS_ERROR: c_int = 0o002;
#[allow(dead_code)]
const STATUS_CRITICAL: c_int = 0o377;

struct GlobalState {
    node_storage: [Node; MAX_NODES],
    node_count: c_int,
}

static mut GLOBAL_STATE: GlobalState = GlobalState {
    node_storage: [Node {
        id: 0,
        parent_id: 0,
        value: 0.0,
        data: [0; 4],
    }; MAX_NODES],
    node_count: 0,
};

#[inline]
unsafe fn state() -> *mut GlobalState {
    &raw mut GLOBAL_STATE
}

unsafe fn find_node_by_id(id: c_int) -> *mut Node {
    let s = state();
    let count = (*s).node_count;
    let mut i: c_int = 0;
    while i < count {
        let p = &raw mut (*s).node_storage[i as usize];
        if (*p).id == id {
            return p;
        }
        i += 1;
    }
    std::ptr::null_mut()
}

#[allow(dead_code)]
unsafe fn add_node(id: c_int, parent_id: c_int, value: f64) -> c_int {
    let s = state();
    if (*s).node_count as usize >= MAX_NODES {
        return STATUS_ERROR;
    }

    let idx = (*s).node_count as usize;
    (*s).node_storage[idx].id = id;
    (*s).node_storage[idx].parent_id = parent_id;
    (*s).node_storage[idx].value = value;

    (*s).node_storage[idx].data[0] = 0o100;
    (*s).node_storage[idx].data[1] = 0o200;
    (*s).node_storage[idx].data[2] = 0o300;
    (*s).node_storage[idx].data[3] = 0o400;

    (*s).node_count += 1;
    STATUS_OK
}

/// Mirrors the original C `process_backward` function. Iterates from
/// `array + size` down to `array + start_offset` (exclusive on the lower end
/// when start_offset >= size), summing values.
unsafe fn process_backward(array: *mut c_int, size: usize, start_offset: c_int) -> c_int {
    let mut sum: c_int = 0;
    let mut ptr: *mut c_int = array.add(size);
    let start: *mut c_int = array.offset(start_offset as isize);

    while ptr > start {
        ptr = ptr.offset(-1);
        sum = sum.wrapping_add(*ptr);
    }

    sum
}

unsafe fn compute_size_metric(s: *const u8) -> c_int {
    // Equivalent of C's strlen.
    let mut len: usize = 0;
    while *s.add(len) != 0 {
        len += 1;
    }

    let mut metric: c_int = len as c_int;
    metric = metric.wrapping_mul(2).wrapping_add(0o10);
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
pub unsafe extern "C" fn jumpnode(
    operation_mode: c_int,
    node_id: c_int,
    depth: c_int,
    flags: c_int,
) -> c_int {
    let mut current_node: *mut Node;
    let mut parent_node: *mut Node;
    let mut result: c_int = 0;
    let _ = result;
    let mut i: c_int;
    let mut accumulated_value: f64;
    let mut temp_array: [c_int; 20] = [0; 20];
    let array_size: usize;
    let mut buffer: [u8; 50] = [0; 50];

    match operation_mode {
        0o001 => {
            current_node = find_node_by_id(node_id);
            if current_node.is_null() {
                return STATUS_ERROR | 0o020;
            }

            accumulated_value = (*current_node).value;

            i = 0;
            while i < depth && (*current_node).parent_id != -1 {
                parent_node = find_node_by_id((*current_node).parent_id);
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
            current_node = find_node_by_id(node_id);
            if current_node.is_null() {
                return STATUS_ERROR | 0o040;
            }

            i = 0;
            while i < 4 {
                temp_array[i as usize] = (*current_node).data[i as usize];
                i += 1;
            }

            i = 4;
            while i < 0o20 {
                temp_array[i as usize] = i.wrapping_mul(0o007);
                i += 1;
            }

            array_size = 0o20;

            result = process_backward(temp_array.as_mut_ptr(), array_size, depth);

            result = result.wrapping_add((array_size as c_int).wrapping_mul(flags));
        }

        0o003 => {
            // sprintf(buffer, "Node_%d_Depth_%d", node_id, depth);
            // Format the same as C's %d for c_int.
            let formatted = format!("Node_{}_Depth_{}", node_id, depth);
            let bytes = formatted.as_bytes();
            // Copy bytes into buffer; assume the formatted string fits the
            // 50-byte buffer (matches the C code's assumption).
            for (idx, b) in bytes.iter().enumerate() {
                buffer[idx] = *b;
            }
            buffer[bytes.len()] = 0;

            result = compute_size_metric(buffer.as_ptr());

            result = result.wrapping_add(flags & 0o177);
        }

        0o004 => {
            current_node = find_node_by_id(node_id);
            if current_node.is_null() {
                return STATUS_ERROR | 0o100;
            }

            accumulated_value = 0.0;
            i = 0;
            while i < 4 {
                accumulated_value +=
                    ((*current_node).data[i as usize] as f64).sqrt() * 2.718281828;
                i += 1;
            }

            accumulated_value *= 1.0 + (depth as f64) * 0.1;

            result = safe_double_to_int(accumulated_value);

            let s = state();
            if (*s).node_count > 2 {
                let end_idx = (*s).node_count as usize;
                let mut iter_idx = end_idx;
                let mut backward_sum: c_int = 0;

                i = 0;
                while i < 3 && iter_idx > 0 {
                    iter_idx -= 1;
                    backward_sum = backward_sum
                        .wrapping_add(safe_double_to_int((*s).node_storage[iter_idx].value));
                    i += 1;
                }

                result = result.wrapping_add(backward_sum);
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
    let s = state();
    (*s).node_count = 0;

    add_node(1, -1, 100.5);
    add_node(2, 1, 50.25);
    add_node(3, 1, 75.75);
    add_node(4, 2, 25.125);
    add_node(5, 2, 30.875);
    add_node(6, 3, 40.0625);
    add_node(7, 4, 12.5);
}
