// Rust translation of c_src/src/lib.c
// The C source defines only a library function `inreftree(a, b, c, d)`.
// To produce an executable, we read four integers from stdin (scanf-style,
// whitespace-separated, possibly across newlines) and print the result.

use std::io::{self, Read};

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
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

impl TreeNode {
    fn new() -> Self {
        TreeNode {
            id: 0,
            value: 0,
            parent_id: 0,
            left_child_id: -1,
            right_child_id: -1,
            label: [0u8; 32],
        }
    }
}

const MAX_NODES: usize = 50;

struct State {
    node_table: Vec<TreeNode>,
    node_count: usize,
}

impl State {
    fn new() -> Self {
        let mut nodes = Vec::with_capacity(MAX_NODES);
        for _ in 0..MAX_NODES {
            nodes.push(TreeNode::new());
        }
        State {
            node_table: nodes,
            node_count: 0,
        }
    }

    fn find_index_by_id(&self, id: i32) -> Option<usize> {
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

        // strncpy(node->label, label, 31); node->label[31] = '\0';
        let mut buf = [0u8; 32];
        let bytes = label.as_bytes();
        let n = bytes.len().min(31);
        buf[..n].copy_from_slice(&bytes[..n]);
        // buf[31] = 0 already.

        let idx = self.node_count;
        {
            let node = &mut self.node_table[idx];
            node.id = id;
            node.value = value;
            node.parent_id = parent_id;
            node.left_child_id = -1;
            node.right_child_id = -1;
            node.label = buf;
        }

        if parent_id != -1 {
            let parent_idx = self.find_index_by_id(parent_id);
            match parent_idx {
                None => return -1,
                Some(pi) => {
                    let parent = &mut self.node_table[pi];
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
        let idx = match self.find_index_by_id(node_id) {
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

fn parse_operation(op_str: Option<&[u8]>) -> Operation {
    match op_str {
        None => Operation::Add,
        Some(s) => {
            // C: strchr stops at NUL terminator. Our buffer is a single byte
            // followed by a 0 terminator (replicating the C usage), so we
            // consider only the first byte.
            let nul = s.iter().position(|&b| b == 0).unwrap_or(s.len());
            let s = &s[..nul];
            if s.contains(&b'+') {
                return Operation::Add;
            }
            if s.contains(&b'*') {
                return Operation::Multiply;
            }
            if s.contains(&b'-') {
                return Operation::Subtract;
            }
            if s.contains(&b'/') {
                return Operation::Divide;
            }
            if s.contains(&b'%') {
                return Operation::Modulo;
            }
            Operation::Add
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

fn label_contains(node: &TreeNode, ch: u8) -> bool {
    // Equivalent to strchr(node->label, ch): scan up to first NUL.
    for &b in node.label.iter() {
        if b == 0 {
            return false;
        }
        if b == ch {
            return true;
        }
    }
    false
}

fn inreftree(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let mut state = State::new();

    state.add_tree_node(1, param1, -1, "root");
    state.add_tree_node(2, param2, 1, "left");
    state.add_tree_node(3, param3, 1, "right");
    state.add_tree_node(4, param4, 2, "left-left");

    let mut target_id: i32 = -1;
    for i in 0..state.node_count {
        if label_contains(&state.node_table[i], b'l') {
            target_id = state.node_table[i].id;
            break;
        }
    }

    let target_idx = state.find_index_by_id(target_id);
    let target_value_zero_or_missing = match target_idx {
        None => true,
        Some(i) => {
            let t = &state.node_table[i];
            t.id != target_id || t.value == 0
        }
    };
    if target_value_zero_or_missing {
        target_id = 1;
    }

    let tree_sum = state.calculate_tree_sum(1);

    let op_string = b"+*-%";
    // C semantics: tree_sum % 4 may be negative for negative tree_sum, in
    // which case op_string[idx] would be UB. We compute the index using
    // C's truncated remainder; for the (typical) non-negative case this
    // matches exactly. For negative results we mirror C-truncated semantics
    // by wrapping into a non-negative range to avoid a panic.
    let raw_idx = tree_sum % 4;
    let idx = ((raw_idx % 4) + 4) % 4;
    let op_byte = op_string[idx as usize];
    let op_char_buf: [u8; 2] = [op_byte, 0];
    let op = parse_operation(Some(&op_char_buf));

    let _op_value = op as i32;
    let func = get_operation_func(op);

    func(tree_sum, target_id, 0, 0)
}

fn main() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return;
    }

    let mut iter = input.split_ascii_whitespace();
    let parse_next = |it: &mut std::str::SplitAsciiWhitespace| -> i32 {
        match it.next() {
            Some(tok) => tok.parse::<i32>().unwrap_or(0),
            None => 0,
        }
    };

    let a = parse_next(&mut iter);
    let b = parse_next(&mut iter);
    let c = parse_next(&mut iter);
    let d = parse_next(&mut iter);

    let result = inreftree(a, b, c, d);
    println!("{}", result);
}
