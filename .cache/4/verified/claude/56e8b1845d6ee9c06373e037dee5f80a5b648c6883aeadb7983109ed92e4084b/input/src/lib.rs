// Rust translation of c_src/src/lib.c (MIT Lincoln Laboratory, 2025).
//
// The translation preserves the exact public ABI of the C shared library:
//   functions: add_op, multiply_op, subtract_op, divide_op, modulo_op,
//              find_node_by_id, add_tree_node, calculate_tree_sum,
//              parse_operation, get_operation_func, inreftree
//   data:      node_table, node_count
//
// Behaviour (including quirks/bugs) is reproduced exactly.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------
// typedef enum { OP_ADD = 1, ... } Operation;
// ---------------------------------------------------------------------------

pub const OP_ADD: c_int = 1;
pub const OP_MULTIPLY: c_int = 2;
pub const OP_SUBTRACT: c_int = 3;
pub const OP_DIVIDE: c_int = 4;
pub const OP_MODULO: c_int = 5;

// ---------------------------------------------------------------------------
// typedef struct { ... } TreeNode;
// ---------------------------------------------------------------------------

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

const MAX_NODES: usize = 50;

const EMPTY_NODE: TreeNode = TreeNode {
    id: 0,
    value: 0,
    parent_id: 0,
    left_child_id: 0,
    right_child_id: 0,
    label: [0; 32],
};

/// `TreeNode node_table[MAX_NODES];` (global, exported, zero initialised)
#[unsafe(no_mangle)]
pub static mut node_table: [TreeNode; MAX_NODES] = [EMPTY_NODE; MAX_NODES];

/// `int node_count = 0;` (global, exported)
#[unsafe(no_mangle)]
pub static mut node_count: c_int = 0;

/// typedef int (*OperationFunc)(int a, int b, int unused1, int unused2);
pub type OperationFunc = Option<extern "C" fn(c_int, c_int, c_int, c_int) -> c_int>;

// ---------------------------------------------------------------------------
// Small helpers reproducing the libc routines used by the C code.
// ---------------------------------------------------------------------------

/// Pointer to the first element of `node_table`, without creating a reference
/// to the mutable static.
#[inline]
fn table_ptr() -> *mut TreeNode {
    (&raw mut node_table) as *mut TreeNode
}

/// `strchr(s, c)`: returns a pointer to the first occurrence of `c` in the
/// NUL-terminated string `s`, or NULL. Only used for non-NUL needles here, so
/// the terminator itself never matches.
#[inline]
unsafe fn c_strchr(s: *const c_char, c: u8) -> *const c_char {
    let mut p = s;
    loop {
        let ch = *p as u8;
        if ch == c {
            return p;
        }
        if ch == 0 {
            return std::ptr::null();
        }
        p = p.add(1);
    }
}

/// `strncpy(dst, src, n)`: copies at most `n` bytes from `src`, stopping after
/// the terminating NUL, and zero-pads the remainder of the `n` bytes.
#[inline]
unsafe fn c_strncpy(dst: *mut c_char, src: *const c_char, n: usize) {
    let mut i = 0usize;
    while i < n {
        let ch = *src.add(i);
        *dst.add(i) = ch;
        i += 1;
        if ch == 0 {
            break;
        }
    }
    while i < n {
        *dst.add(i) = 0;
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// Operation implementations
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
// Tree handling
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn find_node_by_id(id: c_int) -> *mut TreeNode {
    unsafe {
        let base = table_ptr();
        let count = node_count;
        let mut i: c_int = 0;
        while i < count {
            let node = base.offset(i as isize);
            if (*node).id == id {
                return node;
            }
            i += 1;
        }
    }
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn add_tree_node(
    id: c_int,
    value: c_int,
    parent_id: c_int,
    label: *const c_char,
) -> c_int {
    unsafe {
        if node_count >= MAX_NODES as c_int {
            return -1;
        }

        let node = table_ptr().offset(node_count as isize);
        (*node).id = id;
        (*node).value = value;
        (*node).parent_id = parent_id;
        (*node).left_child_id = -1;
        (*node).right_child_id = -1;
        let label_ptr = (&raw mut (*node).label) as *mut c_char;
        c_strncpy(label_ptr, label, 31);
        *label_ptr.add(31) = 0;

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

        node_count = node_count.wrapping_add(1);
        node_count.wrapping_sub(1)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn calculate_tree_sum(node_id: c_int) -> c_int {
    unsafe {
        let node = find_node_by_id(node_id);

        if node.is_null() || (*node).id != node_id {
            return 0;
        }

        let mut sum = (*node).value;

        if (*node).left_child_id != -1 {
            sum = sum.wrapping_add(calculate_tree_sum((*node).left_child_id));
        }

        if (*node).right_child_id != -1 {
            sum = sum.wrapping_add(calculate_tree_sum((*node).right_child_id));
        }

        sum
    }
}

// ---------------------------------------------------------------------------
// Operation dispatch
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn parse_operation(op_str: *const c_char) -> c_int {
    unsafe {
        if op_str.is_null() || !c_strchr(op_str, b'+').is_null() {
            return OP_ADD;
        }
        if !c_strchr(op_str, b'*').is_null() {
            return OP_MULTIPLY;
        }
        if !c_strchr(op_str, b'-').is_null() {
            return OP_SUBTRACT;
        }
        if !c_strchr(op_str, b'/').is_null() {
            return OP_DIVIDE;
        }
        if !c_strchr(op_str, b'%').is_null() {
            return OP_MODULO;
        }
        OP_ADD
    }
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

// The C compiler places the string literals of lib.c contiguously in .rodata as
//     "root\0" "right\0" "left-left\0" "+*-%\0"
// with the literal "left" folded into the tail of "left-left".  `inreftree`
// indexes `op_string` with `tree_sum % 4`, which is negative for negative sums;
// this buffer reproduces the bytes that the C code reads in that case.
static RODATA: [u8; 26] = *b"root\0right\0left-left\0+*-%\0";
const OP_STRING_OFFSET: isize = 21; // index of "+*-%" within RODATA

#[unsafe(no_mangle)]
pub extern "C" fn inreftree(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    unsafe {
        node_count = 0;

        add_tree_node(1, param1, -1, b"root\0".as_ptr() as *const c_char);
        add_tree_node(2, param2, 1, b"left\0".as_ptr() as *const c_char);
        add_tree_node(3, param3, 1, b"right\0".as_ptr() as *const c_char);
        add_tree_node(4, param4, 2, b"left-left\0".as_ptr() as *const c_char);

        let mut target_id: c_int = -1;
        let base = table_ptr();
        let count = node_count;
        let mut i: c_int = 0;
        while i < count {
            let node = base.offset(i as isize);
            let label_ptr = (&raw const (*node).label) as *const c_char;
            if !c_strchr(label_ptr, b'l').is_null() {
                target_id = (*node).id;
                break;
            }
            i += 1;
        }

        let target = find_node_by_id(target_id);
        if target.is_null() || (*target).value == 0 {
            target_id = 1;
        }

        let tree_sum = calculate_tree_sum(1);

        let op_string = RODATA.as_ptr().offset(OP_STRING_OFFSET) as *const c_char;
        let op_char: [c_char; 2] = [
            *op_string.offset(tree_sum.wrapping_rem(4) as isize),
            0,
        ];
        let op = parse_operation(op_char.as_ptr());

        let _op_value = op;

        let func = get_operation_func(op);

        let result = (func.unwrap())(tree_sum, target_id, 0, 0);

        result
    }
}
