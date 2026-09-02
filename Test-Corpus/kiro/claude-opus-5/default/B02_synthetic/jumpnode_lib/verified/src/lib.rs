// Rust translation of c_src/src/lib.c
//
// Original C source:
//   Copyright 2025 MIT Lincoln Laboratory
//   Permission is hereby granted, free of charge,
//   to any person obtaining a copy of this software
//   and associated documentation files (the "Software"),
//   to deal in the Software without restriction,
//   including without limitation the rights to use, copy,
//   modify, merge, publish, distribute, sublicense,
//   and/or sell copies of the Software,
//   and to permit persons to whom the Software is furnished to do so,
//   subject to the following conditions:
//
//   The above copyright notice and this permission notice
//   shall be included in all copies or substantial portions of the Software.
//
//   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
//   EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
//   THE WARRANTIES OF MERCHANTABILITY,
//   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
//   IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
//   FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
//   TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
//   OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
//
// The public ABI of the C library consists of a single exported symbol:
//   int jumpnode(int, int, int, int);
// Everything else in the translation unit is `static` (internal linkage) and is
// reproduced here as private Rust items so that the observable behaviour of
// `jumpnode` is identical, including the fact that `initialize_test_data()` is
// never called (so `node_count` stays 0 for the lifetime of the process).

#![allow(dead_code)]
// `int result = 0;` in the C original is overwritten on every switch arm; the
// initialiser is kept for fidelity with the source.
#![allow(unused_assignments)]

use core::ffi::{c_char, c_int};

/// C: typedef struct { int id; int parent_id; double value; int data[4]; } Node;
#[repr(C)]
#[derive(Copy, Clone)]
struct Node {
    id: c_int,
    parent_id: c_int,
    value: f64,
    data: [c_int; 4],
}

impl Node {
    const fn zeroed() -> Self {
        Node {
            id: 0,
            parent_id: 0,
            value: 0.0,
            data: [0; 4],
        }
    }
}

/// C: #define MAX_NODES 100
const MAX_NODES: usize = 100;

/// C: static Node node_storage[MAX_NODES];
static mut NODE_STORAGE: [Node; MAX_NODES] = [Node::zeroed(); MAX_NODES];
/// C: static int node_count = 0;
static mut NODE_COUNT: c_int = 0;

/// C: #define STATUS_OK       0000
const STATUS_OK: c_int = 0o0;
/// C: #define STATUS_WARNING  0001
const STATUS_WARNING: c_int = 0o1;
/// C: #define STATUS_ERROR    0002
const STATUS_ERROR: c_int = 0o2;
/// C: #define STATUS_CRITICAL 0377
const STATUS_CRITICAL: c_int = 0o377;

/// C: static Node* find_node_by_id(int id)
///
/// Returns a raw pointer into `NODE_STORAGE`, or null, exactly like the C code.
unsafe fn find_node_by_id(id: c_int) -> *mut Node {
    let storage: *mut Node = core::ptr::addr_of_mut!(NODE_STORAGE) as *mut Node;
    let count = NODE_COUNT;
    let mut i: c_int = 0;
    while i < count {
        let elem = storage.offset(i as isize);
        if (*elem).id == id {
            return elem;
        }
        i += 1;
    }
    core::ptr::null_mut()
}

/// C: static int add_node(int id, int parent_id, double value)
unsafe fn add_node(id: c_int, parent_id: c_int, value: f64) -> c_int {
    if NODE_COUNT as usize >= MAX_NODES {
        return STATUS_ERROR;
    }

    let storage: *mut Node = core::ptr::addr_of_mut!(NODE_STORAGE) as *mut Node;
    let slot = storage.offset(NODE_COUNT as isize);

    (*slot).id = id;
    (*slot).parent_id = parent_id;
    (*slot).value = value;

    (*slot).data[0] = 0o100;
    (*slot).data[1] = 0o200;
    (*slot).data[2] = 0o300;
    (*slot).data[3] = 0o400;

    NODE_COUNT += 1;
    STATUS_OK
}

/// C: static int process_backward(int *array, size_t size, int start_offset)
///
/// Walks backwards from `array + size` down to (exclusive) `array + start_offset`
/// summing the elements. Pointer arithmetic and the `int` accumulator wrap-around
/// are reproduced literally.
unsafe fn process_backward(array: *mut c_int, size: usize, start_offset: c_int) -> c_int {
    let mut sum: c_int = 0;

    let mut ptr: *mut c_int = array.wrapping_add(size);
    let start: *mut c_int = array.wrapping_offset(start_offset as isize);

    while ptr > start {
        ptr = ptr.wrapping_sub(1);
        sum = sum.wrapping_add(*ptr);
    }

    sum
}

/// C: size_t strlen(const char *)
unsafe fn c_strlen(str: *const c_char) -> usize {
    let mut n: usize = 0;
    while *str.add(n) != 0 {
        n += 1;
    }
    n
}

/// C: static int compute_size_metric(const char *str)
unsafe fn compute_size_metric(str: *const c_char) -> c_int {
    let len: usize = c_strlen(str);
    let mut metric: c_int;

    metric = len as c_int;

    metric = metric.wrapping_mul(2).wrapping_add(0o10);

    metric
}

/// C: static int safe_double_to_int(double value)
///
/// After the two clamps the value always fits in `int`, so the C cast is a plain
/// truncation towards zero. NaN is handled the way the x86-64 `cvttsd2si`
/// instruction behaves (the "integer indefinite" value, i.e. INT_MIN), matching
/// what the compiled C does for that otherwise-undefined input.
fn safe_double_to_int(value: f64) -> c_int {
    let mut value = value;

    if value > 2147483647.0 {
        value = 2147483647.0;
    }
    if value < -2147483648.0 {
        value = -2147483648.0;
    }

    if value.is_nan() {
        return c_int::MIN;
    }

    value as c_int
}

/// C: int jumpnode(int operation_mode, int node_id, int depth, int flags)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jumpnode(
    operation_mode: c_int,
    node_id: c_int,
    depth: c_int,
    flags: c_int,
) -> c_int {
    let mut current_node: *mut Node;
    let mut result: c_int = 0;
    let mut i: c_int;
    let mut accumulated_value: f64;
    // C: int temp_array[20];  (uninitialised; indices 0..15 are written before use)
    let mut temp_array: [c_int; 20] = [0; 20];
    let array_size: usize;
    // C: char buffer[50];
    let mut buffer: [c_char; 50] = [0; 50];

    match operation_mode {
        // case 0001:
        0o1 => {
            current_node = find_node_by_id(node_id);
            if current_node.is_null() {
                return STATUS_ERROR | 0o20;
            }

            accumulated_value = (*current_node).value;

            i = 0;
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

        // case 0002:
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

        // case 0003:
        0o3 => {
            c_sprintf_node_depth(buffer.as_mut_ptr(), node_id, depth);

            result = compute_size_metric(buffer.as_ptr());

            result = result.wrapping_add(flags & 0o177); /* Mask with octal 0177 */
        }

        // case 0004:
        0o4 => {
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

            accumulated_value *= 1.0 + depth as f64 * 0.1;

            result = safe_double_to_int(accumulated_value);

            if NODE_COUNT > 2 {
                let storage: *mut Node = core::ptr::addr_of_mut!(NODE_STORAGE) as *mut Node;
                let end_ptr: *mut Node = storage.offset(NODE_COUNT as isize);
                let mut iter: *mut Node = end_ptr;
                let mut backward_sum: c_int = 0;

                i = 0;
                while i < 3 && iter > storage {
                    iter = iter.offset(-1);
                    backward_sum = backward_sum.wrapping_add(safe_double_to_int((*iter).value));
                    i += 1;
                }

                result = result.wrapping_add(backward_sum);
            }
        }

        // default:
        _ => {
            result = STATUS_ERROR | 0o200;
        }
    }

    result
}

/// C: sprintf(buffer, "Node_%d_Depth_%d", node_id, depth);
///
/// Writes the formatted text plus a terminating NUL into `buffer`, which the C
/// code sizes at 50 bytes (the widest possible result is 34 characters plus the
/// NUL, so it always fits).
unsafe fn c_sprintf_node_depth(buffer: *mut c_char, node_id: c_int, depth: c_int) -> c_int {
    let mut bytes: [u8; 64] = [0; 64];
    let mut len: usize = 0;

    write_bytes_str(&mut bytes, &mut len, b"Node_");
    write_bytes_int(&mut bytes, &mut len, node_id);
    write_bytes_str(&mut bytes, &mut len, b"_Depth_");
    write_bytes_int(&mut bytes, &mut len, depth);

    let mut k: usize = 0;
    while k < len {
        *buffer.add(k) = bytes[k] as c_char;
        k += 1;
    }
    *buffer.add(len) = 0;

    len as c_int
}

fn write_bytes_str(out: &mut [u8; 64], len: &mut usize, s: &[u8]) {
    for &b in s {
        out[*len] = b;
        *len += 1;
    }
}

/// Formats an `int` the way printf's `%d` conversion does.
fn write_bytes_int(out: &mut [u8; 64], len: &mut usize, value: c_int) {
    let negative = value < 0;
    // Use the unsigned magnitude so INT_MIN is handled correctly.
    let mut magnitude: u32 = if negative {
        (value as i64).unsigned_abs() as u32
    } else {
        value as u32
    };

    let mut digits: [u8; 10] = [0; 10];
    let mut n: usize = 0;
    if magnitude == 0 {
        digits[0] = b'0';
        n = 1;
    } else {
        while magnitude > 0 {
            digits[n] = b'0' + (magnitude % 10) as u8;
            magnitude /= 10;
            n += 1;
        }
    }

    if negative {
        out[*len] = b'-';
        *len += 1;
    }
    while n > 0 {
        n -= 1;
        out[*len] = digits[n];
        *len += 1;
    }
}

// The C function declares `Node *parent_node;` at block scope; the Rust
// translation scopes that variable to `case 0001`, where it is the only place it
// is ever assigned or read.

/// C: static void initialize_test_data(void)
///
/// Present in the C translation unit but never called from anywhere, so
/// `node_count` remains 0 for the life of the process and every
/// `find_node_by_id` lookup in `jumpnode` fails. Translated for completeness;
/// like the C original it has internal linkage and no callers.
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
