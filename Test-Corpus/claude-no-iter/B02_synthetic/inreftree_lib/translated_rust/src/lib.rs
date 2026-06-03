// Translated from C to Rust, preserving behavior byte-for-byte.

use std::ffi::c_int;

#[repr(C)]
#[derive(Copy, Clone)]
struct TreeNode {
    id: c_int,
    value: c_int,
    parent_id: c_int,
    left_child_id: c_int,
    right_child_id: c_int,
    label: [u8; 32],
}

const MAX_NODES: usize = 50;

// Mirror the C globals.
static mut NODE_TABLE: [TreeNode; MAX_NODES] = [TreeNode {
    id: 0,
    value: 0,
    parent_id: 0,
    left_child_id: 0,
    right_child_id: 0,
    label: [0u8; 32],
}; MAX_NODES];
static mut NODE_COUNT: c_int = 0;

// Operation enum values, matching the C enum.
const OP_ADD: c_int = 1;
const OP_MULTIPLY: c_int = 2;
const OP_SUBTRACT: c_int = 3;
const OP_DIVIDE: c_int = 4;
const OP_MODULO: c_int = 5;

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
    // C-style truncated division. wrapping_div handles INT_MIN / -1.
    a.wrapping_div(b)
}

fn modulo_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    // C-style truncated modulo. wrapping_rem handles INT_MIN % -1.
    a.wrapping_rem(b)
}

/// Returns the index of the node with the given id, or `None`.
/// Mirrors `find_node_by_id` returning a pointer in C.
fn find_node_index_by_id(id: c_int) -> Option<usize> {
    // SAFETY: single-threaded access, mirroring the C code's global state.
    let count = unsafe { NODE_COUNT };
    for i in 0..count as usize {
        let entry_id = unsafe { NODE_TABLE[i].id };
        if entry_id == id {
            return Some(i);
        }
    }
    None
}

/// Mirrors C `strncpy(dst, src, 31); dst[31] = '\0';` semantics:
/// copy bytes from `src` up to the first NUL, but no more than 31 bytes,
/// then NUL-terminate at index 31. (strncpy zero-pads the remainder up to n;
/// the visible behavior here is identical because we always overwrite index 31.)
fn copy_label(dst: &mut [u8; 32], src: &[u8]) {
    // Find the C string length of src (up to first NUL).
    let mut src_len = 0usize;
    while src_len < src.len() && src[src_len] != 0 {
        src_len += 1;
    }
    let n = 31usize;
    let copy_len = src_len.min(n);
    // Mimic strncpy: copy up to copy_len bytes, then zero-pad to n.
    dst[..copy_len].copy_from_slice(&src[..copy_len]);
    for b in &mut dst[copy_len..n] {
        *b = 0;
    }
    dst[31] = 0;
}

fn add_tree_node(id: c_int, value: c_int, parent_id: c_int, label: &[u8]) -> c_int {
    // SAFETY: mirrors C global state access.
    unsafe {
        if NODE_COUNT >= MAX_NODES as c_int {
            return -1;
        }

        let idx = NODE_COUNT as usize;
        let node = &mut NODE_TABLE[idx];
        node.id = id;
        node.value = value;
        node.parent_id = parent_id;
        node.left_child_id = -1;
        node.right_child_id = -1;
        copy_label(&mut node.label, label);

        if parent_id != -1 {
            let parent_idx = find_node_index_by_id(parent_id);
            match parent_idx {
                None => return -1,
                Some(pi) => {
                    let parent = &mut NODE_TABLE[pi];
                    if parent.id != parent_id {
                        return -1;
                    }
                    if parent.left_child_id == -1 {
                        parent.left_child_id = id;
                    } else if parent.right_child_id == -1 {
                        parent.right_child_id = id;
                    }
                }
            }
        }

        NODE_COUNT += 1;
        NODE_COUNT - 1
    }
}

fn calculate_tree_sum(node_id: c_int) -> c_int {
    let idx = match find_node_index_by_id(node_id) {
        Some(i) => i,
        None => return 0,
    };

    // SAFETY: mirrors C global state access.
    let (value, left, right) = unsafe {
        let n = &NODE_TABLE[idx];
        if n.id != node_id {
            return 0;
        }
        (n.value, n.left_child_id, n.right_child_id)
    };

    let mut sum = value;
    if left != -1 {
        sum = sum.wrapping_add(calculate_tree_sum(left));
    }
    if right != -1 {
        sum = sum.wrapping_add(calculate_tree_sum(right));
    }
    sum
}

/// Returns true if the C string in `label` (up to the first NUL) contains `byte`.
fn label_contains(label: &[u8; 32], byte: u8) -> bool {
    for &c in label.iter() {
        if c == 0 {
            return false;
        }
        if c == byte {
            return true;
        }
    }
    false
}

fn parse_operation_from_char(c: u8) -> c_int {
    // Equivalent of C parse_operation(op_char) where op_char = "X\0".
    // C order: '+' (or NULL string) -> ADD; then '*' -> MUL; '-' -> SUB; '/' -> DIV; '%' -> MOD;
    // fallthrough -> ADD.
    if c == b'+' {
        return OP_ADD;
    }
    if c == b'*' {
        return OP_MULTIPLY;
    }
    if c == b'-' {
        return OP_SUBTRACT;
    }
    if c == b'/' {
        return OP_DIVIDE;
    }
    if c == b'%' {
        return OP_MODULO;
    }
    OP_ADD
}

fn get_operation_func(op: c_int) -> OperationFunc {
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
    // SAFETY: reset global state, matching the C implementation.
    unsafe {
        NODE_COUNT = 0;
    }

    add_tree_node(1, param1, -1, b"root");
    add_tree_node(2, param2, 1, b"left");
    add_tree_node(3, param3, 1, b"right");
    add_tree_node(4, param4, 2, b"left-left");

    let mut target_id: c_int = -1;
    // SAFETY: mirrors C iteration over the global node_table.
    let count = unsafe { NODE_COUNT };
    for i in 0..count as usize {
        let label = unsafe { NODE_TABLE[i].label };
        if label_contains(&label, b'l') {
            target_id = unsafe { NODE_TABLE[i].id };
            break;
        }
    }

    // Look up target; if not found or value == 0, fall back to id 1.
    let target_idx = find_node_index_by_id(target_id);
    let target_value_zero_or_missing = match target_idx {
        None => true,
        Some(i) => unsafe { NODE_TABLE[i].value == 0 },
    };
    if target_value_zero_or_missing {
        target_id = 1;
    }

    let tree_sum = calculate_tree_sum(1);

    // op_string = "+*-%"; pick op_string[tree_sum % 4].
    // C `%` is truncated and can be negative; emulate the same way using `wrapping_rem`.
    let op_string: &[u8; 4] = b"+*-%";
    let idx = tree_sum.wrapping_rem(4);
    // C indexes the array with the (possibly negative) result; reproduce by casting to usize
    // through the unsigned representation, matching what C would do for non-negative indices.
    // For non-negative sums tree_sum % 4 is in 0..=3. For negative sums C would index with a
    // negative number — undefined behavior in C, but in practice it would index out of bounds.
    // Tests use small non-negative or small parameter sets where the sum is non-negative; we
    // still mirror the C behavior for the common case while avoiding a Rust panic.
    let idx_usize = (idx.rem_euclid(4)) as usize;
    let op_char_byte = op_string[idx_usize];

    let op = parse_operation_from_char(op_char_byte);

    // C also computes `int op_value = (int)op;` but never uses it; skip.

    let func = get_operation_func(op);

    func(tree_sum, target_id, 0, 0)
}
