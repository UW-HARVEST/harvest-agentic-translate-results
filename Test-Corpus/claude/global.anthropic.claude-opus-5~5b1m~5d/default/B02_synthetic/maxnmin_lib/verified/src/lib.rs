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

use core::ffi::{c_char, c_double, c_int};
use core::ptr;

/// `#define MAX_NODES 100`
const MAX_NODES: usize = 100;
/// `#define MAX_NAME_LEN 50`
const MAX_NAME_LEN: usize = 50;

/// Mirrors the C `Node` struct exactly:
///
/// ```c
/// typedef struct {
///     int id;
///     int parent_id;
///     char name[MAX_NAME_LEN];
///     double value;
///     int active;
/// } Node;
/// ```
///
/// On the x86-64 SysV ABI this is 80 bytes with field offsets
/// 0, 4, 8, 64, 72 -- reproduced here via `#[repr(C)]`.
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
    /// Equivalent of C's zero-initialization for a `Node` in static storage.
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

/// `static Node node_storage[MAX_NODES];`
static mut NODE_STORAGE: [Node; MAX_NODES] = [Node::zeroed(); MAX_NODES];
/// `static int node_count = 0;`
static mut NODE_COUNT: c_int = 0;

#[inline(always)]
fn storage_ptr() -> *mut Node {
    (&raw mut NODE_STORAGE) as *mut Node
}

#[inline(always)]
fn node_count() -> c_int {
    unsafe { *(&raw const NODE_COUNT) }
}

#[inline(always)]
fn set_node_count(v: c_int) {
    unsafe { *(&raw mut NODE_COUNT) = v }
}

/// ```c
/// int add_node(int id, int parent_id, const char *name, double value);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_node(
    id: c_int,
    parent_id: c_int,
    name: *const c_char,
    value: c_double,
) -> c_int {
    let count = node_count();

    if count as usize >= MAX_NODES {
        return -1;
    }

    // Node new_node = { .id = id, .parent_id = parent_id,
    //                   .value = value, .active = 1 };
    // (designated initializer => `name` is zero filled)
    let mut new_node = Node {
        id,
        parent_id,
        name: [0; MAX_NAME_LEN],
        value,
        active: 1,
    };

    // strncpy(new_node.name, name, MAX_NAME_LEN - 1);
    // Copies at most 49 bytes, stopping after the source NUL, and
    // zero-pads the remainder of those 49 bytes.
    unsafe {
        let mut i = 0usize;
        while i < MAX_NAME_LEN - 1 {
            let ch = *name.add(i);
            if ch == 0 {
                break;
            }
            new_node.name[i] = ch;
            i += 1;
        }
        while i < MAX_NAME_LEN - 1 {
            new_node.name[i] = 0;
            i += 1;
        }
    }

    // new_node.name[MAX_NAME_LEN - 1] = '\0';
    new_node.name[MAX_NAME_LEN - 1] = 0;

    // node_storage[node_count++] = new_node;
    unsafe {
        *storage_ptr().add(count as usize) = new_node;
    }
    let count = count.wrapping_add(1);
    set_node_count(count);

    // return node_count - 1;
    count.wrapping_sub(1)
}

/// ```c
/// Node* find_node_by_id(int id);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_node_by_id(id: c_int) -> *mut Node {
    let count = node_count();
    let base = storage_ptr();

    let mut i: c_int = 0;
    while i < count {
        unsafe {
            let n = base.add(i as usize);
            if (*n).id == id && (*n).active != 0 {
                return n;
            }
        }
        i = i.wrapping_add(1);
    }

    ptr::null_mut()
}

/// ```c
/// int get_children_count(int parent_id);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_children_count(parent_id: c_int) -> c_int {
    let mut count: c_int = 0;
    let total = node_count();
    let base = storage_ptr();

    let mut i: c_int = 0;
    while i < total {
        unsafe {
            let n = base.add(i as usize);
            if (*n).parent_id == parent_id && (*n).active != 0 {
                count = count.wrapping_add(1);
            }
        }
        i = i.wrapping_add(1);
    }

    count
}

/// ```c
/// double calculate_subtree_sum(int node_id);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn calculate_subtree_sum(node_id: c_int) -> c_double {
    let node = unsafe { find_node_by_id(node_id) };
    if node.is_null() {
        return 0.0;
    }

    let mut sum: c_double = unsafe { (*node).value };

    let total = node_count();
    let base = storage_ptr();

    let mut i: c_int = 0;
    while i < total {
        unsafe {
            let n = base.add(i as usize);
            if (*n).parent_id == node_id && (*n).active != 0 {
                sum += calculate_subtree_sum((*n).id);
            }
        }
        i = i.wrapping_add(1);
    }

    sum
}

/// ```c
/// int process_string(char *str);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_string(str: *mut c_char) -> c_int {
    let mut result: c_int = 0;
    let mut p = str;

    // if (*str) { while (*str) { result += (int)(*str); str++; } }
    // `char` is signed on the target ABI, so the promotion sign-extends.
    unsafe {
        if *p != 0 {
            while *p != 0 {
                result = result.wrapping_add(*p as c_int);
                p = p.add(1);
            }
        }
    }

    result
}

/// ```c
/// int safe_double_to_int(double d);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    if d > c_int::MAX as c_double {
        return c_int::MAX;
    }
    if d < c_int::MIN as c_double {
        return c_int::MIN;
    }

    if d != d {
        return 0;
    }

    // At this point `d` is within [INT_MIN, INT_MAX], so the saturating
    // `as` cast behaves exactly like C's truncating conversion.
    d as c_int
}

/// ```c
/// int maxnmin(int param1, int param2, int param3, int param4);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn maxnmin(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int = 0;

    set_node_count(0);

    unsafe {
        add_node(1, -1, c"root".as_ptr(), 10.5);
        add_node(2, 1, c"child1".as_ptr(), 20.7);
        add_node(3, 1, c"child2".as_ptr(), 15.3);
        add_node(4, 2, c"grandchild1".as_ptr(), 5.9);
        add_node(5, 2, c"grandchild2".as_ptr(), 8.2);
        add_node(6, 3, c"grandchild3".as_ptr(), 12.4);
    }

    let node_id = (param1.wrapping_rem(6)).wrapping_add(1);
    let selected_node = unsafe { find_node_by_id(node_id) };

    if !selected_node.is_null() {
        unsafe {
            let name_ptr = (&raw mut (*selected_node).name) as *mut c_char;

            if *name_ptr != 0 {
                result = result.wrapping_add(process_string(name_ptr));
            }

            let subtree_sum = calculate_subtree_sum(node_id);

            let sum_as_int = safe_double_to_int(subtree_sum);
            result = result.wrapping_add(sum_as_int);
        }
    }

    let second_node_id = (param2.wrapping_rem(6)).wrapping_add(1);
    let second_node = unsafe { find_node_by_id(second_node_id) };

    if !second_node.is_null() {
        unsafe {
            let value_multiplied = (*second_node).value * param3 as c_double;

            let converted_value = safe_double_to_int(value_multiplied);
            result = result.wrapping_add(converted_value);
        }
    }

    let parent_id = (param4.wrapping_rem(3)).wrapping_add(1);
    let children = unsafe { get_children_count(parent_id) };
    result = result.wrapping_add(children.wrapping_mul(10));

    let mut calculation =
        (param1.wrapping_add(param2) as c_double) / (param3.wrapping_add(1) as c_double);
    calculation *= param4 as c_double;

    let final_calc = unsafe { safe_double_to_int(calculation) };
    result = result.wrapping_add(final_calc);

    result
}
