// Copyright 2025 MIT Lincoln Laboratory
//
// Rust translation of c_src/src/lib.c. Public symbol matches the C header
// (c_src/include/lib.h declares only `maxnmin`).

#![allow(static_mut_refs)]

use std::ffi::c_int;

const MAX_NODES: usize = 100;
const MAX_NAME_LEN: usize = 50;

#[derive(Clone, Copy)]
struct Node {
    id: c_int,
    parent_id: c_int,
    name: [u8; MAX_NAME_LEN],
    value: f64,
    active: c_int,
}

impl Node {
    const fn zeroed() -> Self {
        Self {
            id: 0,
            parent_id: 0,
            name: [0u8; MAX_NAME_LEN],
            value: 0.0,
            active: 0,
        }
    }
}

static mut NODE_STORAGE: [Node; MAX_NODES] = [Node::zeroed(); MAX_NODES];
static mut NODE_COUNT: usize = 0;

/// Mimics the original C `add_node`. The `name` argument here is a Rust byte
/// slice (the C version accepts `const char *`); since `add_node` is internal
/// (it isn't exposed in the C header), this signature change is invisible to
/// the public ABI.
fn add_node(id: c_int, parent_id: c_int, name: &[u8], value: f64) -> c_int {
    unsafe {
        if NODE_COUNT >= MAX_NODES {
            return -1;
        }

        let mut new_node = Node {
            id,
            parent_id,
            name: [0u8; MAX_NAME_LEN],
            value,
            active: 1,
        };

        // Replicate `strncpy(new_node.name, name, MAX_NAME_LEN - 1);`
        // followed by `new_node.name[MAX_NAME_LEN - 1] = '\0';`.
        // strncpy stops at the first NUL in `name`, copying at most
        // MAX_NAME_LEN - 1 bytes. The destination is already zero-initialized,
        // matching strncpy's NUL-padding behavior on shorter inputs.
        let mut i = 0usize;
        while i < MAX_NAME_LEN - 1 && i < name.len() && name[i] != 0 {
            new_node.name[i] = name[i];
            i += 1;
        }
        new_node.name[MAX_NAME_LEN - 1] = 0;

        NODE_STORAGE[NODE_COUNT] = new_node;
        NODE_COUNT += 1;
        (NODE_COUNT as c_int) - 1
    }
}

/// Returns the index of the matching node (the C version returns a pointer).
fn find_node_by_id(id: c_int) -> Option<usize> {
    unsafe {
        for i in 0..NODE_COUNT {
            if NODE_STORAGE[i].id == id && NODE_STORAGE[i].active != 0 {
                return Some(i);
            }
        }
    }
    None
}

fn get_children_count(parent_id: c_int) -> c_int {
    let mut count: c_int = 0;
    unsafe {
        for i in 0..NODE_COUNT {
            if NODE_STORAGE[i].parent_id == parent_id && NODE_STORAGE[i].active != 0 {
                count = count.wrapping_add(1);
            }
        }
    }
    count
}

fn calculate_subtree_sum(node_id: c_int) -> f64 {
    let idx = match find_node_by_id(node_id) {
        Some(i) => i,
        None => return 0.0,
    };

    // SAFETY: `idx` was just produced from the live segment of NODE_STORAGE.
    let mut sum: f64 = unsafe { NODE_STORAGE[idx].value };

    unsafe {
        let count = NODE_COUNT;
        for i in 0..count {
            if NODE_STORAGE[i].parent_id == node_id && NODE_STORAGE[i].active != 0 {
                let child_id = NODE_STORAGE[i].id;
                sum += calculate_subtree_sum(child_id);
            }
        }
    }

    sum
}

/// Walks bytes until a NUL terminator and sums each byte cast to `int`.
/// On x86-64 Linux `char` is signed, so we sign-extend via `i8` to match
/// the C semantics for any high-bit-set bytes.
fn process_string(bytes: &[u8]) -> c_int {
    let mut result: c_int = 0;

    if !bytes.is_empty() && bytes[0] != 0 {
        let mut i = 0usize;
        while i < bytes.len() && bytes[i] != 0 {
            let signed = bytes[i] as i8 as c_int;
            result = result.wrapping_add(signed);
            i += 1;
        }
    }

    result
}

fn safe_double_to_int(d: f64) -> c_int {
    if d > c_int::MAX as f64 {
        return c_int::MAX;
    }
    if d < c_int::MIN as f64 {
        return c_int::MIN;
    }
    // NaN check: `d != d` is true only for NaN.
    if d != d {
        return 0;
    }

    // Rust's `as i32` truncates toward zero for in-range values, which
    // matches C's `(int)d` semantics. The earlier comparisons exclude
    // out-of-range and NaN inputs.
    d as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn maxnmin(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    unsafe {
        NODE_COUNT = 0;
    }

    add_node(1, -1, b"root", 10.5);
    add_node(2, 1, b"child1", 20.7);
    add_node(3, 1, b"child2", 15.3);
    add_node(4, 2, b"grandchild1", 5.9);
    add_node(5, 2, b"grandchild2", 8.2);
    add_node(6, 3, b"grandchild3", 12.4);

    let node_id = (param1 % 6).wrapping_add(1);
    let selected = find_node_by_id(node_id);

    if let Some(idx) = selected {
        // Snapshot the name bytes so we don't hold a reference to the static
        // across the recursive call below.
        let name_copy: [u8; MAX_NAME_LEN] = unsafe { NODE_STORAGE[idx].name };

        if name_copy[0] != 0 {
            result = result.wrapping_add(process_string(&name_copy));
        }

        let subtree_sum = calculate_subtree_sum(node_id);

        let sum_as_int = safe_double_to_int(subtree_sum);
        result = result.wrapping_add(sum_as_int);
    }

    let second_node_id = (param2 % 6).wrapping_add(1);
    let second = find_node_by_id(second_node_id);

    if let Some(idx) = second {
        let node_value: f64 = unsafe { NODE_STORAGE[idx].value };
        let value_multiplied = node_value * (param3 as f64);

        let converted_value = safe_double_to_int(value_multiplied);
        result = result.wrapping_add(converted_value);
    }

    let parent_id = (param4 % 3).wrapping_add(1);
    let children = get_children_count(parent_id);
    result = result.wrapping_add(children.wrapping_mul(10));

    // Match C: `(double)(param1 + param2) / (double)(param3 + 1)` performs the
    // integer additions first, then casts to double.
    let numerator = param1.wrapping_add(param2) as f64;
    let denominator = param3.wrapping_add(1) as f64;
    let mut calculation = numerator / denominator;
    calculation *= param4 as f64;

    let final_calc = safe_double_to_int(calculation);
    result = result.wrapping_add(final_calc);

    result
}
