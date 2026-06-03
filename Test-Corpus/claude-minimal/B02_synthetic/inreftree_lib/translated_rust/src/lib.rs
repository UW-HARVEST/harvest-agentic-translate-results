// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum Operation {
    Add = 1,
    Multiply = 2,
    Subtract = 3,
    Divide = 4,
    Modulo = 5,
}

#[derive(Clone, Copy)]
struct TreeNode {
    id: i32,
    value: i32,
    parent_id: i32,
    left_child_id: i32,
    right_child_id: i32,
    label: [u8; 32],
}

impl TreeNode {
    fn empty() -> Self {
        TreeNode {
            id: 0,
            value: 0,
            parent_id: 0,
            left_child_id: -1,
            right_child_id: -1,
            label: [0u8; 32],
        }
    }

    fn label_contains(&self, c: u8) -> bool {
        for &b in self.label.iter() {
            if b == 0 {
                return false;
            }
            if b == c {
                return true;
            }
        }
        false
    }
}

const MAX_NODES: usize = 50;

struct TreeState {
    node_table: [TreeNode; MAX_NODES],
    node_count: usize,
}

impl TreeState {
    fn new() -> Self {
        TreeState {
            node_table: [TreeNode::empty(); MAX_NODES],
            node_count: 0,
        }
    }

    fn find_node_index_by_id(&self, id: i32) -> Option<usize> {
        for i in 0..self.node_count {
            if self.node_table[i].id == id {
                return Some(i);
            }
        }
        None
    }

    fn add_tree_node(&mut self, id: i32, value: i32, parent_id: i32, label: &str) -> i32 {
        if self.node_count >= MAX_NODES {
            return -1;
        }

        let idx = self.node_count;
        let node = &mut self.node_table[idx];
        node.id = id;
        node.value = value;
        node.parent_id = parent_id;
        node.left_child_id = -1;
        node.right_child_id = -1;
        node.label = [0u8; 32];

        // strncpy(node->label, label, 31); node->label[31] = '\0';
        let bytes = label.as_bytes();
        let copy_len = bytes.len().min(31);
        node.label[..copy_len].copy_from_slice(&bytes[..copy_len]);
        // index 31 is already 0

        if parent_id != -1 {
            match self.find_node_index_by_id(parent_id) {
                None => return -1,
                Some(parent_idx) => {
                    let parent = &mut self.node_table[parent_idx];
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

        self.node_count += 1;
        (self.node_count - 1) as i32
    }

    fn calculate_tree_sum(&self, node_id: i32) -> i32 {
        let idx = match self.find_node_index_by_id(node_id) {
            Some(i) => i,
            None => return 0,
        };
        let node = &self.node_table[idx];
        if node.id != node_id {
            return 0;
        }
        let mut sum = node.value;
        if node.left_child_id != -1 {
            sum = sum.wrapping_add(self.calculate_tree_sum(node.left_child_id));
        }
        if node.right_child_id != -1 {
            sum = sum.wrapping_add(self.calculate_tree_sum(node.right_child_id));
        }
        sum
    }
}

fn add_op(a: i32, b: i32, _u1: i32, _u2: i32) -> i32 {
    a.wrapping_add(b)
}

fn multiply_op(a: i32, b: i32, _u1: i32, _u2: i32) -> i32 {
    a.wrapping_mul(b)
}

fn subtract_op(a: i32, b: i32, _u1: i32, _u2: i32) -> i32 {
    a.wrapping_sub(b)
}

fn divide_op(a: i32, b: i32, _u1: i32, _u2: i32) -> i32 {
    if b == 0 {
        return 0;
    }
    a.wrapping_div(b)
}

fn modulo_op(a: i32, b: i32, _u1: i32, _u2: i32) -> i32 {
    if b == 0 {
        return 0;
    }
    a.wrapping_rem(b)
}

type OperationFunc = fn(i32, i32, i32, i32) -> i32;

fn parse_operation(op_str: Option<&str>) -> Operation {
    let s = match op_str {
        None => return Operation::Add,
        Some(s) => s,
    };
    if s.contains('+') {
        return Operation::Add;
    }
    if s.contains('*') {
        return Operation::Multiply;
    }
    if s.contains('-') {
        return Operation::Subtract;
    }
    if s.contains('/') {
        return Operation::Divide;
    }
    if s.contains('%') {
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

pub fn inreftree(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let mut state = TreeState::new();

    state.add_tree_node(1, param1, -1, "root");
    state.add_tree_node(2, param2, 1, "left");
    state.add_tree_node(3, param3, 1, "right");
    state.add_tree_node(4, param4, 2, "left-left");

    let mut target_id: i32 = -1;
    for i in 0..state.node_count {
        if state.node_table[i].label_contains(b'l') {
            target_id = state.node_table[i].id;
            break;
        }
    }

    let target_idx = state.find_node_index_by_id(target_id);
    let target_value_zero_or_missing = match target_idx {
        None => true,
        Some(i) => state.node_table[i].value == 0,
    };
    if target_value_zero_or_missing {
        target_id = 1;
    }

    let tree_sum = state.calculate_tree_sum(1);

    let op_string: &[u8] = b"+*-%";
    // tree_sum % 4 in C with potentially negative input. Match C semantics with wrapping_rem.
    let idx_signed = tree_sum.wrapping_rem(4);
    // C indexing with negative index would be UB; the reachable values here for non-negative
    // tree_sum stay in [0,3]. For negative, C would index out-of-bounds. Clamp to 0 to avoid panic.
    let idx = if idx_signed < 0 || idx_signed >= 4 {
        0usize
    } else {
        idx_signed as usize
    };
    let op_char_byte = op_string[idx];
    let op_char_str = std::str::from_utf8(std::slice::from_ref(&op_char_byte)).unwrap_or("+");
    let op = parse_operation(Some(op_char_str));

    let _op_value = op as i32;

    let func = get_operation_func(op);

    func(tree_sum, target_id, 0, 0)
}
