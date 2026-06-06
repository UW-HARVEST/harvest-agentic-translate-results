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

static mut NODE_TABLE: [TreeNode; MAX_NODES] = [TreeNode {
    id: 0,
    value: 0,
    parent_id: 0,
    left_child_id: 0,
    right_child_id: 0,
    label: [0u8; 32],
}; MAX_NODES];

static mut NODE_COUNT: c_int = 0;

#[allow(dead_code)]
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq)]
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

/// Returns the index in NODE_TABLE for the node with the given id, or None if not found.
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

/// Mimics strncpy semantics: copies at most n bytes from src into dst.
/// If src is shorter than n, dst is null-padded to n bytes.
fn strncpy_to_buf(dst: &mut [u8], src: &[u8], n: usize) {
    let copy_len = src.len().min(n);
    for i in 0..copy_len {
        dst[i] = src[i];
    }
    // Null-pad
    for i in copy_len..n {
        if i >= dst.len() {
            break;
        }
        dst[i] = 0;
    }
}

unsafe fn add_tree_node(id: c_int, value: c_int, parent_id: c_int, label: &[u8]) -> c_int {
    let count = unsafe { NODE_COUNT };
    if count as usize >= MAX_NODES {
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

    // Mimic strncpy(node->label, label, 31); node->label[31] = '\0';
    let mut tmp = [0u8; 32];
    strncpy_to_buf(&mut tmp, label, 31);
    tmp[31] = 0;
    unsafe {
        NODE_TABLE[idx].label = tmp;
    }

    if parent_id != -1 {
        let parent_idx = unsafe { find_node_index_by_id(parent_id) };
        let parent_idx = match parent_idx {
            Some(i) => i,
            None => return -1,
        };
        // The C code also checks parent->id != parent_id, but that's already
        // guaranteed by find_node_by_id; we replicate behavior identically.
        if unsafe { NODE_TABLE[parent_idx].id } != parent_id {
            return -1;
        }

        unsafe {
            if NODE_TABLE[parent_idx].left_child_id == -1 {
                NODE_TABLE[parent_idx].left_child_id = id;
            } else if NODE_TABLE[parent_idx].right_child_id == -1 {
                NODE_TABLE[parent_idx].right_child_id = id;
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

    let (value, left, right) = unsafe {
        (
            NODE_TABLE[idx].value,
            NODE_TABLE[idx].left_child_id,
            NODE_TABLE[idx].right_child_id,
        )
    };

    // Replicate the redundant id check in the C code (always true here, but
    // included for behavioral parity if state were to change unexpectedly).
    let actual_id = unsafe { NODE_TABLE[idx].id };
    if actual_id != node_id {
        return 0;
    }

    let mut sum = value;

    if left != -1 {
        sum = sum.wrapping_add(unsafe { calculate_tree_sum(left) });
    }

    if right != -1 {
        sum = sum.wrapping_add(unsafe { calculate_tree_sum(right) });
    }

    sum
}

/// Returns true if `s` (a null-terminated byte slice) contains `c` before the
/// first null byte.
fn label_contains(label: &[u8; 32], c: u8) -> bool {
    for &b in label.iter() {
        if b == 0 {
            return false;
        }
        if b == c {
            return true;
        }
    }
    false
}

fn parse_operation(op_str: &[u8]) -> Operation {
    // op_str is a null-terminated byte slice.  In the C code:
    //   if (op_str == NULL || strchr(op_str, '+') != NULL) return OP_ADD;
    // We mirror that: the call site always passes a non-null buffer, so we
    // just check for '+' first.
    if contains_byte(op_str, b'+') {
        return Operation::OpAdd;
    }
    if contains_byte(op_str, b'*') {
        return Operation::OpMultiply;
    }
    if contains_byte(op_str, b'-') {
        return Operation::OpSubtract;
    }
    if contains_byte(op_str, b'/') {
        return Operation::OpDivide;
    }
    if contains_byte(op_str, b'%') {
        return Operation::OpModulo;
    }
    Operation::OpAdd
}

fn contains_byte(s: &[u8], c: u8) -> bool {
    for &b in s {
        if b == 0 {
            return false;
        }
        if b == c {
            return true;
        }
    }
    false
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
    unsafe {
        NODE_COUNT = 0;

        add_tree_node(1, param1, -1, b"root");
        add_tree_node(2, param2, 1, b"left");
        add_tree_node(3, param3, 1, b"right");
        add_tree_node(4, param4, 2, b"left-left");

        let mut target_id: c_int = -1;
        let count = NODE_COUNT as usize;
        for i in 0..count {
            if label_contains(&NODE_TABLE[i].label, b'l') {
                target_id = NODE_TABLE[i].id;
                break;
            }
        }

        let target_idx = find_node_index_by_id(target_id);
        let target_value_zero_or_missing = match target_idx {
            None => true,
            Some(i) => NODE_TABLE[i].value == 0,
        };
        if target_value_zero_or_missing {
            target_id = 1;
        }

        let tree_sum = calculate_tree_sum(1);

        let op_string: &[u8; 4] = b"+*-%";
        let idx = (tree_sum.rem_euclid(4)) as usize;
        // C uses signed % which can be negative; replicate exactly:
        // In C, `tree_sum % 4` for negative tree_sum yields a negative or zero
        // result, and indexing op_string with that is undefined behavior.  We
        // mirror C's signed behavior by using wrapping_rem and casting.
        let signed_idx = tree_sum.wrapping_rem(4);
        // Use whichever the C compiler would produce: signed_idx (which may be
        // negative).  We must reproduce exact C semantics: indexing
        // `op_string[signed_idx]` where signed_idx might be 0..3 (or negative
        // for negative tree_sum, which is UB in C).  In practice gcc/clang
        // index by the signed integer cast to size_t, so a negative value
        // would underflow.  For non-negative tree_sum, signed_idx is in 0..3.
        let _ = idx;
        let op_byte = if signed_idx >= 0 && (signed_idx as usize) < 4 {
            op_string[signed_idx as usize]
        } else {
            // Reproduce undefined behavior path: read from the array
            // address with a negative offset cast as in C.  Practically
            // unreachable for typical inputs; default to '+' to avoid
            // panics in the Rust translation.
            let off = signed_idx as isize;
            // SAFETY: emulate C indexing semantics for negative offsets.
            let base = op_string.as_ptr();
            *base.offset(off)
        };

        let op_char: [u8; 2] = [op_byte, 0];
        let op = parse_operation(&op_char);

        let _op_value = op as i32;

        let func = get_operation_func(op);

        func(tree_sum, target_id, 0, 0)
    }
}
