use std::os::raw::c_int;

const MAX_NODES: usize = 50;

#[derive(Clone)]
struct TreeNode {
    id: c_int,
    value: c_int,
    parent_id: c_int,
    left_child_id: c_int,
    right_child_id: c_int,
    label: [u8; 32],
}

impl Default for TreeNode {
    fn default() -> Self {
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

unsafe fn find_node_by_id(id: c_int) -> Option<usize> {
    for i in 0..NODE_COUNT as usize {
        if NODE_TABLE[i].id == id {
            return Some(i);
        }
    }
    None
}

unsafe fn add_tree_node(id: c_int, value: c_int, parent_id: c_int, label: &[u8]) -> c_int {
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
    let copy_len = label.len().min(31);
    node.label[..copy_len].copy_from_slice(&label[..copy_len]);
    node.label[copy_len..].fill(0);

    if parent_id != -1 {
        let parent_idx = find_node_by_id(parent_id);
        match parent_idx {
            Some(pi) => {
                if NODE_TABLE[pi].id != parent_id {
                    return -1;
                }
                if NODE_TABLE[pi].left_child_id == -1 {
                    NODE_TABLE[pi].left_child_id = id;
                } else if NODE_TABLE[pi].right_child_id == -1 {
                    NODE_TABLE[pi].right_child_id = id;
                }
            }
            None => return -1,
        }
    }

    NODE_COUNT += 1;
    NODE_COUNT - 1
}

unsafe fn calculate_tree_sum(node_id: c_int) -> c_int {
    let idx = match find_node_by_id(node_id) {
        Some(i) => i,
        None => return 0,
    };

    if NODE_TABLE[idx].id != node_id {
        return 0;
    }

    let mut sum = NODE_TABLE[idx].value;
    let left = NODE_TABLE[idx].left_child_id;
    let right = NODE_TABLE[idx].right_child_id;

    if left != -1 {
        sum += calculate_tree_sum(left);
    }
    if right != -1 {
        sum += calculate_tree_sum(right);
    }

    sum
}

fn label_contains(label: &[u8; 32], ch: u8) -> bool {
    for &b in label.iter() {
        if b == 0 { break; }
        if b == ch { return true; }
    }
    false
}

#[repr(C)]
#[derive(Clone, Copy)]
enum Operation {
    Add = 1,
    Multiply = 2,
    Subtract = 3,
    Divide = 4,
    Modulo = 5,
}

fn parse_operation(op_char: u8) -> Operation {
    // C checks: NULL -> ADD, strchr for +, *, -, /, %
    // We always have a valid single char from op_string
    if op_char == b'+' { return Operation::Add; }
    if op_char == b'*' { return Operation::Multiply; }
    if op_char == b'-' { return Operation::Subtract; }
    if op_char == b'/' { return Operation::Divide; }
    if op_char == b'%' { return Operation::Modulo; }
    Operation::Add
}

type OperationFunc = fn(c_int, c_int, c_int, c_int) -> c_int;

fn add_op(a: c_int, b: c_int, _: c_int, _: c_int) -> c_int { a.wrapping_add(b) }
fn multiply_op(a: c_int, b: c_int, _: c_int, _: c_int) -> c_int { a.wrapping_mul(b) }
fn subtract_op(a: c_int, b: c_int, _: c_int, _: c_int) -> c_int { a.wrapping_sub(b) }
fn divide_op(a: c_int, b: c_int, _: c_int, _: c_int) -> c_int {
    if b == 0 { 0 } else { a.wrapping_div(b) }
}
fn modulo_op(a: c_int, b: c_int, _: c_int, _: c_int) -> c_int {
    if b == 0 { 0 } else { a.wrapping_rem(b) }
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
        for i in 0..NODE_COUNT as usize {
            if label_contains(&NODE_TABLE[i].label, b'l') {
                target_id = NODE_TABLE[i].id;
                break;
            }
        }

        if let Some(idx) = find_node_by_id(target_id) {
            if NODE_TABLE[idx].value == 0 {
                target_id = 1;
            }
        } else {
            target_id = 1;
        }

        let tree_sum = calculate_tree_sum(1);

        let op_string: &[u8] = b"+*-%";
        // C: tree_sum % 4 — in C, % can be negative for negative dividend
        let rem = tree_sum.wrapping_rem(4);
        let index = if rem < 0 { (rem + 4) as usize } else { rem as usize };
        let op_char = op_string[index];
        let op = parse_operation(op_char);

        let _op_value = op as c_int;

        let func = get_operation_func(op);

        func(tree_sum, target_id, 0, 0)
    }
}
