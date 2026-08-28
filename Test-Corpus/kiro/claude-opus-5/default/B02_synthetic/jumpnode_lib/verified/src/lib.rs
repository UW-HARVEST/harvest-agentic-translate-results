// Rust translation of c_src/src/lib.c
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::ffi::{c_char, c_double, c_int};
use std::ptr;

// typedef struct { int id; int parent_id; double value; int data[4]; } Node;
#[repr(C)]
#[derive(Clone, Copy)]
struct Node {
    id: c_int,
    parent_id: c_int,
    value: c_double,
    data: [c_int; 4],
}

const MAX_NODES: usize = 100;

// static Node node_storage[MAX_NODES];  (zero-initialized, file scope)
static mut NODE_STORAGE: [Node; MAX_NODES] = [Node {
    id: 0,
    parent_id: 0,
    value: 0.0,
    data: [0; 4],
}; MAX_NODES];

// static int node_count = 0;
static mut NODE_COUNT: c_int = 0;

const STATUS_OK: c_int = 0o0;
#[allow(dead_code)]
const STATUS_WARNING: c_int = 0o1;
const STATUS_ERROR: c_int = 0o2;
#[allow(dead_code)]
const STATUS_CRITICAL: c_int = 0o377;

/// Base pointer for `node_storage`, matching the C array decay.
#[inline]
fn node_storage_ptr() -> *mut Node {
    ptr::addr_of_mut!(NODE_STORAGE) as *mut Node
}

#[inline]
fn node_count_get() -> c_int {
    unsafe { ptr::read(ptr::addr_of!(NODE_COUNT)) }
}

#[inline]
fn node_count_set(v: c_int) {
    unsafe { ptr::write(ptr::addr_of_mut!(NODE_COUNT), v) }
}

// static Node* find_node_by_id(int id)
fn find_node_by_id(id: c_int) -> *mut Node {
    let base = node_storage_ptr();
    let count = node_count_get();
    let mut i: c_int = 0;
    while i < count {
        unsafe {
            let elem = base.offset(i as isize);
            if (*elem).id == id {
                return elem;
            }
        }
        i += 1;
    }
    ptr::null_mut()
}

// static int add_node(int id, int parent_id, double value)
#[allow(dead_code)]
fn add_node(id: c_int, parent_id: c_int, value: c_double) -> c_int {
    let count = node_count_get();
    if count as usize >= MAX_NODES {
        return STATUS_ERROR;
    }

    unsafe {
        let slot = node_storage_ptr().offset(count as isize);
        (*slot).id = id;
        (*slot).parent_id = parent_id;
        (*slot).value = value;

        (*slot).data[0] = 0o100;
        (*slot).data[1] = 0o200;
        (*slot).data[2] = 0o300;
        (*slot).data[3] = 0o400;
    }

    node_count_set(count.wrapping_add(1));
    STATUS_OK
}

// static int process_backward(int *array, size_t size, int start_offset)
//
// Walks backward from `array + size` down to (exclusive) `array + start_offset`.
// The C code performs no bounds validation on `start_offset`, so the pointer
// arithmetic is reproduced verbatim.
unsafe fn process_backward(array: *mut c_int, size: usize, start_offset: c_int) -> c_int {
    let mut sum: c_int = 0;

    let mut p = array.offset(size as isize);
    let start = array.offset(start_offset as isize);

    while p > start {
        p = p.offset(-1);
        sum = sum.wrapping_add(*p);
    }

    sum
}

// static int compute_size_metric(const char *str)
unsafe fn compute_size_metric(s: *const c_char) -> c_int {
    // strlen
    let mut len: usize = 0;
    while *s.add(len) != 0 {
        len += 1;
    }

    let mut metric: c_int = len as c_int;

    metric = metric.wrapping_mul(2).wrapping_add(0o10);

    metric
}

// static int safe_double_to_int(double value)
fn safe_double_to_int(value: c_double) -> c_int {
    let mut value = value;

    if value > 2147483647.0 {
        value = 2147483647.0;
    }
    if value < -2147483648.0 {
        value = -2147483648.0;
    }

    // NaN compares false against both bounds above, so it survives the clamp
    // and reaches C's `(int)value`. That cast lowers to `cvttsd2si` on x86-64,
    // which returns the "integer indefinite" value INT_MIN for NaN, whereas
    // Rust's `as` would saturate it to 0. Reproduce the C result.
    if value.is_nan() {
        return c_int::MIN;
    }

    value as c_int
}

#[unsafe(no_mangle)]
#[allow(unused_assignments)]
pub unsafe extern "C" fn jumpnode(
    operation_mode: c_int,
    node_id: c_int,
    depth: c_int,
    flags: c_int,
) -> c_int {
    let mut current_node: *mut Node;
    let mut parent_node: *mut Node;
    let mut result: c_int = 0;
    let mut i: c_int;
    let mut accumulated_value: c_double;
    let mut temp_array: [c_int; 20] = [0; 20];
    let array_size: usize;
    let mut buffer: [c_char; 50] = [0; 50];

    match operation_mode {
        0o1 => {
            current_node = find_node_by_id(node_id);
            if current_node.is_null() {
                return STATUS_ERROR | 0o20;
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

        0o2 => {
            current_node = find_node_by_id(node_id);
            if current_node.is_null() {
                return STATUS_ERROR | 0o40;
            }

            i = 0;
            while i < 4 {
                temp_array[i as usize] = (*current_node).data[i as usize];
                i += 1;
            }

            i = 4;
            while i < 0o20 {
                temp_array[i as usize] = i.wrapping_mul(0o7);
                i += 1;
            }

            array_size = 0o20;

            result = process_backward(temp_array.as_mut_ptr(), array_size, depth);

            result = result.wrapping_add((array_size as c_int).wrapping_mul(flags));
        }

        0o3 => {
            sprintf_node_depth(&mut buffer, node_id, depth);

            result = compute_size_metric(buffer.as_ptr());

            result = result.wrapping_add(flags & 0o177); /* Mask with octal 0177 */
        }

        0o4 => {
            current_node = find_node_by_id(node_id);
            if current_node.is_null() {
                return STATUS_ERROR | 0o100;
            }

            accumulated_value = 0.0;
            i = 0;
            while i < 4 {
                accumulated_value +=
                    ((*current_node).data[i as usize] as c_double).sqrt() * 2.718281828;
                i += 1;
            }

            accumulated_value *= 1.0 + (depth as c_double) * 0.1;

            result = safe_double_to_int(accumulated_value);

            let count = node_count_get();
            if count > 2 {
                let base = node_storage_ptr();
                let end_ptr = base.offset(count as isize);
                let mut iter = end_ptr;
                let mut backward_sum: c_int = 0;

                i = 0;
                while i < 3 && iter > base {
                    iter = iter.offset(-1);
                    backward_sum = backward_sum.wrapping_add(safe_double_to_int((*iter).value));
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

/// Reproduces `sprintf(buffer, "Node_%d_Depth_%d", node_id, depth)`.
///
/// The formatted result is at most 34 bytes (`"Node_"` + 11 + `"_Depth_"` + 11)
/// plus the terminating NUL, so it always fits the 50-byte C buffer.
fn sprintf_node_depth(buffer: &mut [c_char; 50], node_id: c_int, depth: c_int) {
    let s = format!("Node_{}_Depth_{}", node_id, depth);
    let bytes = s.as_bytes();
    for (dst, &b) in buffer.iter_mut().zip(bytes.iter()) {
        *dst = b as c_char;
    }
    buffer[bytes.len()] = 0;
}

// static void initialize_test_data(void)
//
// Present in the C source but never called (it is `static`), so `node_count`
// remains 0 at runtime. Kept for fidelity.
#[allow(dead_code)]
fn initialize_test_data() {
    node_count_set(0);

    add_node(1, -1, 100.5);
    add_node(2, 1, 50.25);
    add_node(3, 1, 75.75);
    add_node(4, 2, 25.125);
    add_node(5, 2, 30.875);
    add_node(6, 3, 40.0625);
    add_node(7, 4, 12.5);
}
