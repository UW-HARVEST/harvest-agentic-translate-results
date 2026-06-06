// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust, preserving exact behavior.

use std::os::raw::c_int;

const MAX_NODES: usize = 100;
const MAX_NAME_LEN: usize = 50;

#[derive(Copy, Clone)]
struct Node {
    id: c_int,
    parent_id: c_int,
    name: [u8; MAX_NAME_LEN],
    value: f64,
    active: c_int,
}

impl Node {
    const fn zero() -> Self {
        Node {
            id: 0,
            parent_id: 0,
            name: [0u8; MAX_NAME_LEN],
            value: 0.0,
            active: 0,
        }
    }
}

struct Storage {
    nodes: [Node; MAX_NODES],
    count: usize,
}

static mut STORAGE: Storage = Storage {
    nodes: [Node::zero(); MAX_NODES],
    count: 0,
};

unsafe fn add_node(id: c_int, parent_id: c_int, name: &[u8], value: f64) -> c_int {
    let storage = unsafe { &mut *(&raw mut STORAGE) };
    if storage.count >= MAX_NODES {
        return -1;
    }

    let mut new_node = Node {
        id,
        parent_id,
        name: [0u8; MAX_NAME_LEN],
        value,
        active: 1,
    };

    // strncpy semantics: copy up to MAX_NAME_LEN - 1 bytes from name (stopping at NUL),
    // then ensure terminator at MAX_NAME_LEN - 1.
    let mut i = 0usize;
    while i < MAX_NAME_LEN - 1 && i < name.len() && name[i] != 0 {
        new_node.name[i] = name[i];
        i += 1;
    }
    // remaining bytes of new_node.name are already 0
    new_node.name[MAX_NAME_LEN - 1] = 0;

    storage.nodes[storage.count] = new_node;
    storage.count += 1;
    (storage.count as c_int) - 1
}

unsafe fn find_node_by_id(id: c_int) -> Option<usize> {
    let storage = unsafe { &*(&raw const STORAGE) };
    for i in 0..storage.count {
        if storage.nodes[i].id == id && storage.nodes[i].active != 0 {
            return Some(i);
        }
    }
    None
}

unsafe fn get_children_count(parent_id: c_int) -> c_int {
    let storage = unsafe { &*(&raw const STORAGE) };
    let mut count: c_int = 0;
    for i in 0..storage.count {
        if storage.nodes[i].parent_id == parent_id && storage.nodes[i].active != 0 {
            count += 1;
        }
    }
    count
}

unsafe fn calculate_subtree_sum(node_id: c_int) -> f64 {
    let idx = unsafe { find_node_by_id(node_id) };
    let idx = match idx {
        Some(i) => i,
        None => return 0.0,
    };

    let storage_ptr: *const Storage = &raw const STORAGE;
    let mut sum = unsafe { (*storage_ptr).nodes[idx].value };

    let count = unsafe { (*storage_ptr).count };
    for i in 0..count {
        let (pid, active, id_i) = unsafe {
            (
                (*storage_ptr).nodes[i].parent_id,
                (*storage_ptr).nodes[i].active,
                (*storage_ptr).nodes[i].id,
            )
        };
        if pid == node_id && active != 0 {
            sum += unsafe { calculate_subtree_sum(id_i) };
        }
    }

    sum
}

fn process_string(bytes: &[u8]) -> c_int {
    let mut result: c_int = 0;
    if !bytes.is_empty() && bytes[0] != 0 {
        let mut i = 0usize;
        while i < bytes.len() && bytes[i] != 0 {
            // C: result += (int)(*str)
            // char on most platforms is signed; (int)(char) sign-extends.
            let c = bytes[i] as i8 as c_int;
            result = result.wrapping_add(c);
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
    if d != d {
        return 0;
    }
    // C cast truncates toward zero for in-range values; Rust `as i32` saturates.
    // For values within [INT_MIN, INT_MAX], saturating == truncating.
    d as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn maxnmin(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    unsafe {
        let mut result: c_int = 0;

        // Reset count
        {
            let storage = &mut *(&raw mut STORAGE);
            storage.count = 0;
        }

        add_node(1, -1, b"root", 10.5);
        add_node(2, 1, b"child1", 20.7);
        add_node(3, 1, b"child2", 15.3);
        add_node(4, 2, b"grandchild1", 5.9);
        add_node(5, 2, b"grandchild2", 8.2);
        add_node(6, 3, b"grandchild3", 12.4);

        let node_id = (param1 % 6).wrapping_add(1);
        let selected_idx = find_node_by_id(node_id);

        if let Some(idx) = selected_idx {
            let storage_ptr: *const Storage = &raw const STORAGE;
            let name_bytes = (*storage_ptr).nodes[idx].name;

            if name_bytes[0] != 0 {
                result = result.wrapping_add(process_string(&name_bytes));
            }

            let subtree_sum = calculate_subtree_sum(node_id);
            let sum_as_int = safe_double_to_int(subtree_sum);
            result = result.wrapping_add(sum_as_int);
        }

        let second_node_id = (param2 % 6).wrapping_add(1);
        let second_idx = find_node_by_id(second_node_id);

        if let Some(idx) = second_idx {
            let storage_ptr: *const Storage = &raw const STORAGE;
            let value = (*storage_ptr).nodes[idx].value;
            // C: second_node->value * param3 -> double * int -> double
            let value_multiplied = value * (param3 as f64);
            let converted_value = safe_double_to_int(value_multiplied);
            result = result.wrapping_add(converted_value);
        }

        let parent_id = (param4 % 3).wrapping_add(1);
        let children = get_children_count(parent_id);
        result = result.wrapping_add(children.wrapping_mul(10));

        // C: (double)(param1 + param2) / (double)(param3 + 1)
        // The additions happen as int (with possible overflow), then cast to double.
        let sum12 = param1.wrapping_add(param2);
        let denom = param3.wrapping_add(1);
        let mut calculation = (sum12 as f64) / (denom as f64);
        calculation *= param4 as f64;

        let final_calc = safe_double_to_int(calculation);
        result = result.wrapping_add(final_calc);

        result
    }
}
