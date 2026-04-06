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

fn parse_operation(op_char: u8) -> Operation {
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
    // C: tree_sum % 4 — C's % can be negative for negative dividend
    let rem = tree_sum.wrapping_rem(4);
    let index = if rem < 0 { (rem + 4) as usize } else { rem as usize };
    let op_char = op_string[index];
    let op = parse_operation(op_char);

    apply_op(op, tree_sum, target_id)
}
