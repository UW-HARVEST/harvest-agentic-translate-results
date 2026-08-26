use std::ffi::c_int;
use std::sync::Mutex;

const MAX_NODES: usize = 50;
const LABEL_LEN: usize = 32;

#[derive(Copy, Clone)]
#[repr(i32)]
enum Operation {
    Add = 1,
    Multiply = 2,
    Subtract = 3,
    Divide = 4,
    Modulo = 5,
}

#[derive(Copy, Clone)]
struct TreeNode {
    id: c_int,
    value: c_int,
    parent_id: c_int,
    left_child_id: c_int,
    right_child_id: c_int,
    label: [u8; LABEL_LEN],
}

impl TreeNode {
    const fn empty() -> Self {
        Self {
            id: 0,
            value: 0,
            parent_id: 0,
            left_child_id: 0,
            right_child_id: 0,
            label: [0; LABEL_LEN],
        }
    }
}

struct State {
    node_table: [TreeNode; MAX_NODES],
    node_count: usize,
}

impl State {
    const fn new() -> Self {
        Self {
            node_table: [TreeNode::empty(); MAX_NODES],
            node_count: 0,
        }
    }

    fn reset(&mut self) {
        self.node_count = 0;
    }

    fn find_node_by_id(&self, id: c_int) -> Option<usize> {
        let mut i = 0;
        while i < self.node_count {
            if self.node_table[i].id == id {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    fn add_tree_node(&mut self, id: c_int, value: c_int, parent_id: c_int, label: &[u8]) -> c_int {
        if self.node_count >= MAX_NODES {
            return -1;
        }

        let slot = self.node_count;
        let node = &mut self.node_table[slot];
        node.id = id;
        node.value = value;
        node.parent_id = parent_id;
        node.left_child_id = -1;
        node.right_child_id = -1;
        copy_label(&mut node.label, label);

        if parent_id != -1 {
            let Some(parent_index) = self.find_node_by_id(parent_id) else {
                return -1;
            };
            if self.node_table[parent_index].id != parent_id {
                return -1;
            }

            let parent = &mut self.node_table[parent_index];
            if parent.left_child_id == -1 {
                parent.left_child_id = id;
            } else if parent.right_child_id == -1 {
                parent.right_child_id = id;
            }
        }

        self.node_count += 1;
        (self.node_count - 1) as c_int
    }

    fn calculate_tree_sum(&self, node_id: c_int) -> c_int {
        let Some(index) = self.find_node_by_id(node_id) else {
            return 0;
        };
        let node = self.node_table[index];
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

static STATE: Mutex<State> = Mutex::new(State::new());

fn copy_label(dst: &mut [u8; LABEL_LEN], src: &[u8]) {
    let limit = 31usize;
    let mut i = 0;
    while i < limit {
        if i < src.len() {
            dst[i] = src[i];
            if src[i] == 0 {
                i += 1;
                while i < limit {
                    dst[i] = 0;
                    i += 1;
                }
                dst[31] = 0;
                return;
            }
        } else {
            dst[i] = 0;
        }
        i += 1;
    }
    dst[31] = 0;
}

fn parse_operation(op_str: Option<&[u8]>) -> Operation {
    let Some(op_str) = op_str else {
        return Operation::Add;
    };

    if contains_byte(op_str, b'+') {
        return Operation::Add;
    }
    if contains_byte(op_str, b'*') {
        return Operation::Multiply;
    }
    if contains_byte(op_str, b'-') {
        return Operation::Subtract;
    }
    if contains_byte(op_str, b'/') {
        return Operation::Divide;
    }
    if contains_byte(op_str, b'%') {
        return Operation::Modulo;
    }
    Operation::Add
}

fn contains_byte(buf: &[u8], needle: u8) -> bool {
    let mut i = 0;
    while i < buf.len() {
        if buf[i] == 0 {
            return false;
        }
        if buf[i] == needle {
            return true;
        }
        i += 1;
    }
    false
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
    if b == 0 {
        return 0;
    }
    match a.checked_div(b) {
        Some(value) => value,
        None => a.wrapping_div(b),
    }
}

fn modulo_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    match a.checked_rem(b) {
        Some(value) => value,
        None => 0,
    }
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
    let mut state = STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    state.reset();

    state.add_tree_node(1, param1, -1, b"root");
    state.add_tree_node(2, param2, 1, b"left");
    state.add_tree_node(3, param3, 1, b"right");
    state.add_tree_node(4, param4, 2, b"left-left");

    let mut target_id = -1;
    let mut i = 0;
    while i < state.node_count {
        if contains_byte(&state.node_table[i].label, b'l') {
            target_id = state.node_table[i].id;
            break;
        }
        i += 1;
    }

    match state.find_node_by_id(target_id) {
        Some(index) if state.node_table[index].value != 0 => {}
        _ => target_id = 1,
    }

    let tree_sum = state.calculate_tree_sum(1);
    let op_string = b"+*-%";
    let op_index = tree_sum % 4;
    let op_char = [op_string[op_index as usize], 0];
    let op = parse_operation(Some(&op_char));

    let _op_value = op as c_int;
    let func = get_operation_func(op);
    func(tree_sum, target_id, 0, 0)
}
