// Copyright 2025 MIT Lincoln Laboratory
//
// Rust translation of c_src/src/lib.c
// Preserves byte-identical behavior with the C implementation.
//
// All non-static C functions are exported with the same name and ABI
// so the resulting cdylib has the same public surface as the C build.

use std::ffi::{c_char, c_int};
use std::ptr;

#[allow(dead_code)]
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum Operation {
    Add = 1,
    Multiply = 2,
    Subtract = 3,
    Divide = 4,
    Modulo = 5,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TreeNode {
    pub id: c_int,
    pub value: c_int,
    pub parent_id: c_int,
    pub left_child_id: c_int,
    pub right_child_id: c_int,
    pub label: [c_char; 32],
}

const MAX_NODES: usize = 50;

static mut NODE_TABLE: [TreeNode; MAX_NODES] = [TreeNode {
    id: 0,
    value: 0,
    parent_id: 0,
    left_child_id: 0,
    right_child_id: 0,
    label: [0; 32],
}; MAX_NODES];

static mut NODE_COUNT: c_int = 0;

/// C type: `int (*)(int, int, int, int)`
pub type OperationFunc = extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[unsafe(no_mangle)]
pub extern "C" fn add_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_mul(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn subtract_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_sub(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn divide_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    // C's `/` truncates toward zero. Rust's `wrapping_div` matches that and
    // avoids panics on INT_MIN / -1 (which would be UB in C).
    a.wrapping_div(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn modulo_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a.wrapping_rem(b)
}

/// Returns a pointer to the matching node within the static table, or NULL
/// if no node has the given id. Mirrors the C signature `TreeNode*`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_node_by_id(id: c_int) -> *mut TreeNode {
    let count = NODE_COUNT as usize;
    let table_ptr: *mut TreeNode = (&raw mut NODE_TABLE) as *mut TreeNode;
    for i in 0..count {
        let entry = table_ptr.add(i);
        if (*entry).id == id {
            return entry;
        }
    }
    ptr::null_mut()
}

/// Replicates `strncpy(dst, src, 31)` then `dst[31] = '\0'`.
unsafe fn copy_label(dst: &mut [c_char; 32], src: *const c_char) {
    let mut i: usize = 0;
    let mut hit_nul = false;
    while i < 31 {
        if !hit_nul {
            let b = *src.add(i);
            dst[i] = b;
            if b == 0 {
                hit_nul = true;
            }
        } else {
            // strncpy zero-fills the remainder of the n-byte window after
            // it encounters NUL in the source.
            dst[i] = 0;
        }
        i += 1;
    }
    dst[31] = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_tree_node(
    id: c_int,
    value: c_int,
    parent_id: c_int,
    label: *const c_char,
) -> c_int {
    if NODE_COUNT >= MAX_NODES as c_int {
        return -1;
    }

    let idx = NODE_COUNT as usize;
    let table_ptr: *mut TreeNode = (&raw mut NODE_TABLE) as *mut TreeNode;
    let node = table_ptr.add(idx);
    (*node).id = id;
    (*node).value = value;
    (*node).parent_id = parent_id;
    (*node).left_child_id = -1;
    (*node).right_child_id = -1;
    copy_label(&mut (*node).label, label);

    if parent_id != -1 {
        let parent = find_node_by_id(parent_id);
        if parent.is_null() || (*parent).id != parent_id {
            return -1;
        }

        if (*parent).left_child_id == -1 {
            (*parent).left_child_id = id;
        } else if (*parent).right_child_id == -1 {
            (*parent).right_child_id = id;
        }
    }

    NODE_COUNT += 1;
    NODE_COUNT - 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn calculate_tree_sum(node_id: c_int) -> c_int {
    let node = find_node_by_id(node_id);
    if node.is_null() || (*node).id != node_id {
        return 0;
    }

    let value = (*node).value;
    let left = (*node).left_child_id;
    let right = (*node).right_child_id;

    let mut sum: c_int = value;
    if left != -1 {
        sum = sum.wrapping_add(calculate_tree_sum(left));
    }
    if right != -1 {
        sum = sum.wrapping_add(calculate_tree_sum(right));
    }
    sum
}

/// Replicates `strchr(s, c) != NULL`.
unsafe fn strchr_contains(s: *const c_char, c: c_char) -> bool {
    if s.is_null() {
        return false;
    }
    let mut i: usize = 0;
    loop {
        let b = *s.add(i);
        if b == c {
            // strchr matches the trailing NUL when c == 0; that's fine here.
            return true;
        }
        if b == 0 {
            return false;
        }
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_operation(op_str: *const c_char) -> c_int {
    // `op_str == NULL || strchr(op_str, '+') != NULL` -> OP_ADD
    if op_str.is_null() || strchr_contains(op_str, b'+' as c_char) {
        return Operation::Add as c_int;
    }
    if strchr_contains(op_str, b'*' as c_char) {
        return Operation::Multiply as c_int;
    }
    if strchr_contains(op_str, b'-' as c_char) {
        return Operation::Subtract as c_int;
    }
    if strchr_contains(op_str, b'/' as c_char) {
        return Operation::Divide as c_int;
    }
    if strchr_contains(op_str, b'%' as c_char) {
        return Operation::Modulo as c_int;
    }
    Operation::Add as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn get_operation_func(op: c_int) -> OperationFunc {
    match op {
        1 => add_op,
        2 => multiply_op,
        3 => subtract_op,
        4 => divide_op,
        5 => modulo_op,
        _ => add_op,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn inreftree(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    unsafe {
        NODE_COUNT = 0;

        let root_label = b"root\0".as_ptr() as *const c_char;
        let left_label = b"left\0".as_ptr() as *const c_char;
        let right_label = b"right\0".as_ptr() as *const c_char;
        let leftleft_label = b"left-left\0".as_ptr() as *const c_char;

        add_tree_node(1, param1, -1, root_label);
        add_tree_node(2, param2, 1, left_label);
        add_tree_node(3, param3, 1, right_label);
        add_tree_node(4, param4, 2, leftleft_label);

        let mut target_id: c_int = -1;
        let count = NODE_COUNT as usize;
        let table_ptr: *const TreeNode = (&raw const NODE_TABLE) as *const TreeNode;
        for i in 0..count {
            let label_ptr = (*table_ptr.add(i)).label.as_ptr();
            if strchr_contains(label_ptr, b'l' as c_char) {
                target_id = (*table_ptr.add(i)).id;
                break;
            }
        }

        let target = find_node_by_id(target_id);
        let need_reset = target.is_null() || (*target).value == 0;
        if need_reset {
            target_id = 1;
        }

        let tree_sum = calculate_tree_sum(1);

        let op_string: &[u8; 4] = b"+*-%";
        let idx_signed = tree_sum.wrapping_rem(4);
        // Mirrors `op_string[tree_sum % 4]`. C's `%` may yield negative
        // values for negative `tree_sum`, and indexing the literal at a
        // negative offset is undefined behaviour. We reproduce the
        // straightforward case (0..=3) verbatim and fall back to a
        // deterministic Euclidean-mod read for negative indices to avoid UB.
        let op_byte = if (0..4).contains(&idx_signed) {
            op_string[idx_signed as usize]
        } else {
            let m = idx_signed.rem_euclid(4) as usize;
            op_string[m]
        };
        let op_char: [c_char; 2] = [op_byte as c_char, 0];
        let op = parse_operation(op_char.as_ptr());

        let _op_value = op;

        let func = get_operation_func(op);
        func(tree_sum, target_id, 0, 0)
    }
}
