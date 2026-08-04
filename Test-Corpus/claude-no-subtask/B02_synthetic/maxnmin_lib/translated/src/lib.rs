// Copyright 2025 MIT Lincoln Laboratory
// Rust translation preserving byte-identical behavior of the original C library.

use std::ffi::{c_char, c_double, c_int};

const MAX_NODES: usize = 100;
const MAX_NAME_LEN: usize = 50;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Node {
    id: c_int,
    parent_id: c_int,
    name: [c_char; MAX_NAME_LEN],
    value: c_double,
    active: c_int,
}

impl Node {
    const fn zeroed() -> Self {
        Node {
            id: 0,
            parent_id: 0,
            name: [0; MAX_NAME_LEN],
            value: 0.0,
            active: 0,
        }
    }
}

static mut NODE_STORAGE: [Node; MAX_NODES] = [Node::zeroed(); MAX_NODES];
static mut NODE_COUNT: c_int = 0;

/// Mimic C strncpy: copy up to n bytes of src into dst, padding with zeros if
/// src is shorter, NOT null-terminating if src is at least n bytes long.
#[inline]
unsafe fn strncpy_c(dst: *mut c_char, src: *const c_char, n: usize) {
    let mut i = 0usize;
    let mut src_done = false;
    while i < n {
        let b = if src_done { 0 } else { *src.add(i) };
        *dst.add(i) = b;
        if b == 0 {
            src_done = true;
        }
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_node(
    id: c_int,
    parent_id: c_int,
    name: *const c_char,
    value: c_double,
) -> c_int {
    if NODE_COUNT >= MAX_NODES as c_int {
        return -1;
    }

    let mut new_node = Node {
        id,
        parent_id,
        name: [0; MAX_NAME_LEN],
        value,
        active: 1,
    };

    strncpy_c(new_node.name.as_mut_ptr(), name, MAX_NAME_LEN - 1);
    new_node.name[MAX_NAME_LEN - 1] = 0;

    let idx = NODE_COUNT as usize;
    NODE_STORAGE[idx] = new_node;
    NODE_COUNT += 1;
    NODE_COUNT - 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_node_by_id(id: c_int) -> *mut Node {
    let count = NODE_COUNT as usize;
    for i in 0..count {
        if NODE_STORAGE[i].id == id && NODE_STORAGE[i].active != 0 {
            return &mut NODE_STORAGE[i] as *mut Node;
        }
    }
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_children_count(parent_id: c_int) -> c_int {
    let mut count: c_int = 0;
    let n = NODE_COUNT as usize;
    for i in 0..n {
        if NODE_STORAGE[i].parent_id == parent_id && NODE_STORAGE[i].active != 0 {
            count += 1;
        }
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

    let n = NODE_COUNT as usize;
    for i in 0..n {
        if NODE_STORAGE[i].parent_id == node_id && NODE_STORAGE[i].active != 0 {
            sum += calculate_subtree_sum(NODE_STORAGE[i].id);
        }
    }

    sum
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_string(str_ptr: *mut c_char) -> c_int {
    let mut result: c_int = 0;
    let mut p = str_ptr;

    if *p != 0 {
        while *p != 0 {
            // In C, char is signed on x86_64 Linux; (int)(*str) sign-extends.
            result += (*p) as c_int;
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

    // NaN check (d != d)
    if d != d {
        return 0;
    }

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

    NODE_COUNT = 0;

    let root = b"root\0";
    let child1 = b"child1\0";
    let child2 = b"child2\0";
    let grandchild1 = b"grandchild1\0";
    let grandchild2 = b"grandchild2\0";
    let grandchild3 = b"grandchild3\0";

    add_node(1, -1, root.as_ptr() as *const c_char, 10.5);
    add_node(2, 1, child1.as_ptr() as *const c_char, 20.7);
    add_node(3, 1, child2.as_ptr() as *const c_char, 15.3);
    add_node(4, 2, grandchild1.as_ptr() as *const c_char, 5.9);
    add_node(5, 2, grandchild2.as_ptr() as *const c_char, 8.2);
    add_node(6, 3, grandchild3.as_ptr() as *const c_char, 12.4);

    let node_id = (param1 % 6) + 1;
    let selected_node = find_node_by_id(node_id);

    if !selected_node.is_null() {
        let name_ptr = (*selected_node).name.as_mut_ptr();

        if *name_ptr != 0 {
            result += process_string(name_ptr);
        }

        let subtree_sum = calculate_subtree_sum(node_id);

        let sum_as_int = safe_double_to_int(subtree_sum);
        result += sum_as_int;
    }

    let second_node_id = (param2 % 6) + 1;
    let second_node = find_node_by_id(second_node_id);

    if !second_node.is_null() {
        // C: second_node->value * param3 — param3 (int) is implicitly promoted to double.
        let value_multiplied: c_double = (*second_node).value * (param3 as c_double);

        let converted_value = safe_double_to_int(value_multiplied);
        result += converted_value;
    }

    let parent_id = (param4 % 3) + 1;
    let children = get_children_count(parent_id);
    result += children * 10;

    // C: (double)(param1 + param2) / (double)(param3 + 1)
    // The additions happen in int (with C's wrapping/UB on overflow); we preserve that
    // by using wrapping_add to avoid Rust panic on overflow in debug builds.
    let numerator: c_double = param1.wrapping_add(param2) as c_double;
    let denominator: c_double = param3.wrapping_add(1) as c_double;
    let mut calculation: c_double = numerator / denominator;
    calculation *= param4 as c_double;

    let final_calc = safe_double_to_int(calculation);
    result += final_calc;

    result
}
