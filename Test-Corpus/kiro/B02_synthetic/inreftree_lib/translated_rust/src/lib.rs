use std::os::raw::{c_char, c_int};

const MAX_NODES: usize = 50;

#[repr(C)]
#[derive(Clone)]
pub struct TreeNode {
    pub id: c_int,
    pub value: c_int,
    pub parent_id: c_int,
    pub left_child_id: c_int,
    pub right_child_id: c_int,
    pub label: [u8; 32],
}

static mut NODE_TABLE: [TreeNode; MAX_NODES] = {
    const INIT: TreeNode = TreeNode {
        id: 0,
        value: 0,
        parent_id: 0,
        left_child_id: 0,
        right_child_id: 0,
        label: [0; 32],
    };
    [INIT; MAX_NODES]
};
static mut NODE_COUNT: c_int = 0;

type OperationFunc = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_op(a: c_int, b: c_int, _: c_int, _: c_int) -> c_int {
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_op(a: c_int, b: c_int, _: c_int, _: c_int) -> c_int {
    a.wrapping_mul(b)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn subtract_op(a: c_int, b: c_int, _: c_int, _: c_int) -> c_int {
    a.wrapping_sub(b)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn divide_op(a: c_int, b: c_int, _: c_int, _: c_int) -> c_int {
    if b == 0 { 0 } else { a.wrapping_div(b) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn modulo_op(a: c_int, b: c_int, _: c_int, _: c_int) -> c_int {
    if b == 0 { 0 } else { a.wrapping_rem(b) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_node_by_id(id: c_int) -> *mut TreeNode {
    for i in 0..NODE_COUNT as usize {
        if NODE_TABLE[i].id == id {
            return &mut NODE_TABLE[i] as *mut TreeNode;
        }
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

    // strncpy(node->label, label, 31); node->label[31] = '\0';
    node.label = [0; 32];
    if !label.is_null() {
        let mut i = 0;
        while i < 31 {
            let c = *label.add(i);
            if c == 0 {
                break;
            }
            node.label[i] = c as u8;
            i += 1;
        }
    }

    if parent_id != -1 {
        let parent_ptr = find_node_by_id(parent_id);
        if parent_ptr.is_null() || (*parent_ptr).id != parent_id {
            return -1;
        }
        if (*parent_ptr).left_child_id == -1 {
            (*parent_ptr).left_child_id = id;
        } else if (*parent_ptr).right_child_id == -1 {
            (*parent_ptr).right_child_id = id;
        }
    }

    NODE_COUNT += 1;
    NODE_COUNT - 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn calculate_tree_sum(node_id: c_int) -> c_int {
    let node_ptr = find_node_by_id(node_id);
    if node_ptr.is_null() || (*node_ptr).id != node_id {
        return 0;
    }

    let mut sum = (*node_ptr).value;
    let left = (*node_ptr).left_child_id;
    let right = (*node_ptr).right_child_id;

    if left != -1 {
        sum += calculate_tree_sum(left);
    }
    if right != -1 {
        sum += calculate_tree_sum(right);
    }

    sum
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_operation(op_str: *const c_char) -> c_int {
    // C: if (op_str == NULL || strchr(op_str, '+') != NULL) return OP_ADD;
    if op_str.is_null() {
        return 1; // OP_ADD
    }
    if strchr(op_str, b'+') {
        return 1;
    }
    if strchr(op_str, b'*') {
        return 2;
    }
    if strchr(op_str, b'-') {
        return 3;
    }
    if strchr(op_str, b'/') {
        return 4;
    }
    if strchr(op_str, b'%') {
        return 5;
    }
    1 // OP_ADD
}

unsafe fn strchr(s: *const c_char, c: u8) -> bool {
    let mut p = s;
    loop {
        let ch = *p as u8;
        if ch == c {
            return true;
        }
        if ch == 0 {
            return false;
        }
        p = p.add(1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_operation_func(op: c_int) -> OperationFunc {
    match op {
        1 => add_op,
        2 => multiply_op,
        3 => subtract_op,
        4 => divide_op,
        5 => modulo_op,
        _ => add_op,
    }
}

fn label_contains(label: &[u8; 32], ch: u8) -> bool {
    for &b in label.iter() {
        if b == 0 {
            break;
        }
        if b == ch {
            return true;
        }
    }
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn inreftree(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    NODE_COUNT = 0;

    add_tree_node(1, param1, -1, b"root\0".as_ptr() as *const c_char);
    add_tree_node(2, param2, 1, b"left\0".as_ptr() as *const c_char);
    add_tree_node(3, param3, 1, b"right\0".as_ptr() as *const c_char);
    add_tree_node(4, param4, 2, b"left-left\0".as_ptr() as *const c_char);

    let mut target_id: c_int = -1;
    for i in 0..NODE_COUNT as usize {
        if label_contains(&NODE_TABLE[i].label, b'l') {
            target_id = NODE_TABLE[i].id;
            break;
        }
    }

    let target = find_node_by_id(target_id);
    if target.is_null() || (*target).value == 0 {
        target_id = 1;
    }

    let tree_sum = calculate_tree_sum(1);

    let op_string: &[u8] = b"+*-%";
    // C: tree_sum % 4 — negative remainder causes UB (negative array index).
    // In the compiled C .so, the out-of-bounds byte never matches +*-/%,
    // so parse_operation returns OP_ADD. Replicate by using a null char.
    let rem = tree_sum.wrapping_rem(4);
    let op_byte = if rem >= 0 && (rem as usize) < 4 {
        op_string[rem as usize]
    } else {
        0u8 // will not match any op char, parse_operation returns OP_ADD
    };
    let op_char_buf: [u8; 2] = [op_byte, 0];
    let op = parse_operation(op_char_buf.as_ptr() as *const c_char);

    let _op_value = op;

    let func = get_operation_func(op);

    func(tree_sum, target_id, 0, 0)
}
