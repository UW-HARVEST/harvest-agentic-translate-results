use std::os::raw::{c_char, c_int};
use std::sync::Mutex;

const MAX_NODES: usize = 50;

#[repr(C)]
#[derive(Clone)]
pub struct TreeNode {
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
            label: [0u8; 32],
        }
    }
}

struct State {
    node_table: [TreeNode; MAX_NODES],
    node_count: c_int,
}

impl State {
    fn new() -> Self {
        State {
            node_table: std::array::from_fn(|_| TreeNode::default()),
            node_count: 0,
        }
    }

    fn find_node_by_id(&self, id: c_int) -> Option<usize> {
        for i in 0..self.node_count as usize {
            if self.node_table[i].id == id {
                return Some(i);
            }
        }
        None
    }

    fn add_tree_node(&mut self, id: c_int, value: c_int, parent_id: c_int, label: &[u8]) -> c_int {
        if self.node_count >= MAX_NODES as c_int {
            return -1;
        }

        let idx = self.node_count as usize;
        let node = &mut self.node_table[idx];
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
            if let Some(pi) = self.find_node_by_id(parent_id) {
                if self.node_table[pi].id != parent_id {
                    return -1;
                }
                if self.node_table[pi].left_child_id == -1 {
                    self.node_table[pi].left_child_id = id;
                } else if self.node_table[pi].right_child_id == -1 {
                    self.node_table[pi].right_child_id = id;
                }
            } else {
                return -1;
            }
        }

        self.node_count += 1;
        self.node_count - 1
    }

    fn calculate_tree_sum(&self, node_id: c_int) -> c_int {
        let idx = match self.find_node_by_id(node_id) {
            Some(i) => i,
            None => return 0,
        };

        if self.node_table[idx].id != node_id {
            return 0;
        }

        let mut sum = self.node_table[idx].value;

        if self.node_table[idx].left_child_id != -1 {
            sum += self.calculate_tree_sum(self.node_table[idx].left_child_id);
        }

        if self.node_table[idx].right_child_id != -1 {
            sum += self.calculate_tree_sum(self.node_table[idx].right_child_id);
        }

        sum
    }
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

fn parse_operation_internal(op_char: u8) -> Operation {
    // C checks: NULL->ADD, strchr for +, *, -, /, %
    // We receive a single char; match C's strchr logic on a 1-char string
    if op_char == b'+' { return Operation::Add; }
    if op_char == b'*' { return Operation::Multiply; }
    if op_char == b'-' { return Operation::Subtract; }
    if op_char == b'/' { return Operation::Divide; }
    if op_char == b'%' { return Operation::Modulo; }
    Operation::Add
}

fn apply_op(op: Operation, a: c_int, b: c_int) -> c_int {
    match op {
        Operation::Add => a.wrapping_add(b),
        Operation::Multiply => a.wrapping_mul(b),
        Operation::Subtract => a.wrapping_sub(b),
        Operation::Divide => {
            if b == 0 { 0 } else { a.wrapping_div(b) }
        }
        Operation::Modulo => {
            if b == 0 { 0 } else { a.wrapping_rem(b) }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn inreftree(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut state = State::new();

    state.add_tree_node(1, param1, -1, b"root");
    state.add_tree_node(2, param2, 1, b"left");
    state.add_tree_node(3, param3, 1, b"right");
    state.add_tree_node(4, param4, 2, b"left-left");

    let mut target_id: c_int = -1;
    for i in 0..state.node_count as usize {
        if label_contains(&state.node_table[i].label, b'l') {
            target_id = state.node_table[i].id;
            break;
        }
    }

    // C: if (target == NULL || target->value == 0) target_id = 1;
    let reset = match state.find_node_by_id(target_id) {
        None => true,
        Some(idx) => state.node_table[idx].value == 0,
    };
    if reset {
        target_id = 1;
    }

    let tree_sum = state.calculate_tree_sum(1);

    let op_string: &[u8] = b"+*-%";
    // C: tree_sum % 4 — C's % can be negative for negative dividend.
    // In C, a negative index is UB and reads garbage, which doesn't match
    // any operator char, so parse_operation defaults to OP_ADD.
    let rem = tree_sum.wrapping_rem(4);
    let op = if rem < 0 {
        Operation::Add
    } else {
        parse_operation_internal(op_string[rem as usize])
    };

    apply_op(op, tree_sum, target_id)
}

// --- Global state matching C's global node_table/node_count ---

static GLOBAL_STATE: Mutex<State> = Mutex::new(State {
    node_table: {
        const DEFAULT_NODE: TreeNode = TreeNode {
            id: 0, value: 0, parent_id: 0,
            left_child_id: 0, right_child_id: 0, label: [0u8; 32],
        };
        [DEFAULT_NODE; MAX_NODES]
    },
    node_count: 0,
});

// --- Exported C-compatible arithmetic ops ---

#[unsafe(no_mangle)]
pub extern "C" fn add_op(a: c_int, b: c_int, _: c_int, _: c_int) -> c_int {
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_op(a: c_int, b: c_int, _: c_int, _: c_int) -> c_int {
    a.wrapping_mul(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn subtract_op(a: c_int, b: c_int, _: c_int, _: c_int) -> c_int {
    a.wrapping_sub(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn divide_op(a: c_int, b: c_int, _: c_int, _: c_int) -> c_int {
    if b == 0 { 0 } else { a.wrapping_div(b) }
}

#[unsafe(no_mangle)]
pub extern "C" fn modulo_op(a: c_int, b: c_int, _: c_int, _: c_int) -> c_int {
    if b == 0 { 0 } else { a.wrapping_rem(b) }
}

// --- Exported tree functions using global state ---

/// Returns pointer to node in global table, or null.
#[unsafe(no_mangle)]
pub extern "C" fn find_node_by_id(id: c_int) -> *mut TreeNode {
    let mut state = GLOBAL_STATE.lock().unwrap();
    for i in 0..state.node_count as usize {
        if state.node_table[i].id == id {
            return &mut state.node_table[i] as *mut TreeNode;
        }
    }
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn add_tree_node(
    id: c_int, value: c_int, parent_id: c_int, label: *const c_char,
) -> c_int {
    let mut state = GLOBAL_STATE.lock().unwrap();
    let label_bytes: &[u8] = if label.is_null() {
        b""
    } else {
        unsafe { std::ffi::CStr::from_ptr(label).to_bytes() }
    };
    state.add_tree_node(id, value, parent_id, label_bytes)
}

#[unsafe(no_mangle)]
pub extern "C" fn calculate_tree_sum(node_id: c_int) -> c_int {
    let state = GLOBAL_STATE.lock().unwrap();
    state.calculate_tree_sum(node_id)
}

// --- Exported parse_operation matching C signature ---

#[unsafe(no_mangle)]
pub extern "C" fn parse_operation(op_str: *const c_char) -> c_int {
    if op_str.is_null() {
        return 1; // OP_ADD
    }
    let cstr = unsafe { std::ffi::CStr::from_ptr(op_str) };
    let bytes = cstr.to_bytes();
    if bytes.iter().any(|&b| b == b'+') { return 1; }
    if bytes.iter().any(|&b| b == b'*') { return 2; }
    if bytes.iter().any(|&b| b == b'-') { return 3; }
    if bytes.iter().any(|&b| b == b'/') { return 4; }
    if bytes.iter().any(|&b| b == b'%') { return 5; }
    1 // OP_ADD
}

type OperationFunc = extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[unsafe(no_mangle)]
pub extern "C" fn get_operation_func(op: c_int) -> OperationFunc {
    match op {
        1 => add_op,
        2 => multiply_op,
        3 => subtract_op,
        4 => divide_op,
        5 => modulo_op,
        _ => add_op,
    }
}
