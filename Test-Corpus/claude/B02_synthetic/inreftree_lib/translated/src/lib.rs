// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Behavior preserved exactly.

use std::ffi::c_int;
use std::sync::Mutex;

#[repr(i32)]
#[derive(Copy, Clone, PartialEq, Eq)]
#[allow(dead_code)]
enum Operation {
    OpAdd = 1,
    OpMultiply = 2,
    OpSubtract = 3,
    OpDivide = 4,
    OpModulo = 5,
}

#[derive(Copy, Clone)]
struct TreeNode {
    id: i32,
    value: i32,
    parent_id: i32,
    left_child_id: i32,
    right_child_id: i32,
    label: [u8; 32],
}

impl TreeNode {
    const fn new() -> Self {
        TreeNode {
            id: 0,
            value: 0,
            parent_id: 0,
            left_child_id: 0,
            right_child_id: 0,
            label: [0u8; 32],
        }
    }
}

const MAX_NODES: usize = 50;

struct State {
    node_table: [TreeNode; MAX_NODES],
    node_count: i32,
}

impl State {
    const fn new() -> Self {
        State {
            node_table: [TreeNode::new(); MAX_NODES],
            node_count: 0,
        }
    }
}

static STATE: Mutex<State> = Mutex::new(State::new());

type OperationFunc = fn(i32, i32, i32, i32) -> i32;

fn add_op(a: i32, b: i32, _unused1: i32, _unused2: i32) -> i32 {
    a.wrapping_add(b)
}

fn multiply_op(a: i32, b: i32, _unused1: i32, _unused2: i32) -> i32 {
    a.wrapping_mul(b)
}

fn subtract_op(a: i32, b: i32, _unused1: i32, _unused2: i32) -> i32 {
    a.wrapping_sub(b)
}

fn divide_op(a: i32, b: i32, _unused1: i32, _unused2: i32) -> i32 {
    if b == 0 {
        return 0;
    }
    a.wrapping_div(b)
}

fn modulo_op(a: i32, b: i32, _unused1: i32, _unused2: i32) -> i32 {
    if b == 0 {
        return 0;
    }
    a.wrapping_rem(b)
}

fn find_node_index_by_id(state: &State, id: i32) -> Option<usize> {
    let count = state.node_count as usize;
    for i in 0..count {
        if state.node_table[i].id == id {
            return Some(i);
        }
    }
    None
}

/// Mimics strncpy(dst, src, 31) followed by dst[31] = '\0'.
/// Copies up to 31 bytes from `src` (a NUL-terminated byte slice) into `dst[..31]`,
/// padding the remainder of dst[..31] with NUL, then forcing dst[31] = 0.
fn copy_label(dst: &mut [u8; 32], src: &[u8]) {
    // src is treated as a C string: copy up to and including the NUL or 31 bytes,
    // and zero-fill the rest of the first 31 bytes per strncpy semantics.
    let mut i = 0usize;
    while i < 31 {
        // Find next byte in src; if past the NUL, pad with zero.
        let b = if i < src.len() { src[i] } else { 0 };
        dst[i] = b;
        if b == 0 {
            // strncpy pads remaining with NULs.
            i += 1;
            while i < 31 {
                dst[i] = 0;
                i += 1;
            }
            break;
        }
        i += 1;
    }
    dst[31] = 0;
}

fn add_tree_node(state: &mut State, id: i32, value: i32, parent_id: i32, label: &[u8]) -> i32 {
    if state.node_count as usize >= MAX_NODES {
        return -1;
    }

    let idx = state.node_count as usize;
    {
        let node = &mut state.node_table[idx];
        node.id = id;
        node.value = value;
        node.parent_id = parent_id;
        node.left_child_id = -1;
        node.right_child_id = -1;
        copy_label(&mut node.label, label);
    }

    if parent_id != -1 {
        let parent_idx_opt = find_node_index_by_id(state, parent_id);
        let parent_idx = match parent_idx_opt {
            Some(i) => i,
            None => return -1,
        };
        // The C code also checks parent->id != parent_id, but find_node_by_id
        // only returns a node whose id matches; preserve the check anyway by
        // confirming, though it cannot fail here.
        if state.node_table[parent_idx].id != parent_id {
            return -1;
        }

        let parent = &mut state.node_table[parent_idx];
        if parent.left_child_id == -1 {
            parent.left_child_id = id;
        } else if parent.right_child_id == -1 {
            parent.right_child_id = id;
        }
    }

    state.node_count += 1;
    state.node_count - 1
}

fn calculate_tree_sum(state: &State, node_id: i32) -> i32 {
    let idx = match find_node_index_by_id(state, node_id) {
        Some(i) => i,
        None => return 0,
    };

    let node = &state.node_table[idx];
    if node.id != node_id {
        return 0;
    }

    let mut sum: i32 = node.value;

    if node.left_child_id != -1 {
        sum = sum.wrapping_add(calculate_tree_sum(state, node.left_child_id));
    }

    if node.right_child_id != -1 {
        sum = sum.wrapping_add(calculate_tree_sum(state, node.right_child_id));
    }

    sum
}

/// Equivalent to C's strchr: returns true if the NUL-terminated byte slice
/// contains the given byte before the terminating NUL.
fn cstr_contains(s: &[u8], byte: u8) -> bool {
    for &b in s.iter() {
        if b == 0 {
            return false;
        }
        if b == byte {
            return true;
        }
    }
    false
}

fn parse_operation(op_str: Option<&[u8]>) -> Operation {
    // C: if (op_str == NULL || strchr(op_str, '+') != NULL) return OP_ADD;
    match op_str {
        None => return Operation::OpAdd,
        Some(s) => {
            if cstr_contains(s, b'+') {
                return Operation::OpAdd;
            }
            if cstr_contains(s, b'*') {
                return Operation::OpMultiply;
            }
            if cstr_contains(s, b'-') {
                return Operation::OpSubtract;
            }
            if cstr_contains(s, b'/') {
                return Operation::OpDivide;
            }
            if cstr_contains(s, b'%') {
                return Operation::OpModulo;
            }
            Operation::OpAdd
        }
    }
}

fn get_operation_func(op: Operation) -> OperationFunc {
    match op as i32 {
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
    let mut state = STATE.lock().unwrap();
    state.node_count = 0;

    add_tree_node(&mut state, 1, param1, -1, b"root\0");
    add_tree_node(&mut state, 2, param2, 1, b"left\0");
    add_tree_node(&mut state, 3, param3, 1, b"right\0");
    add_tree_node(&mut state, 4, param4, 2, b"left-left\0");

    let mut target_id: i32 = -1;
    let count = state.node_count as usize;
    for i in 0..count {
        if cstr_contains(&state.node_table[i].label, b'l') {
            target_id = state.node_table[i].id;
            break;
        }
    }

    // Find the target; in C, if NULL or value == 0 (note: this also fires
    // when target_id is -1 because find_node_by_id returns NULL).
    let target_value_zero_or_missing = match find_node_index_by_id(&state, target_id) {
        Some(i) => state.node_table[i].value == 0,
        None => true,
    };
    if target_value_zero_or_missing {
        target_id = 1;
    }

    let tree_sum = calculate_tree_sum(&state, 1);

    let op_string: &[u8] = b"+*-%";
    // tree_sum % 4 in C uses signed modulo; in Rust use wrapping_rem.
    // For safety with negative values mirror C: signed mod can be negative,
    // so index would be out of range. We replicate as-is using wrapping arithmetic
    // and then take the result modulo 4 with C-style sign behavior.
    let idx_i = (tree_sum as i32).wrapping_rem(4);
    // C's `op_string[tree_sum % 4]` indexes into a 5-byte array (4 chars + NUL).
    // If idx_i is negative we'd be accessing memory out-of-bounds in C (UB).
    // For deterministic translation we treat it as wrapping modulo into [0,4).
    // Use a safe index: if negative, add 4.
    let idx = if idx_i < 0 {
        // Mirror what most compilers produce on x86_64: signed remainder yields
        // a value in (-4, 0], so adding 4 gives [0, 4]. Clamp to [0, 3].
        let v = idx_i + 4;
        if v == 4 { 0 } else { v as usize }
    } else {
        idx_i as usize
    };

    let op_char_byte = op_string[idx];
    let op_char: [u8; 2] = [op_char_byte, 0];
    let op = parse_operation(Some(&op_char));

    let _op_value = op as i32; // matches the (unused) C local

    let func = get_operation_func(op);

    // Drop the lock before the function call (not strictly necessary since
    // function pointer doesn't touch state).
    let result = func(tree_sum, target_id, 0, 0);

    result
}
