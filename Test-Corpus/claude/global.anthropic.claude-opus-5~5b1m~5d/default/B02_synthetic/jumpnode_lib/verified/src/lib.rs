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

// The C original declares all of `jumpnode`'s locals up front and leaves some
// helpers unused; both are preserved verbatim here.
#![allow(dead_code, unused_assignments)]

use core::ffi::{c_char, c_double, c_int};

/// typedef struct { int id; int parent_id; double value; int data[4]; } Node;
#[repr(C)]
#[derive(Copy, Clone)]
struct Node {
    id: c_int,
    parent_id: c_int,
    value: c_double,
    data: [c_int; 4],
}

const MAX_NODES: usize = 100;

/// static Node node_storage[MAX_NODES];
static mut NODE_STORAGE: [Node; MAX_NODES] = [Node {
    id: 0,
    parent_id: 0,
    value: 0.0,
    data: [0; 4],
}; MAX_NODES];

/// static int node_count = 0;
static mut NODE_COUNT: c_int = 0;

const STATUS_OK: c_int = 0o0;
const STATUS_WARNING: c_int = 0o1;
const STATUS_ERROR: c_int = 0o2;
const STATUS_CRITICAL: c_int = 0o377;

/// Base pointer of `node_storage` (equivalent to the array-to-pointer decay of
/// `node_storage` in C).
#[inline]
fn node_storage_base() -> *mut Node {
    (&raw mut NODE_STORAGE).cast::<Node>()
}

/// static Node* find_node_by_id(int id)
unsafe fn find_node_by_id(id: c_int) -> *mut Node {
    let base = node_storage_base();
    let count = unsafe { NODE_COUNT };
    let mut i: c_int = 0;
    while i < count {
        let elem = unsafe { base.offset(i as isize) };
        if unsafe { (*elem).id } == id {
            return elem;
        }
        i += 1;
    }
    core::ptr::null_mut()
}

/// static int add_node(int id, int parent_id, double value)
unsafe fn add_node(id: c_int, parent_id: c_int, value: c_double) -> c_int {
    if unsafe { NODE_COUNT } as usize >= MAX_NODES {
        return STATUS_ERROR;
    }

    let base = node_storage_base();
    let slot = unsafe { base.offset(NODE_COUNT as isize) };

    unsafe {
        (*slot).id = id;
        (*slot).parent_id = parent_id;
        (*slot).value = value;

        (*slot).data[0] = 0o100;
        (*slot).data[1] = 0o200;
        (*slot).data[2] = 0o300;
        (*slot).data[3] = 0o400;

        NODE_COUNT += 1;
    }

    STATUS_OK
}

/// static int process_backward(int *array, size_t size, int start_offset)
unsafe fn process_backward(array: *mut c_int, size: usize, start_offset: c_int) -> c_int {
    let mut sum: c_int = 0;

    // ptr = array + size;  start = array + start_offset;
    let mut ptr = unsafe { array.offset(size as isize) };
    let start = unsafe { array.offset(start_offset as isize) };

    while ptr > start {
        ptr = unsafe { ptr.offset(-1) };
        sum = sum.wrapping_add(unsafe { *ptr });
    }

    sum
}

/// static int compute_size_metric(const char *str)
unsafe fn compute_size_metric(s: *const c_char) -> c_int {
    let len: usize = unsafe { c_strlen(s) };
    let mut metric: c_int;

    metric = len as c_int;

    metric = metric.wrapping_mul(2).wrapping_add(0o10);

    metric
}

/// strlen()
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n: usize = 0;
    while unsafe { *s.add(n) } != 0 {
        n += 1;
    }
    n
}

/// static int safe_double_to_int(double value)
fn safe_double_to_int(value: c_double) -> c_int {
    let mut value = value;

    if value > 2147483647.0 {
        value = 2147483647.0;
    }
    if value < -2147483648.0 {
        value = -2147483648.0;
    }

    value as c_int
}

/// int jumpnode(int operation_mode, int node_id, int depth, int flags)
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
    let mut i: c_int;
    let mut accumulated_value: c_double;
    let mut temp_array: [c_int; 20] = [0; 20];
    let array_size: usize;
    let mut buffer: [c_char; 50] = [0; 50];

    match operation_mode {
        0o1 => {
            current_node = unsafe { find_node_by_id(node_id) };
            if current_node.is_null() {
                return STATUS_ERROR | 0o20;
            }

            accumulated_value = unsafe { (*current_node).value };

            i = 0;
            while i < depth && unsafe { (*current_node).parent_id } != -1 {
                parent_node = unsafe { find_node_by_id((*current_node).parent_id) };
                if parent_node.is_null() {
                    break;
                }

                accumulated_value += unsafe { (*parent_node).value } * 1.5;
                current_node = parent_node;

                i += 1;
            }

            result = safe_double_to_int(accumulated_value);
        }

        0o2 => {
            current_node = unsafe { find_node_by_id(node_id) };
            if current_node.is_null() {
                return STATUS_ERROR | 0o40;
            }

            i = 0;
            while i < 4 {
                temp_array[i as usize] = unsafe { (*current_node).data[i as usize] };
                i += 1;
            }

            i = 4;
            while i < 0o20 {
                temp_array[i as usize] = i.wrapping_mul(0o7);
                i += 1;
            }

            array_size = 0o20;

            result = unsafe { process_backward(temp_array.as_mut_ptr(), array_size, depth) };

            result = result.wrapping_add((array_size as c_int).wrapping_mul(flags));
        }

        0o3 => {
            // sprintf(buffer, "Node_%d_Depth_%d", node_id, depth);
            c_sprintf_node_depth(&mut buffer, node_id, depth);

            result = unsafe { compute_size_metric(buffer.as_ptr()) };

            result = result.wrapping_add(flags & 0o177); /* Mask with octal 0177 */
        }

        0o4 => {
            current_node = unsafe { find_node_by_id(node_id) };
            if current_node.is_null() {
                return STATUS_ERROR | 0o100;
            }

            accumulated_value = 0.0;
            i = 0;
            while i < 4 {
                accumulated_value +=
                    (unsafe { (*current_node).data[i as usize] } as c_double).sqrt() * 2.718281828;
                i += 1;
            }

            accumulated_value *= 1.0 + (depth as c_double) * 0.1;

            result = safe_double_to_int(accumulated_value);

            if unsafe { NODE_COUNT } > 2 {
                let base = node_storage_base();
                let end_ptr = unsafe { base.offset(NODE_COUNT as isize) };
                let mut iter = end_ptr;
                let mut backward_sum: c_int = 0;

                i = 0;
                while i < 3 && iter > base {
                    iter = unsafe { iter.offset(-1) };
                    backward_sum =
                        backward_sum.wrapping_add(safe_double_to_int(unsafe { (*iter).value }));
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

/// Emulates `sprintf(buffer, "Node_%d_Depth_%d", node_id, depth)` writing into
/// the fixed 50-byte buffer, NUL terminated.
fn c_sprintf_node_depth(buffer: &mut [c_char; 50], node_id: c_int, depth: c_int) {
    let id_s = fmt_int(node_id);
    let depth_s = fmt_int(depth);

    // Longest possible rendering is 5 + 11 + 7 + 11 = 34 bytes plus the NUL,
    // so it always fits in the 50 byte buffer (as it does in the C original).
    let mut pos: usize = 0;
    for part in [
        b"Node_".as_slice(),
        id_s.as_slice(),
        b"_Depth_".as_slice(),
        depth_s.as_slice(),
    ] {
        for &b in part {
            buffer[pos] = b as c_char;
            pos += 1;
        }
    }
    buffer[pos] = 0;
}

/// Renders a `c_int` the way `printf("%d", v)` does.
struct IntStr {
    bytes: [u8; 12],
    len: usize,
}

impl IntStr {
    fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

fn fmt_int(v: c_int) -> IntStr {
    let mut tmp = [0u8; 12];
    let mut n: usize = 0;

    let negative = v < 0;
    // Use the unsigned magnitude so that i32::MIN is handled like C does.
    let mut mag: u32 = if negative {
        (v as i64).unsigned_abs() as u32
    } else {
        v as u32
    };

    if mag == 0 {
        tmp[n] = b'0';
        n += 1;
    } else {
        while mag > 0 {
            tmp[n] = b'0' + (mag % 10) as u8;
            mag /= 10;
            n += 1;
        }
    }
    if negative {
        tmp[n] = b'-';
        n += 1;
    }

    // reverse
    let mut bytes = [0u8; 12];
    for k in 0..n {
        bytes[k] = tmp[n - 1 - k];
    }

    IntStr { bytes, len: n }
}

/// static void initialize_test_data(void)
///
/// Present in the original C source but never called; preserved here (and
/// likewise never called) so that the library's observable state matches.
///
/// Also reachable through an opt-in, non-default cargo feature that is only
/// used by the translation's own differential tests; the default build exports
/// exactly the same symbols as the C library.
#[cfg(feature = "expose_init_test_data")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jumpnode_initialize_test_data() {
    unsafe { initialize_test_data() }
}

unsafe fn initialize_test_data() {
    unsafe {
        NODE_COUNT = 0;

        add_node(1, -1, 100.5);
        add_node(2, 1, 50.25);
        add_node(3, 1, 75.75);
        add_node(4, 2, 25.125);
        add_node(5, 2, 30.875);
        add_node(6, 3, 40.0625);
        add_node(7, 4, 12.5);
    }
}
