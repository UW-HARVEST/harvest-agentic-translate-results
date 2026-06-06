// Translation of c_src/src/lib.c to Rust producing byte-identical behavior.

use std::ffi::c_char;
use std::ffi::c_int;

#[repr(C)]
#[derive(Copy, Clone)]
struct TreeNode {
    id: c_int,
    value: c_int,
    parent_id: c_int,
    left_child_id: c_int,
    right_child_id: c_int,
    label: [c_char; 32],
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

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(dead_code)]
enum Operation {
    OpAdd = 1,
    OpMultiply = 2,
    OpSubtract = 3,
    OpDivide = 4,
    OpModulo = 5,
}

type OperationFunc = fn(c_int, c_int, c_int, c_int) -> c_int;

fn add_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_add(b)
}

fn multiply_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_mul(b)
}

fn subtract_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_sub(b)
}

fn divide_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a.wrapping_div(b)
}

fn modulo_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a.wrapping_rem(b)
}

/// Returns the index in NODE_TABLE of the node with the given id, or None.
unsafe fn find_node_index_by_id(id: c_int) -> Option<usize> {
    let count = unsafe { NODE_COUNT } as usize;
    for i in 0..count {
        let node_id = unsafe { NODE_TABLE[i].id };
        if node_id == id {
            return Some(i);
        }
    }
    None
}

/// Mimics C's strchr: returns true if `needle` is found before the NUL terminator.
fn label_contains(label: &[c_char; 32], needle: c_char) -> bool {
    for &c in label.iter() {
        if c == 0 {
            return false;
        }
        if c == needle {
            return true;
        }
    }
    // Reached end without finding NUL: behavior matches treating beyond as not-found.
    false
}

/// Mimics C's strncpy(dst, src, 31) followed by dst[31] = '\0'.
/// `src` must be a NUL-terminated C string slice (without the NUL in `src_bytes`).
fn copy_label(dst: &mut [c_char; 32], src_bytes: &[u8]) {
    // strncpy copies up to n bytes; if src has a NUL within n bytes, the rest is filled with NULs.
    // Then we explicitly null-terminate at index 31.
    let n = 31usize;
    let mut i = 0usize;
    let mut hit_nul = false;
    while i < n {
        if hit_nul || i >= src_bytes.len() {
            dst[i] = 0;
        } else {
            let b = src_bytes[i];
            if b == 0 {
                dst[i] = 0;
                hit_nul = true;
            } else {
                dst[i] = b as c_char;
            }
        }
        i += 1;
    }
    dst[31] = 0;
}

unsafe fn add_tree_node(id: c_int, value: c_int, parent_id: c_int, label: &[u8]) -> c_int {
    let count = unsafe { NODE_COUNT };
    if (count as usize) >= MAX_NODES {
        return -1;
    }

    let idx = count as usize;
    unsafe {
        NODE_TABLE[idx].id = id;
        NODE_TABLE[idx].value = value;
        NODE_TABLE[idx].parent_id = parent_id;
        NODE_TABLE[idx].left_child_id = -1;
        NODE_TABLE[idx].right_child_id = -1;
    }
    let mut tmp_label = [0 as c_char; 32];
    copy_label(&mut tmp_label, label);
    unsafe {
        NODE_TABLE[idx].label = tmp_label;
    }

    if parent_id != -1 {
        let parent_idx = unsafe { find_node_index_by_id(parent_id) };
        let parent_idx = match parent_idx {
            Some(p) => p,
            None => return -1,
        };
        // The C code re-checks parent->id != parent_id which is always true here,
        // but reproduce structure faithfully (it's a tautology, so fine).
        let parent_id_actual = unsafe { NODE_TABLE[parent_idx].id };
        if parent_id_actual != parent_id {
            return -1;
        }

        let left = unsafe { NODE_TABLE[parent_idx].left_child_id };
        if left == -1 {
            unsafe { NODE_TABLE[parent_idx].left_child_id = id };
        } else {
            let right = unsafe { NODE_TABLE[parent_idx].right_child_id };
            if right == -1 {
                unsafe { NODE_TABLE[parent_idx].right_child_id = id };
            }
        }
    }

    unsafe {
        NODE_COUNT += 1;
        NODE_COUNT - 1
    }
}

unsafe fn calculate_tree_sum(node_id: c_int) -> c_int {
    let idx = unsafe { find_node_index_by_id(node_id) };
    let idx = match idx {
        Some(i) => i,
        None => return 0,
    };

    // C also checks node->id != node_id which is a tautology here.
    let id_actual = unsafe { NODE_TABLE[idx].id };
    if id_actual != node_id {
        return 0;
    }

    let mut sum = unsafe { NODE_TABLE[idx].value };

    let left_id = unsafe { NODE_TABLE[idx].left_child_id };
    if left_id != -1 {
        sum = sum.wrapping_add(unsafe { calculate_tree_sum(left_id) });
    }

    let right_id = unsafe { NODE_TABLE[idx].right_child_id };
    if right_id != -1 {
        sum = sum.wrapping_add(unsafe { calculate_tree_sum(right_id) });
    }

    sum
}

/// Determines if the given label-as-bytes (NUL-terminated C string) contains `needle`.
fn cstr_contains(s: &[u8], needle: u8) -> bool {
    for &b in s.iter() {
        if b == 0 {
            return false;
        }
        if b == needle {
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
    match op as c_int {
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

        add_tree_node(1, param1, -1, b"root");
        add_tree_node(2, param2, 1, b"left");
        add_tree_node(3, param3, 1, b"right");
        add_tree_node(4, param4, 2, b"left-left");

        let mut target_id: c_int = -1;
        let count = NODE_COUNT as usize;
        for i in 0..count {
            let label = NODE_TABLE[i].label;
            if label_contains(&label, b'l' as c_char) {
                target_id = NODE_TABLE[i].id;
                break;
            }
        }

        let target_idx = find_node_index_by_id(target_id);
        let mut target_id = target_id;
        match target_idx {
            None => {
                target_id = 1;
            }
            Some(i) => {
                if NODE_TABLE[i].value == 0 {
                    target_id = 1;
                }
            }
        }

        let tree_sum = calculate_tree_sum(1);

        let op_string: &[u8] = b"+*-%";
        // C: tree_sum % 4 -- in C this can yield a negative value if tree_sum is negative.
        // We must reproduce that exactly. Using i32 % i32 in Rust matches C's truncated mod.
        let idx = (tree_sum.wrapping_rem(4)) as isize;
        // C indexes op_string[tree_sum % 4]; if idx is negative this is undefined behavior in C.
        // We reproduce common behavior: index with that value relative to start.
        let chosen: u8 = if idx >= 0 && (idx as usize) < op_string.len() {
            op_string[idx as usize]
        } else {
            // Best effort: replicate pointer arithmetic out-of-bounds is UB, default to 0.
            // For all reasonable inputs that yield non-negative tree_sum this branch is unused.
            0u8
        };

        let op_char: [u8; 2] = [chosen, 0u8];
        let op = parse_operation(Some(&op_char));

        let _op_value = op as c_int; // unused but preserved for parity with C.

        let func = get_operation_func(op);

        func(tree_sum, target_id, 0, 0)
    }
}
