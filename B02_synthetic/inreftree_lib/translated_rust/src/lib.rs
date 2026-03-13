use std::os::raw::c_int;

const MAX_NODES: usize = 50;

#[repr(C)]
#[derive(Clone, Copy)]
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
    node_count: usize,
}

impl State {
    fn new() -> Self {
        State {
            node_table: [TreeNode::default(); MAX_NODES],
            node_count: 0,
        }
    }

    fn find_node_by_id(&self, id: c_int) -> Option<usize> {
        for i in 0..self.node_count {
            if self.node_table[i].id == id {
                return Some(i);
            }
        }
        None
    }

    fn add_tree_node(&mut self, id: c_int, value: c_int, parent_id: c_int, label: &[u8]) -> c_int {
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

        // strncpy(node->label, label, 31); node->label[31] = '\0';
        let copy_len = label.len().min(31);
        node.label[..copy_len].copy_from_slice(&label[..copy_len]);
        node.label[copy_len..].fill(0);

        if parent_id != -1 {
            let parent_idx = self.find_node_by_id(parent_id);
            match parent_idx {
                Some(pi) if self.node_table[pi].id == parent_id => {
                    if self.node_table[pi].left_child_id == -1 {
                        self.node_table[pi].left_child_id = id;
                    } else if self.node_table[pi].right_child_id == -1 {
                        self.node_table[pi].right_child_id = id;
                    }
                }
                _ => return -1,
            }
        }

        self.node_count += 1;
        (self.node_count - 1) as c_int
    }

    fn calculate_tree_sum(&self, node_id: c_int) -> c_int {
        let idx = match self.find_node_by_id(node_id) {
            Some(i) if self.node_table[i].id == node_id => i,
            _ => return 0,
        };

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

fn label_contains(label: &[u8; 32], c: u8) -> bool {
    // Match strchr on a null-terminated C string
    for &b in label.iter() {
        if b == 0 { break; }
        if b == c { return true; }
    }
    false
}

fn parse_operation(op_char: u8) -> c_int {
    // parse_operation checks: NULL→ADD, '+'→ADD, '*'→MUL, '-'→SUB, '/'→DIV, '%'→MOD
    // We receive a single char; the C code does strchr on a 1-char string.
    match op_char {
        b'+' => 1,
        b'*' => 2,
        b'-' => 3,
        b'/' => 4,
        b'%' => 5,
        _ => 1,
    }
}

fn apply_op(op: c_int, a: c_int, b: c_int) -> c_int {
    match op {
        1 => a.wrapping_add(b),
        2 => a.wrapping_mul(b),
        3 => a.wrapping_sub(b),
        4 => {
            if b == 0 { 0 } else { a.wrapping_div(b) }
        }
        5 => {
            if b == 0 { 0 } else { a.wrapping_rem(b) }
        }
        _ => a.wrapping_add(b),
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
    for i in 0..state.node_count {
        if label_contains(&state.node_table[i].label, b'l') {
            target_id = state.node_table[i].id;
            break;
        }
    }

    let target = state.find_node_by_id(target_id);
    match target {
        Some(ti) if state.node_table[ti].value != 0 => {}
        _ => { target_id = 1; }
    }

    let tree_sum = state.calculate_tree_sum(1);

    let op_string: &[u8] = b"+*-%";
    // C: op_string[tree_sum % 4] — replicate C signed modulo + pointer arithmetic
    let idx = tree_sum % 4;
    let op_char = if idx >= 0 && (idx as usize) < op_string.len() {
        op_string[idx as usize]
    } else {
        // UB in C for negative index; use wrapping pointer offset to match
        unsafe { *op_string.as_ptr().offset(idx as isize) }
    };

    let op = parse_operation(op_char);
    apply_op(op, tree_sum, target_id)
}
