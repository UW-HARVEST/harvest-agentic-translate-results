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

// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Rust translation of `c_src/src/lib.c`.
//!
//! The C translation unit declares every helper with external linkage, so the
//! shared object exports them all. They are mirrored here with the same names,
//! signatures and observable behaviour.

#![allow(non_upper_case_globals)]

use std::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

// typedef enum { OP_ADD = 1, ... } Operation;
pub const OP_ADD: c_int = 1;
pub const OP_MULTIPLY: c_int = 2;
pub const OP_SUBTRACT: c_int = 3;
pub const OP_DIVIDE: c_int = 4;
pub const OP_MODULO: c_int = 5;

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

impl TreeNode {
    const fn zeroed() -> Self {
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

/// `typedef int (*OperationFunc)(int a, int b, int unused1, int unused2);`
pub type OperationFunc = Option<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int>;

const MAX_NODES: usize = 50;

// The C file defines these as tentative definitions with external linkage, so
// they live in .bss and are visible to the linker.
#[unsafe(no_mangle)]
pub static mut node_table: [TreeNode; MAX_NODES] = [TreeNode::zeroed(); MAX_NODES];

#[unsafe(no_mangle)]
pub static mut node_count: c_int = 0;

// ---------------------------------------------------------------------------
// Helpers mirroring the C library functions used internally
// ---------------------------------------------------------------------------

fn table() -> *mut TreeNode {
    // Raw pointer to the first element; avoids taking a reference to a
    // `static mut`.
    std::ptr::addr_of_mut!(node_table) as *mut TreeNode
}

fn get_node_count() -> c_int {
    unsafe { std::ptr::read(std::ptr::addr_of!(node_count)) }
}

fn set_node_count(value: c_int) {
    unsafe { std::ptr::write(std::ptr::addr_of_mut!(node_count), value) }
}

/// `strchr(s, c)` for a non-zero `c`: true when the NUL-terminated string `s`
/// contains the byte `c`.
unsafe fn c_str_contains(s: *const c_char, c: u8) -> bool {
    let mut p = s;
    loop {
        let b = *p as u8;
        if b == 0 {
            return false;
        }
        if b == c {
            return true;
        }
        p = p.add(1);
    }
}

// ---------------------------------------------------------------------------
// Arithmetic operations
// ---------------------------------------------------------------------------

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
    // C truncating division; INT_MIN / -1 overflows in C, mirror the
    // hardware-wrapping result rather than panicking.
    a.wrapping_div(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn modulo_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a.wrapping_rem(b)
}

// ---------------------------------------------------------------------------
// Tree table
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn find_node_by_id(id: c_int) -> *mut TreeNode {
    let base = table();
    let count = get_node_count();
    let mut i: c_int = 0;
    while i < count {
        unsafe {
            let node = base.add(i as usize);
            if (*node).id == id {
                return node;
            }
        }
        i += 1;
    }
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_tree_node(
    id: c_int,
    value: c_int,
    parent_id: c_int,
    label: *const c_char,
) -> c_int {
    let count = get_node_count();
    if count as usize >= MAX_NODES {
        return -1;
    }

    let node = table().add(count as usize);
    (*node).id = id;
    (*node).value = value;
    (*node).parent_id = parent_id;
    (*node).left_child_id = -1;
    (*node).right_child_id = -1;

    // strncpy(node->label, label, 31); node->label[31] = '\0';
    let mut hit_nul = false;
    for i in 0..31usize {
        let byte = if hit_nul { 0 } else { *label.add(i) };
        if byte == 0 {
            hit_nul = true;
        }
        (*node).label[i] = byte;
    }
    (*node).label[31] = 0;

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

    set_node_count(count + 1);
    get_node_count() - 1
}

#[unsafe(no_mangle)]
pub extern "C" fn calculate_tree_sum(node_id: c_int) -> c_int {
    let node = find_node_by_id(node_id);

    if node.is_null() || unsafe { (*node).id } != node_id {
        return 0;
    }

    let (value, left, right) = unsafe { ((*node).value, (*node).left_child_id, (*node).right_child_id) };

    let mut sum = value;

    if left != -1 {
        sum = sum.wrapping_add(calculate_tree_sum(left));
    }

    if right != -1 {
        sum = sum.wrapping_add(calculate_tree_sum(right));
    }

    sum
}

// ---------------------------------------------------------------------------
// Operation dispatch
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_operation(op_str: *const c_char) -> c_int {
    if op_str.is_null() || c_str_contains(op_str, b'+') {
        return OP_ADD;
    }
    if c_str_contains(op_str, b'*') {
        return OP_MULTIPLY;
    }
    if c_str_contains(op_str, b'-') {
        return OP_SUBTRACT;
    }
    if c_str_contains(op_str, b'/') {
        return OP_DIVIDE;
    }
    if c_str_contains(op_str, b'%') {
        return OP_MODULO;
    }
    OP_ADD
}

#[unsafe(no_mangle)]
pub extern "C" fn get_operation_func(op: c_int) -> OperationFunc {
    match op {
        1 => Some(add_op),
        2 => Some(multiply_op),
        3 => Some(subtract_op),
        4 => Some(divide_op),
        5 => Some(modulo_op),
        _ => Some(add_op),
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn inreftree(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    set_node_count(0);

    unsafe {
        add_tree_node(1, param1, -1, b"root\0".as_ptr() as *const c_char);
        add_tree_node(2, param2, 1, b"left\0".as_ptr() as *const c_char);
        add_tree_node(3, param3, 1, b"right\0".as_ptr() as *const c_char);
        add_tree_node(4, param4, 2, b"left-left\0".as_ptr() as *const c_char);
    }

    let mut target_id: c_int = -1;
    let count = get_node_count();
    let base = table();
    let mut i: c_int = 0;
    while i < count {
        unsafe {
            let node = base.add(i as usize);
            if c_str_contains((*node).label.as_ptr(), b'l') {
                target_id = (*node).id;
                break;
            }
        }
        i += 1;
    }

    let target = find_node_by_id(target_id);
    if target.is_null() || unsafe { (*target).value } == 0 {
        target_id = 1;
    }

    let tree_sum = calculate_tree_sum(1);

    // const char* op_string = "+*-%";
    // char op_char[2] = {op_string[tree_sum % 4], '\0'};
    //
    // C's `%` truncates toward zero, so a negative `tree_sum` yields a
    // negative index and the C code reads out of bounds ahead of the string
    // literal. In practice that byte is never one of "+*-%" (it is the NUL
    // terminator or a letter from a neighbouring literal), so
    // parse_operation() falls through to its OP_ADD default. Model that by
    // feeding an empty string, which takes the same path.
    const OP_STRING: [u8; 4] = [b'+', b'*', b'-', b'%'];
    let idx = tree_sum % 4;
    let selected: u8 = if (0..4).contains(&idx) {
        OP_STRING[idx as usize]
    } else {
        0
    };

    let op_char: [c_char; 2] = [selected as c_char, 0];
    let op = unsafe { parse_operation(op_char.as_ptr()) };

    let _op_value = op;

    let func = get_operation_func(op);

    let result = unsafe { (func.unwrap())(tree_sum, target_id, 0, 0) };

    result
}
