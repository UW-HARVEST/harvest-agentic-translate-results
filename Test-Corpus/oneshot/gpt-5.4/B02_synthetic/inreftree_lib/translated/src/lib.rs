use std::os::raw::c_int;
use std::sync::{Mutex, OnceLock};

#[repr(i32)]
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
    id: c_int,
    value: c_int,
    parent_id: c_int,
    left_child_id: c_int,
    right_child_id: c_int,
    label: [u8; 32],
}

const MAX_NODES: usize = 50;

type OperationFunc = fn(c_int, c_int, c_int, c_int) -> c_int;

struct State {
    node_table: Vec<TreeNode>,
}

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(State { node_table: Vec::new() }))
}

fn add_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a + b
}

fn multiply_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a * b
}

fn subtract_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a - b
}

fn divide_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 {
        0
    } else {
        a / b
    }
}

fn modulo_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 {
        0
    } else {
        a % b
    }
}

fn find_node_index_by_id(nodes: &[TreeNode], id: c_int) -> Option<usize> {
    nodes.iter().position(|node| node.id == id)
}

fn make_label(label: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    let bytes = label.as_bytes();
    let len = bytes.len().min(31);
    out[..len].copy_from_slice(&bytes[..len]);
    out
}

fn label_contains(node: &TreeNode, ch: u8) -> bool {
    node.label.iter().copied().take_while(|b| *b != 0).any(|b| b == ch)
}

fn add_tree_node(state: &mut State, id: c_int, value: c_int, parent_id: c_int, label: &str) -> c_int {
    if state.node_table.len() >= MAX_NODES {
        return -1;
    }

    let node = TreeNode {
        id,
        value,
        parent_id,
        left_child_id: -1,
        right_child_id: -1,
        label: make_label(label),
    };

    if parent_id != -1 {
        let Some(parent_index) = find_node_index_by_id(&state.node_table, parent_id) else {
            return -1;
        };

        let parent = &mut state.node_table[parent_index];
        if parent.left_child_id == -1 {
            parent.left_child_id = id;
        } else if parent.right_child_id == -1 {
            parent.right_child_id = id;
        }
    }

    state.node_table.push(node);
    (state.node_table.len() - 1) as c_int
}

fn calculate_tree_sum(nodes: &[TreeNode], node_id: c_int) -> c_int {
    let Some(index) = find_node_index_by_id(nodes, node_id) else {
        return 0;
    };

    let node = &nodes[index];
    let mut sum = node.value;

    if node.left_child_id != -1 {
        sum += calculate_tree_sum(nodes, node.left_child_id);
    }

    if node.right_child_id != -1 {
        sum += calculate_tree_sum(nodes, node.right_child_id);
    }

    sum
}

fn parse_operation(op_str: Option<&str>) -> Operation {
    let Some(op_str) = op_str else {
        return Operation::Add;
    };

    if op_str.contains('+') {
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
    let mut guard = state().lock().unwrap();
    let state = &mut *guard;
    state.node_table.clear();

    let _ = add_tree_node(state, 1, param1, -1, "root");
    let _ = add_tree_node(state, 2, param2, 1, "left");
    let _ = add_tree_node(state, 3, param3, 1, "right");
    let _ = add_tree_node(state, 4, param4, 2, "left-left");

    let mut target_id: c_int = -1;
    for node in &state.node_table {
        if label_contains(node, b'l') {
            target_id = node.id;
            break;
        }
    }

    match find_node_index_by_id(&state.node_table, target_id) {
        Some(index) if state.node_table[index].value != 0 => {}
        _ => target_id = 1,
    }

    let tree_sum = calculate_tree_sum(&state.node_table, 1);

    let op_chars = [b'+', b'*', b'-', b'%'];
    let idx = tree_sum.rem_euclid(4) as usize;
    let op_char = op_chars[idx] as char;
    let op = parse_operation(Some(&op_char.to_string()));

    let _op_value = op as c_int;

    let func = get_operation_func(op);
    func(tree_sum, target_id, 0, 0)
}
