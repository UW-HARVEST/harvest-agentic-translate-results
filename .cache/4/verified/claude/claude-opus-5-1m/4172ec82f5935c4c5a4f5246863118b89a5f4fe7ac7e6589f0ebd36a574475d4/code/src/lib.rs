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

#![allow(non_snake_case)]
#![allow(dead_code)]

use core::ffi::{c_double, c_int};
use core::ptr;

// typedef struct {
//     int id;
//     int parent_id;
//     double value;
//     int data[4];
// } Node;
#[repr(C)]
#[derive(Copy, Clone)]
struct Node {
    id: c_int,
    parent_id: c_int,
    value: c_double,
    data: [c_int; 4],
}

impl Node {
    const ZERO: Node = Node {
        id: 0,
        parent_id: 0,
        value: 0.0,
        data: [0; 4],
    };
}

// #define MAX_NODES 100
const MAX_NODES: usize = 100;

// static Node node_storage[MAX_NODES];
static mut NODE_STORAGE: [Node; MAX_NODES] = [Node::ZERO; MAX_NODES];
// static int node_count = 0;
static mut NODE_COUNT: c_int = 0;

// #define STATUS_OK       0000
// #define STATUS_WARNING  0001
// #define STATUS_ERROR    0002
// #define STATUS_CRITICAL 0377
const STATUS_OK: c_int = 0o0000;
const STATUS_WARNING: c_int = 0o0001;
const STATUS_ERROR: c_int = 0o0002;
const STATUS_CRITICAL: c_int = 0o0377;

/// Base pointer for the `node_storage` array (equivalent to the C array-to-pointer decay).
#[inline]
fn node_storage_base() -> *mut Node {
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
unsafe fn find_node_by_id(id: c_int) -> *mut Node {
    let base = node_storage_base();
    let count = node_count_get();
    let mut i: c_int = 0;
    while i < count {
        let p = base.offset(i as isize);
        if (*p).id == id {
            return p;
        }
        i += 1;
    }
    ptr::null_mut()
}

// static int add_node(int id, int parent_id, double value)
unsafe fn add_node(id: c_int, parent_id: c_int, value: c_double) -> c_int {
    let count = node_count_get();
    if count as usize >= MAX_NODES {
        return STATUS_ERROR;
    }

    let base = node_storage_base();
    let slot = base.offset(count as isize);

    (*slot).id = id;
    (*slot).parent_id = parent_id;
    (*slot).value = value;

    (*slot).data[0] = 0o100;
    (*slot).data[1] = 0o200;
    (*slot).data[2] = 0o300;
    (*slot).data[3] = 0o400;

    node_count_set(count + 1);
    STATUS_OK
}

// static int process_backward(int *array, size_t size, int start_offset)
unsafe fn process_backward(array: *mut c_int, size: usize, start_offset: c_int) -> c_int {
    let mut sum: c_int = 0;

    let mut p: *mut c_int = array.add(size);
    let start: *mut c_int = array.offset(start_offset as isize);

    while p > start {
        p = p.offset(-1);
        sum = sum.wrapping_add(*p);
    }

    sum
}

// static int compute_size_metric(const char *str)
fn compute_size_metric(bytes: &[u8]) -> c_int {
    // size_t len = strlen(str);
    let len: usize = match bytes.iter().position(|&b| b == 0) {
        Some(n) => n,
        None => bytes.len(),
    };

    // metric = (int)len;
    let mut metric: c_int = len as u32 as c_int;

    // metric = metric * 2 + 010;
    metric = metric.wrapping_mul(2).wrapping_add(0o10);

    metric
}

/// Reproduces the x86-64 semantics of a C `(int)` cast from `double`
/// (truncation toward zero; `cvttsd2si` yields INT_MIN for NaN / out-of-range).
#[inline]
fn c_cast_double_to_int(value: c_double) -> c_int {
    if value.is_nan() {
        return c_int::MIN;
    }
    let t = value.trunc();
    if t >= 2147483648.0 || t < -2147483648.0 {
        return c_int::MIN;
    }
    t as c_int
}

// static int safe_double_to_int(double value)
fn safe_double_to_int(mut value: c_double) -> c_int {
    if value > 2147483647.0 {
        value = 2147483647.0;
    }
    if value < -2147483648.0 {
        value = -2147483648.0;
    }

    c_cast_double_to_int(value)
}

// int jumpnode(int operation_mode, int node_id, int depth, int flags)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jumpnode(
    operation_mode: c_int,
    node_id: c_int,
    depth: c_int,
    flags: c_int,
) -> c_int {
    let mut current_node: *mut Node;
    let mut parent_node: *mut Node;
    #[allow(unused_assignments)]
    let mut result: c_int = 0;
    let mut i: c_int;
    let mut accumulated_value: c_double;
    // int temp_array[20];
    let mut temp_array: [c_int; 20] = [0; 20];
    let array_size: usize;
    // char buffer[50];
    let mut buffer: [u8; 50] = [0; 50];

    match operation_mode {
        // case 0001:
        0o0001 => {
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

        // case 0002:
        0o0002 => {
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

            result = result.wrapping_add((array_size as u32 as c_int).wrapping_mul(flags));
        }

        // case 0003:
        0o0003 => {
            // sprintf(buffer, "Node_%d_Depth_%d", node_id, depth);
            sprintf_node_depth(&mut buffer, node_id, depth);

            result = compute_size_metric(&buffer[..]);

            // result += (flags & 0177);
            result = result.wrapping_add(flags & 0o177);
        }

        // case 0004:
        0o0004 => {
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
                let base = node_storage_base();
                let end_ptr: *mut Node = base.offset(count as isize);
                let mut iter: *mut Node = end_ptr;
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

        // default:
        _ => {
            result = STATUS_ERROR | 0o200;
        }
    }

    result
}

/// Emulates `sprintf(buffer, "Node_%d_Depth_%d", node_id, depth)` into a
/// fixed-size C character buffer (NUL terminated).
fn sprintf_node_depth(buffer: &mut [u8; 50], node_id: c_int, depth: c_int) {
    let mut out: usize = 0;

    let put = |buf: &mut [u8; 50], pos: &mut usize, b: u8| {
        if *pos < buf.len() {
            buf[*pos] = b;
        }
        *pos += 1;
    };

    for &b in b"Node_" {
        put(buffer, &mut out, b);
    }
    for &b in fmt_int(node_id).as_bytes() {
        put(buffer, &mut out, b);
    }
    for &b in b"_Depth_" {
        put(buffer, &mut out, b);
    }
    for &b in fmt_int(depth).as_bytes() {
        put(buffer, &mut out, b);
    }
    put(buffer, &mut out, 0);
}

/// Formats a `c_int` exactly as printf's `%d` does.
fn fmt_int(v: c_int) -> IntStr {
    let mut buf = [0u8; 12];
    let mut len = 0usize;

    let negative = v < 0;
    // Use the unsigned magnitude so that INT_MIN is handled correctly.
    let mut mag: u32 = if negative {
        (v as i64).unsigned_abs() as u32
    } else {
        v as u32
    };

    if mag == 0 {
        buf[0] = b'0';
        len = 1;
    } else {
        let mut tmp = [0u8; 12];
        let mut n = 0usize;
        while mag > 0 {
            tmp[n] = b'0' + (mag % 10) as u8;
            mag /= 10;
            n += 1;
        }
        if negative {
            buf[len] = b'-';
            len += 1;
        }
        while n > 0 {
            n -= 1;
            buf[len] = tmp[n];
            len += 1;
        }
        return IntStr { buf, len };
    }

    if negative {
        // Unreachable for mag == 0, kept for structural fidelity.
        buf[1] = b'0';
        buf[0] = b'-';
        len = 2;
    }

    IntStr { buf, len }
}

struct IntStr {
    buf: [u8; 12],
    len: usize,
}

impl IntStr {
    fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }
}

// static void initialize_test_data(void)
//
// Present in the C source but never called (it is `static` and unused), so the
// node storage stays empty at run time.  Kept for completeness / fidelity.
unsafe fn initialize_test_data() {
    node_count_set(0);

    add_node(1, -1, 100.5);
    add_node(2, 1, 50.25);
    add_node(3, 1, 75.75);
    add_node(4, 2, 25.125);
    add_node(5, 2, 30.875);
    add_node(6, 3, 40.0625);
    add_node(7, 4, 12.5);
}

/// Test-only probe surface (cargo feature `shadow_probe`, OFF by default).
///
/// `lib.c`'s helpers all have internal linkage, and `initialize_test_data` is
/// never called, so `jumpnode`'s modes 1/2/4 always take their "node not found"
/// error return. That leaves most of the algorithm unreachable — and therefore
/// unverifiable — through the public API alone.
///
/// This module exports thin wrappers around exactly those helpers. The C side
/// gets a matching set from `shadow_c/lib_shadow.c`, which `#include`s the
/// untouched `c_src/src/lib.c` so the statics land in the same translation unit.
/// That lets the differential suite compare the low-level functions directly and
/// drive `jumpnode` with populated node storage.
///
/// The default build enables none of this, so `libjumpnode_lib.so` exports the
/// same single symbol the C `.so` does (asserted by `symbol_parity.rs`).
#[cfg(feature = "shadow_probe")]
mod shadow_probe {
    use super::*;
    use core::ffi::{c_char, c_double, c_int};

    /// `initialize_test_data(); return node_count;`
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn probe_init() -> c_int {
        initialize_test_data();
        node_count_get()
    }

    /// Clear the static storage back to its load-time (`.bss`) state.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn probe_reset() {
        node_count_set(0);
        let base = node_storage_base();
        let mut i = 0usize;
        while i < MAX_NODES {
            *base.add(i) = Node::ZERO;
            i += 1;
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn probe_node_count() -> c_int {
        node_count_get()
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn probe_add_node(
        id: c_int,
        parent_id: c_int,
        value: c_double,
    ) -> c_int {
        add_node(id, parent_id, value)
    }

    /// `find_node_by_id`, reported as an index into the storage array
    /// (`-1` for the `NULL` return) so it can cross the FFI boundary.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn probe_find(id: c_int) -> c_int {
        let p = find_node_by_id(id);
        if p.is_null() {
            -1
        } else {
            p.offset_from(node_storage_base()) as c_int
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn probe_process_backward(
        array: *mut c_int,
        size: usize,
        start_offset: c_int,
    ) -> c_int {
        process_backward(array, size, start_offset)
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn probe_compute_size_metric(s: *const c_char) -> c_int {
        // Mirror `compute_size_metric(str)`, whose first act is `strlen(str)`.
        let mut len = 0usize;
        while *s.add(len) != 0 {
            len += 1;
        }
        let bytes = core::slice::from_raw_parts(s as *const u8, len + 1);
        compute_size_metric(bytes)
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn probe_safe_double_to_int(value: c_double) -> c_int {
        safe_double_to_int(value)
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn probe_node_id(idx: c_int) -> c_int {
        (*node_storage_base().offset(idx as isize)).id
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn probe_node_parent_id(idx: c_int) -> c_int {
        (*node_storage_base().offset(idx as isize)).parent_id
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn probe_node_value(idx: c_int) -> c_double {
        (*node_storage_base().offset(idx as isize)).value
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn probe_node_data(idx: c_int, k: c_int) -> c_int {
        (*node_storage_base().offset(idx as isize)).data[k as usize]
    }

    /// `sizeof(Node)` — verifies the `#[repr(C)]` layout matches the C struct.
    #[unsafe(no_mangle)]
    pub extern "C" fn probe_sizeof_node() -> usize {
        core::mem::size_of::<Node>()
    }

    /// The four `STATUS_*` macros, so the constants themselves are compared.
    #[unsafe(no_mangle)]
    pub extern "C" fn probe_status(which: c_int) -> c_int {
        match which {
            0 => STATUS_OK,
            1 => STATUS_WARNING,
            2 => STATUS_ERROR,
            3 => STATUS_CRITICAL,
            4 => MAX_NODES as c_int,
            _ => -1,
        }
    }
}
