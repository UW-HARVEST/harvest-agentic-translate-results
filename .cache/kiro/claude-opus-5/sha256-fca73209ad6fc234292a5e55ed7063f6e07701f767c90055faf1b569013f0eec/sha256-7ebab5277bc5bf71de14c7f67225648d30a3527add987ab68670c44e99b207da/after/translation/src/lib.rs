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
//
// Rust translation of c_src/src/lib.c -- behaviour-preserving, including the
// original's quirks (truncating `%`, wrapping int arithmetic, redundant
// emptiness checks, and the NaN test placed *after* the range tests in
// safe_double_to_int).

use std::ffi::{c_char, c_double, c_int};

const MAX_NODES: usize = 100;
const MAX_NAME_LEN: usize = 50;

/// Mirror of the C `Node` struct; `#[repr(C)]` keeps the layout identical so
/// that pointers handed out by `find_node_by_id` stay ABI-compatible.
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
    /// Equivalent of C's zero-initialisation for static storage duration.
    const ZERO: Node = Node {
        id: 0,
        parent_id: 0,
        name: [0; MAX_NAME_LEN],
        value: 0.0,
        active: 0,
    };
}

static mut NODE_STORAGE: [Node; MAX_NODES] = [Node::ZERO; MAX_NODES];
static mut NODE_COUNT: c_int = 0;

/// `&mut` view over the file-scope storage. Single-threaded, exactly like the C.
#[inline]
unsafe fn storage() -> &'static mut [Node; MAX_NODES] {
    &mut *(&raw mut NODE_STORAGE)
}

#[inline]
unsafe fn node_count() -> c_int {
    *(&raw const NODE_COUNT)
}

#[inline]
unsafe fn set_node_count(v: c_int) {
    *(&raw mut NODE_COUNT) = v;
}

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

    // C builds a local `Node` with a partial designated initialiser, which
    // zero-fills every byte of the object -- padding included -- before the
    // named members are stored, and then copies the whole object into the
    // array. Zero the destination slot first so the padding bytes around
    // `name` and `active` match byte-for-byte.
    let slot: *mut Node = &raw mut storage()[count as usize];
    std::ptr::write_bytes(slot as *mut u8, 0, std::mem::size_of::<Node>());

    let new_node = &mut *slot;
    new_node.id = id;
    new_node.parent_id = parent_id;
    new_node.value = value;
    new_node.active = 1;

    // strncpy(new_node.name, name, MAX_NAME_LEN - 1); then force-terminate.
    // strncpy stops at the source NUL and zero-fills the remainder, which the
    // zeroing above already accounts for.
    for i in 0..(MAX_NAME_LEN - 1) {
        let ch = *name.add(i);
        if ch == 0 {
            break;
        }
        new_node.name[i] = ch;
    }
    new_node.name[MAX_NAME_LEN - 1] = 0;

    set_node_count(count.wrapping_add(1));
    node_count().wrapping_sub(1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_node_by_id(id: c_int) -> *mut Node {
    let count = node_count();
    let nodes = storage();
    let mut i: c_int = 0;
    while i < count {
        let idx = i as usize;
        if nodes[idx].id == id && nodes[idx].active != 0 {
            return &mut nodes[idx] as *mut Node;
        }
        i += 1;
    }
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_children_count(parent_id: c_int) -> c_int {
    let mut count: c_int = 0;
    let total = node_count();
    let nodes = storage();
    let mut i: c_int = 0;
    while i < total {
        let idx = i as usize;
        if nodes[idx].parent_id == parent_id && nodes[idx].active != 0 {
            count = count.wrapping_add(1);
        }
        i += 1;
    }
    count
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn calculate_subtree_sum(node_id: c_int) -> c_double {
    let node = find_node_by_id(node_id);
    if node.is_null() {
        return 0.0;
    }

    let mut sum: c_double = (*node).value;

    // Accumulation order is preserved so floating-point rounding matches.
    let total = node_count();
    let mut i: c_int = 0;
    while i < total {
        let idx = i as usize;
        let (child_parent, child_active, child_id) = {
            let nodes = storage();
            (nodes[idx].parent_id, nodes[idx].active, nodes[idx].id)
        };
        if child_parent == node_id && child_active != 0 {
            sum += calculate_subtree_sum(child_id);
        }
        i += 1;
    }

    sum
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_string(str: *mut c_char) -> c_int {
    let mut result: c_int = 0;
    let mut p = str;

    // The outer emptiness test is redundant in the original; kept as-is.
    if *p != 0 {
        while *p != 0 {
            result = result.wrapping_add(*p as c_int);
            p = p.add(1);
        }
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    if d > c_int::MAX as c_double {
        return c_int::MAX;
    }
    if d < c_int::MIN as c_double {
        return c_int::MIN;
    }

    // NaN reaches here because both comparisons above are false for NaN.
    if d != d {
        return 0;
    }

    // Remaining values are in range, so this truncates toward zero just like
    // the C cast.
    d as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn maxnmin(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int = 0;

    set_node_count(0);

    add_node(1, -1, c"root".as_ptr(), 10.5);
    add_node(2, 1, c"child1".as_ptr(), 20.7);
    add_node(3, 1, c"child2".as_ptr(), 15.3);
    add_node(4, 2, c"grandchild1".as_ptr(), 5.9);
    add_node(5, 2, c"grandchild2".as_ptr(), 8.2);
    add_node(6, 3, c"grandchild3".as_ptr(), 12.4);

    let node_id = (param1 % 6).wrapping_add(1);
    let selected_node = find_node_by_id(node_id);

    if !selected_node.is_null() {
        let name_ptr = (*selected_node).name.as_mut_ptr();

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
        param1.wrapping_add(param2) as c_double / param3.wrapping_add(1) as c_double;
    calculation *= param4 as c_double;

    let final_calc = safe_double_to_int(calculation);
    result = result.wrapping_add(final_calc);

    result
}
