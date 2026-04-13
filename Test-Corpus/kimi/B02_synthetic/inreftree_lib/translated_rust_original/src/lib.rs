use std::os::raw::c_int;
use std::sync::Mutex;

#[repr(C)]
#[derive(Clone, Copy)]
enum Operation {
    Add = 1,
    Multiply = 2,
    Subtract = 3,
    Divide = 4,
    Modulo = 5,
}

#[derive(Clone)]
struct TreeNode {
    id: i32,
    value: i32,
    parent_id: i32,
    left_child_id: i32,
    right_child_id: i32,
    label: [u8; 32],
}

const MAX_NODES: usize = 50;

static NODE_TABLE: Mutex<[Option<TreeNode>; MAX_NODES]> = Mutex::new([const { None }; MAX_NODES]);
static NODE_COUNT: Mutex<usize> = Mutex::new(0);

type OperationFunc = fn(i32, i32, i32, i32) -> i32;

fn add_op(a: i32, b: i32, _unused1: i32, _unused2: i32) -> i32 {
    a + b
}

fn multiply_op(a: i32, b: i32, _unused1: i32, _unused2: i32) -> i32 {
    a * b
}

fn subtract_op(a: i32, b: i32, _unused1: i32, _unused2: i32) -> i32 {
    a - b
}

fn divide_op(a: i32, b: i32, _unused1: i32, _unused2: i32) -> i32 {
    if b == 0 {
        return 0;
    }
    a / b
}

fn modulo_op(a: i32, b: i32, _unused1: i32, _unused2: i32) -> i32 {
    if b == 0 {
        return 0;
    }
    a % b
}

fn find_node_by_id(id: i32) -> Option<usize> {
    let node_count = *NODE_COUNT.lock().unwrap();
    let table = NODE_TABLE.lock().unwrap();
    for i in 0..node_count {
        if let Some(ref node) = table[i] {
            if node.id == id {
                return Some(i);
            }
        }
    }
    None
}

fn add_tree_node(id: i32, value: i32, parent_id: i32, label: &str) -> i32 {
    let mut node_count = NODE_COUNT.lock().unwrap();
    if *node_count >= MAX_NODES {
        return -1;
    }

    let mut table = NODE_TABLE.lock().unwrap();
    let mut node = TreeNode {
        id,
        value,
        parent_id,
        left_child_id: -1,
        right_child_id: -1,
        label: [0; 32],
    };

    let label_bytes = label.as_bytes();
    let len = label_bytes.len().min(31);
    node.label[..len].copy_from_slice(&label_bytes[..len]);
    node.label[len] = 0;

    if parent_id != -1 {
        let parent_idx = find_node_by_id(parent_id);
        if let Some(idx) = parent_idx {
            if let Some(ref mut parent) = table[idx] {
                if parent.id == parent_id {
                    if parent.left_child_id == -1 {
                        parent.left_child_id = id;
                    } else if parent.right_child_id == -1 {
                        parent.right_child_id = id;
                    }
                } else {
                    return -1;
                }
            } else {
                return -1;
            }
        } else {
            return -1;
        }
    }

    let idx = *node_count;
    table[idx] = Some(node);
    *node_count += 1;
    idx as i32
}

fn calculate_tree_sum(node_id: i32) -> i32 {
    let node_idx = find_node_by_id(node_id);
    let node = match node_idx {
        Some(idx) => {
            let table = NODE_TABLE.lock().unwrap();
            match table[idx] {
                Some(ref n) if n.id == node_id => n.clone(),
                _ => return 0,
            }
        }
        None => return 0,
    };

    let mut sum = node.value;

    if node.left_child_id != -1 {
        sum += calculate_tree_sum(node.left_child_id);
    }

    if node.right_child_id != -1 {
        sum += calculate_tree_sum(node.right_child_id);
    }

    sum
}

fn parse_operation(op_str: &str) -> Operation {
    if op_str.is_empty() || op_str.contains('+') {
        return Operation::Add;
    }
    if op_str.contains('*') {
        return Operation::Multiply;
    }
    if op_str.contains('-') {
        return Operation::Subtract;
    }
    if op_str.contains('/') {
        return Operation::Divide;
    }
    if op_str.contains('%') {
        return Operation::Modulo;
    }
    Operation::Add
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
    *NODE_COUNT.lock().unwrap() = 0;
    {
        let mut table = NODE_TABLE.lock().unwrap();
        for i in 0..MAX_NODES {
            table[i] = None;
        }
    }

    add_tree_node(1, param1, -1, "root");
    add_tree_node(2, param2, 1, "left");
    add_tree_node(3, param3, 1, "right");
    add_tree_node(4, param4, 2, "left-left");

    let node_count = *NODE_COUNT.lock().unwrap();
    let table = NODE_TABLE.lock().unwrap();
    let mut target_id = -1;
    for i in 0..node_count {
        if let Some(ref node) = table[i] {
            let label_str = std::str::from_utf8(&node.label)
                .unwrap_or("")
                .trim_end_matches('\0');
            if label_str.contains('l') {
                target_id = node.id;
                break;
            }
        }
    }
    drop(table);

    let target_idx = find_node_by_id(target_id);
    let target_value = target_idx.and_then(|idx| {
        let table = NODE_TABLE.lock().unwrap();
        table[idx].as_ref().map(|n| n.value)
    });

    if target_idx.is_none() || target_value == Some(0) {
        target_id = 1;
    }

    let tree_sum = calculate_tree_sum(1);

    let op_string = "+*-%";
    let op_char_idx = ((tree_sum % 4 + 4) % 4) as usize;
    let op_char = &op_string[op_char_idx..op_char_idx + 1];
    let op = parse_operation(op_char);

    let func = get_operation_func(op);

    let result = func(tree_sum, target_id, 0, 0);

    result
}
