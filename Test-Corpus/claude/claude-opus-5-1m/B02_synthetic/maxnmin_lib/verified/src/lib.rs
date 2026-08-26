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

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_double, c_int};
use std::ptr;

// The two <string.h> primitives the C source relies on. They are called through
// libc rather than reimplemented so that the observable behaviour for a caller
// supplied pointer -- including an invalid one, which the C source never checks
// -- is exactly the C's: the fault happens inside libc, delivering SIGSEGV,
// instead of being turned into a Rust `debug_assertions` abort.
unsafe extern "C" {
    fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    fn strlen(s: *const c_char) -> usize;
}

// #define MAX_NODES 100
const MAX_NODES: usize = 100;
// #define MAX_NAME_LEN 50
const MAX_NAME_LEN: usize = 50;

// C limits.h
const INT_MAX: c_int = 2147483647;
const INT_MIN: c_int = -2147483648;

/// typedef struct {
///     int id;
///     int parent_id;
///     char name[MAX_NAME_LEN];
///     double value;
///     int active;
/// } Node;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Node {
    pub id: c_int,
    pub parent_id: c_int,
    pub name: [c_char; MAX_NAME_LEN],
    pub value: c_double,
    pub active: c_int,
}

impl Node {
    const fn zeroed() -> Node {
        Node {
            id: 0,
            parent_id: 0,
            name: [0; MAX_NAME_LEN],
            value: 0.0,
            active: 0,
        }
    }
}

// static Node node_storage[MAX_NODES];
static mut NODE_STORAGE: [Node; MAX_NODES] = [Node::zeroed(); MAX_NODES];
// static int node_count = 0;
static mut NODE_COUNT: c_int = 0;

#[inline]
fn storage_ptr() -> *mut Node {
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

/// int add_node(int id, int parent_id, const char *name, double value)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_node(
    id: c_int,
    parent_id: c_int,
    name: *const c_char,
    value: c_double,
) -> c_int {
    let count = node_count_get();
    if count as usize >= MAX_NODES {
        return -1;
    }

    // Node new_node = { .id = id, .parent_id = parent_id, .value = value, .active = 1 };
    // (designated initializer zero-fills the remaining members, i.e. `name`)
    let mut new_node = Node {
        id,
        parent_id,
        name: [0; MAX_NAME_LEN],
        value,
        active: 1,
    };

    // strncpy(new_node.name, name, MAX_NAME_LEN - 1);
    // Delegated to libc so the semantics (copy up to n bytes, stop at the source
    // NUL, NUL-pad the rest of the n-byte range, never NUL-terminate when the
    // source is longer) and the behaviour on an invalid `name` are the C's.
    strncpy(new_node.name.as_mut_ptr(), name, MAX_NAME_LEN - 1);
    // new_node.name[MAX_NAME_LEN - 1] = '\0';
    new_node.name[MAX_NAME_LEN - 1] = 0;

    // node_storage[node_count++] = new_node;
    let base = storage_ptr();
    ptr::write(base.add(count as usize), new_node);
    let new_count = count.wrapping_add(1);
    node_count_set(new_count);

    // return node_count - 1;
    new_count.wrapping_sub(1)
}

/// Node* find_node_by_id(int id)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_node_by_id(id: c_int) -> *mut Node {
    let base = storage_ptr();
    let count = node_count_get();
    let mut i: c_int = 0;
    while i < count {
        let n = base.add(i as usize);
        if (*n).id == id && (*n).active != 0 {
            return n;
        }
        i = i.wrapping_add(1);
    }
    ptr::null_mut()
}

/// int get_children_count(int parent_id)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_children_count(parent_id: c_int) -> c_int {
    let base = storage_ptr();
    let count = node_count_get();
    let mut result: c_int = 0;
    let mut i: c_int = 0;
    while i < count {
        let n = base.add(i as usize);
        if (*n).parent_id == parent_id && (*n).active != 0 {
            result = result.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    result
}

/// Quiet a NaN the way an x86 SSE arithmetic instruction does: set the
/// is-quiet bit, leaving the sign and the payload alone.
#[inline]
fn quiet_nan(x: c_double) -> c_double {
    c_double::from_bits(x.to_bits() | 0x0008_0000_0000_0000)
}

/// `sum += child`, reproducing the reference C build's operand order.
///
/// When both operands are NaN, IEEE 754 leaves the resulting payload up to the
/// implementation, and x86 resolves it by *operand position*: `ADDSD` returns
/// SRC1 (quieted if it is a signalling NaN), else SRC2 if that is a NaN, else the
/// arithmetic sum. The reference C compiler emits
///
/// ```text
///     call  calculate_subtree_sum     ; the child's sum lands in xmm0
///     movsd -0x8(%rbp),%xmm1          ; xmm1 = the accumulator
///     addsd %xmm1,%xmm0               ; Intel: addsd xmm0, xmm1  =>  SRC1 = child
/// ```
///
/// so the CHILD is SRC1 and therefore wins the tie. Writing this out explicitly
/// instead of as `sum + child` is what makes the result reproducible: `fadd` is
/// commutative as far as LLVM is concerned, so an optimising build is free to
/// swap the operands, which would silently change the NaN payload that comes
/// back out of this function.
#[inline]
fn add_child_into_sum(child: c_double, sum: c_double) -> c_double {
    if child.is_nan() {
        return quiet_nan(child);
    }
    if sum.is_nan() {
        return quiet_nan(sum);
    }
    // Neither operand is a NaN, so the result is unambiguous (including the
    // default QNaN that inf + -inf produces).
    child + sum
}

/// double calculate_subtree_sum(int node_id)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn calculate_subtree_sum(node_id: c_int) -> c_double {
    let node = find_node_by_id(node_id);
    if node.is_null() {
        return 0.0;
    }

    let mut sum: c_double = (*node).value;

    let base = storage_ptr();
    let count = node_count_get();
    let mut i: c_int = 0;
    while i < count {
        let n = base.add(i as usize);
        if (*n).parent_id == node_id && (*n).active != 0 {
            // sum += calculate_subtree_sum(node_storage[i].id);
            sum = add_child_into_sum(calculate_subtree_sum((*n).id), sum);
        }
        i = i.wrapping_add(1);
    }

    sum
}

/// int process_string(char *str)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_string(str_: *mut c_char) -> c_int {
    let mut result: c_int = 0;

    // if (*str) { while (*str) { result += (int)(*str); str++; } }
    //
    // The C walks the string one byte at a time, so the loop covers exactly the
    // bytes before the first NUL. `strlen` finds that same count, and -- like the
    // C's very first `*str` -- it is the operation that faults if the caller
    // passed an invalid pointer (the C source never checks for NULL).
    let n = strlen(str_);
    let mut i: usize = 0;
    while i < n {
        // `char` is signed on the reference platform, so the value is
        // sign-extended to int.
        result = result.wrapping_add(*str_.add(i) as c_int);
        i += 1;
    }

    result
}

/// int safe_double_to_int(double d)
#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    if d > INT_MAX as c_double {
        return INT_MAX;
    }
    if d < INT_MIN as c_double {
        return INT_MIN;
    }

    if d != d {
        return 0;
    }

    // (int)d -- truncation toward zero; the range has been checked above.
    d as c_int
}

/// int maxnmin(int param1, int param2, int param3, int param4)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn maxnmin(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int = 0;

    node_count_set(0);

    add_node(1, -1, b"root\0".as_ptr() as *const c_char, 10.5);
    add_node(2, 1, b"child1\0".as_ptr() as *const c_char, 20.7);
    add_node(3, 1, b"child2\0".as_ptr() as *const c_char, 15.3);
    add_node(4, 2, b"grandchild1\0".as_ptr() as *const c_char, 5.9);
    add_node(5, 2, b"grandchild2\0".as_ptr() as *const c_char, 8.2);
    add_node(6, 3, b"grandchild3\0".as_ptr() as *const c_char, 12.4);

    let node_id = (param1 % 6).wrapping_add(1);
    let selected_node = find_node_by_id(node_id);

    if !selected_node.is_null() {
        let name_ptr = ptr::addr_of_mut!((*selected_node).name) as *mut c_char;

        if *name_ptr != 0 {
            result = result.wrapping_add(process_string(name_ptr));
        }

        let subtree_sum = calculate_subtree_sum(node_id);

        let sum_as_int = safe_double_to_int(subtree_sum);
        result = result.wrapping_add(sum_as_int);
    }

    let second_node_id = (param2 % 6).wrapping_add(1);
    let second_node = find_node_by_id(second_node_id);

    if !second_node.is_null() {
        let value_multiplied = (*second_node).value * param3 as c_double;

        let converted_value = safe_double_to_int(value_multiplied);
        result = result.wrapping_add(converted_value);
    }

    let parent_id = (param4 % 3).wrapping_add(1);
    let children = get_children_count(parent_id);
    result = result.wrapping_add(children.wrapping_mul(10));

    let mut calculation =
        (param1.wrapping_add(param2)) as c_double / (param3.wrapping_add(1)) as c_double;
    calculation *= param4 as c_double;

    let final_calc = safe_double_to_int(calculation);
    result = result.wrapping_add(final_calc);

    result
}
