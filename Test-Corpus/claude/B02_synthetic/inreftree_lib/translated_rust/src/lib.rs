// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Behavior preserved exactly.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::c_int;
use std::os::raw::c_char;

// Operation enum: in C this is just an int, so we use c_int as the public ABI.
pub type Operation = c_int;
pub const OP_ADD: Operation = 1;
pub const OP_MULTIPLY: Operation = 2;
pub const OP_SUBTRACT: Operation = 3;
pub const OP_DIVIDE: Operation = 4;
pub const OP_MODULO: Operation = 5;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct TreeNode {
    pub id: c_int,
    pub value: c_int,
    pub parent_id: c_int,
    pub left_child_id: c_int,
    pub right_child_id: c_int,
    pub label: [c_char; 32],
}

impl TreeNode {
    const fn new() -> Self {
        TreeNode {
            id: 0,
            value: 0,
            parent_id: 0,
            left_child_id: 0,
            right_child_id: 0,
            label: [0; 32],
        }
    }
}

pub const MAX_NODES: usize = 50;

// Global state matching C: `TreeNode node_table[MAX_NODES];` and `int node_count = 0;`
#[no_mangle]
pub static mut node_table: [TreeNode; MAX_NODES] = [TreeNode::new(); MAX_NODES];

#[no_mangle]
pub static mut node_count: c_int = 0;

pub type OperationFunc = extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[no_mangle]
pub extern "C" fn add_op(a: c_int, b: c_int, _u1: c_int, _u2: c_int) -> c_int {
    a.wrapping_add(b)
}

#[no_mangle]
pub extern "C" fn multiply_op(a: c_int, b: c_int, _u1: c_int, _u2: c_int) -> c_int {
    a.wrapping_mul(b)
}

#[no_mangle]
pub extern "C" fn subtract_op(a: c_int, b: c_int, _u1: c_int, _u2: c_int) -> c_int {
    a.wrapping_sub(b)
}

#[no_mangle]
pub extern "C" fn divide_op(a: c_int, b: c_int, _u1: c_int, _u2: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a.wrapping_div(b)
}

#[no_mangle]
pub extern "C" fn modulo_op(a: c_int, b: c_int, _u1: c_int, _u2: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a.wrapping_rem(b)
}

/// Returns true if the NUL-terminated byte slice contains `byte`. Mirrors strchr semantics
/// for "is this byte present before the NUL".
unsafe fn cstr_contains(mut p: *const c_char, byte: c_char) -> bool {
    loop {
        let b = *p;
        if b == 0 {
            return false;
        }
        if b == byte {
            return true;
        }
        p = p.add(1);
    }
}

/// Equivalent to C's strchr: returns pointer to the byte if present, NULL otherwise.
unsafe fn c_strchr(mut p: *const c_char, byte: c_char) -> *const c_char {
    loop {
        let b = *p;
        if b == byte {
            return p;
        }
        if b == 0 {
            return std::ptr::null();
        }
        p = p.add(1);
    }
}

/// Mimics strncpy(dst, src, 31) followed by dst[31] = '\0'.
unsafe fn copy_label(dst: *mut c_char, src: *const c_char) {
    let mut i: usize = 0;
    let mut hit_nul = false;
    while i < 31 {
        if !hit_nul {
            let b = *src.add(i);
            *dst.add(i) = b;
            if b == 0 {
                hit_nul = true;
            }
        } else {
            *dst.add(i) = 0;
        }
        i += 1;
    }
    *dst.add(31) = 0;
}

#[no_mangle]
pub unsafe extern "C" fn find_node_by_id(id: c_int) -> *mut TreeNode {
    let count = node_count;
    let table_ptr = (&raw mut node_table) as *mut TreeNode;
    for i in 0..count {
        let node = table_ptr.offset(i as isize);
        if (*node).id == id {
            return node;
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn add_tree_node(
    id: c_int,
    value: c_int,
    parent_id: c_int,
    label: *const c_char,
) -> c_int {
    if (node_count as usize) >= MAX_NODES {
        return -1;
    }

    let table_ptr = (&raw mut node_table) as *mut TreeNode;
    let node = table_ptr.offset(node_count as isize);
    (*node).id = id;
    (*node).value = value;
    (*node).parent_id = parent_id;
    (*node).left_child_id = -1;
    (*node).right_child_id = -1;
    copy_label((*node).label.as_mut_ptr(), label);

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

    node_count += 1;
    node_count - 1
}

#[no_mangle]
pub unsafe extern "C" fn calculate_tree_sum(node_id: c_int) -> c_int {
    let node = find_node_by_id(node_id);
    if node.is_null() || (*node).id != node_id {
        return 0;
    }

    let mut sum: c_int = (*node).value;

    if (*node).left_child_id != -1 {
        sum = sum.wrapping_add(calculate_tree_sum((*node).left_child_id));
    }

    if (*node).right_child_id != -1 {
        sum = sum.wrapping_add(calculate_tree_sum((*node).right_child_id));
    }

    sum
}

#[no_mangle]
pub unsafe extern "C" fn parse_operation(op_str: *const c_char) -> Operation {
    if op_str.is_null() || !c_strchr(op_str, b'+' as c_char).is_null() {
        return OP_ADD;
    }
    if !c_strchr(op_str, b'*' as c_char).is_null() {
        return OP_MULTIPLY;
    }
    if !c_strchr(op_str, b'-' as c_char).is_null() {
        return OP_SUBTRACT;
    }
    if !c_strchr(op_str, b'/' as c_char).is_null() {
        return OP_DIVIDE;
    }
    if !c_strchr(op_str, b'%' as c_char).is_null() {
        return OP_MODULO;
    }
    OP_ADD
}

#[no_mangle]
pub extern "C" fn get_operation_func(op: Operation) -> Option<OperationFunc> {
    match op {
        1 => Some(add_op),
        2 => Some(multiply_op),
        3 => Some(subtract_op),
        4 => Some(divide_op),
        5 => Some(modulo_op),
        _ => Some(add_op),
    }
}

#[no_mangle]
pub unsafe extern "C" fn inreftree(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    node_count = 0;

    let root_label = b"root\0";
    let left_label = b"left\0";
    let right_label = b"right\0";
    let leftleft_label = b"left-left\0";

    add_tree_node(1, param1, -1, root_label.as_ptr() as *const c_char);
    add_tree_node(2, param2, 1, left_label.as_ptr() as *const c_char);
    add_tree_node(3, param3, 1, right_label.as_ptr() as *const c_char);
    add_tree_node(4, param4, 2, leftleft_label.as_ptr() as *const c_char);

    let mut target_id: c_int = -1;
    let count = node_count;
    let table_ptr = (&raw mut node_table) as *mut TreeNode;
    for i in 0..count {
        let n = table_ptr.offset(i as isize);
        if cstr_contains((*n).label.as_ptr(), b'l' as c_char) {
            target_id = (*n).id;
            break;
        }
    }

    let target = find_node_by_id(target_id);
    if target.is_null() || (*target).value == 0 {
        target_id = 1;
    }

    let tree_sum = calculate_tree_sum(1);

    // op_string includes the implicit C string terminator (5 bytes total).
    let op_string: &[u8; 5] = b"+*-%\0";
    // C: tree_sum % 4 — signed remainder. With negative tree_sum, indexing the
    // 5-byte array would be UB in C. We mirror what x86_64 produces (signed rem)
    // and bound to [0,4) to avoid panicking; in practice tree_sum will be >= 0
    // for typical inputs.
    let rem = tree_sum.wrapping_rem(4);
    let idx = if rem < 0 { (rem + 4) as usize } else { rem as usize };
    let op_char_byte = op_string[idx];
    let op_char: [c_char; 2] = [op_char_byte as c_char, 0];
    let op = parse_operation(op_char.as_ptr());

    let _op_value = op as c_int;

    let func = get_operation_func(op).expect("get_operation_func returned None");

    func(tree_sum, target_id, 0, 0)
}
