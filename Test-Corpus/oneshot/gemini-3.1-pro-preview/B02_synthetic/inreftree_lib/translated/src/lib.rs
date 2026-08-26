use std::os::raw::c_int;

#[derive(Clone, Copy)]
enum Operation {
    Add = 1,
    Multiply = 2,
    Subtract = 3,
    Divide = 4,
    Modulo = 5,
}

struct TreeNode {
    id: c_int,
    value: c_int,
    parent_id: c_int,
    left_child_id: c_int,
    right_child_id: c_int,
    label: String,
}

struct TreeState {
    nodes: Vec<TreeNode>,
}

impl TreeState {
    fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    fn find_node_by_id(&self, id: c_int) -> Option<&TreeNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    fn add_tree_node(&mut self, id: c_int, value: c_int, parent_id: c_int, label: &str) -> c_int {
        if self.nodes.len() >= 50 {
            return -1;
        }

        if parent_id != -1 {
            if let Some(parent_idx) = self.nodes.iter().position(|n| n.id == parent_id) {
                let parent = &mut self.nodes[parent_idx];
                if parent.left_child_id == -1 {
                    parent.left_child_id = id;
                } else if parent.right_child_id == -1 {
                    parent.right_child_id = id;
                }
            } else {
                return -1;
            }
        }

        let mut label_str = label.to_string();
        label_str.truncate(31);

        let node = TreeNode {
            id,
            value,
            parent_id,
            left_child_id: -1,
            right_child_id: -1,
            label: label_str,
        };

        self.nodes.push(node);
        (self.nodes.len() - 1) as c_int
    }

    fn calculate_tree_sum(&self, node_id: c_int) -> c_int {
        let Some(node) = self.find_node_by_id(node_id) else {
            return 0;
        };

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
    if b == 0 { 0 } else { a.wrapping_div(b) }
}

fn modulo_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 { 0 } else { a.wrapping_rem(b) }
}

fn parse_operation(op_str: Option<&str>) -> Operation {
    let Some(s) = op_str else {
        return Operation::Add;
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
    match op {
        Operation::Add => add_op,
        Operation::Multiply => multiply_op,
        Operation::Subtract => subtract_op,
        Operation::Divide => divide_op,
        Operation::Modulo => modulo_op,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn inreftree(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut state = TreeState::new();

    state.add_tree_node(1, param1, -1, "root");
    state.add_tree_node(2, param2, 1, "left");
    state.add_tree_node(3, param3, 1, "right");
    state.add_tree_node(4, param4, 2, "left-left");

    let mut target_id = -1;
    for node in &state.nodes {
        if node.label.contains('l') {
            target_id = node.id;
            break;
        }
    }

    let target = state.find_node_by_id(target_id);
    if target.is_none() || target.unwrap().value == 0 {
        target_id = 1;
    }

    let tree_sum = state.calculate_tree_sum(1);

    let op_string = b"+*-%";
    let idx = (tree_sum.wrapping_rem(4).abs()) as usize;
    let op_char = op_string[idx] as char;
    let op_str = op_char.to_string();

    let op = parse_operation(Some(&op_str));
    let _op_value = op as c_int;

    let func = get_operation_func(op);

    func(tree_sum, target_id, 0, 0)
}
