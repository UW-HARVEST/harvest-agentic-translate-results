use std::ffi::{c_char, c_int};
use std::ptr;

const OP_ADD: c_int = 1;
const OP_MULTIPLY: c_int = 2;
const OP_SUBTRACT: c_int = 3;
const OP_DIVIDE: c_int = 4;
const OP_MODULO: c_int = 5;

const MAX_NODES: usize = 50;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct TreeNode {
    id: c_int,
    value: c_int,
    parent_id: c_int,
    left_child_id: c_int,
    right_child_id: c_int,
    label: [c_char; 32],
}

const EMPTY_NODE: TreeNode = TreeNode {
    id: 0,
    value: 0,
    parent_id: 0,
    left_child_id: 0,
    right_child_id: 0,
    label: [0; 32],
};

type OperationFunc = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

static mut NODE_TABLE: [TreeNode; MAX_NODES] = [EMPTY_NODE; MAX_NODES];
static mut NODE_COUNT: c_int = 0;

#[unsafe(no_mangle)]
pub extern "C" fn add_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_mul(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn subtract_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_sub(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn divide_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a.wrapping_div(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn modulo_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a.wrapping_rem(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn find_node_by_id(id: c_int) -> *mut TreeNode {
    unsafe {
        let table = ptr::addr_of_mut!(NODE_TABLE) as *mut TreeNode;
        let mut i = 0;
        while i < NODE_COUNT {
            let node = table.add(i as usize);
            if (*node).id == id {
                return node;
            }
            i += 1;
        }
    }
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_tree_node(
    id: c_int,
    value: c_int,
    parent_id: c_int,
    label: *const c_char,
) -> c_int {
    unsafe {
        if NODE_COUNT >= MAX_NODES as c_int {
            return -1;
        }

        let table = ptr::addr_of_mut!(NODE_TABLE) as *mut TreeNode;
        let node = table.add(NODE_COUNT as usize);
        (*node).id = id;
        (*node).value = value;
        (*node).parent_id = parent_id;
        (*node).left_child_id = -1;
        (*node).right_child_id = -1;

        let mut i = 0usize;
        while i < 31 {
            let ch = *label.add(i);
            (*node).label[i] = ch;
            if ch == 0 {
                i += 1;
                while i < 31 {
                    (*node).label[i] = 0;
                    i += 1;
                }
                break;
            }
            i += 1;
        }
        (*node).label[31] = 0;

        if parent_id != -1 {
            let parent = find_node_by_id(parent_id);
            if parent.is_null() || (*parent).id != parent_id {
                return -1;
            }

            if (*parent).left_child_id == -1 {
                (*parent).left_child_id = id;
            } else if (*parent).right_child_id == -1 {
                (*parent).right_child_id = id;
            }
        }

        NODE_COUNT += 1;
        NODE_COUNT - 1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn calculate_tree_sum(node_id: c_int) -> c_int {
    unsafe {
        let node = find_node_by_id(node_id);

        if node.is_null() || (*node).id != node_id {
            return 0;
        }

        let mut sum = (*node).value;

        if (*node).left_child_id != -1 {
            sum = sum.wrapping_add(calculate_tree_sum((*node).left_child_id));
        }

        if (*node).right_child_id != -1 {
            sum = sum.wrapping_add(calculate_tree_sum((*node).right_child_id));
        }

        sum
    }
}

unsafe fn c_strchr(mut s: *const c_char, needle: c_char) -> *const c_char {
    unsafe {
        loop {
            let ch = *s;
            if ch == needle {
                return s;
            }
            if ch == 0 {
                return ptr::null();
            }
            s = s.add(1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_operation(op_str: *const c_char) -> c_int {
    unsafe {
        if op_str.is_null() || !c_strchr(op_str, b'+' as c_char).is_null() {
            return OP_ADD;
        }
        if !c_strchr(op_str, b'*' as c_char).is_null() {
            return OP_MULTIPLY;
        }
        if !c_strchr(op_str, b'-' as c_char).is_null() {
            return OP_SUBTRACT;
        }
        if !c_strchr(op_str, b'/' as c_char).is_null() {
            return OP_DIVIDE;
        }
        if !c_strchr(op_str, b'%' as c_char).is_null() {
            return OP_MODULO;
        }
        OP_ADD
    }
}

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

#[unsafe(no_mangle)]
pub extern "C" fn inreftree(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    unsafe {
        NODE_COUNT = 0;

        add_tree_node(1, param1, -1, c"root".as_ptr());
        add_tree_node(2, param2, 1, c"left".as_ptr());
        add_tree_node(3, param3, 1, c"right".as_ptr());
        add_tree_node(4, param4, 2, c"left-left".as_ptr());

        let mut target_id = -1;
        let table = ptr::addr_of_mut!(NODE_TABLE) as *mut TreeNode;
        let mut i = 0;
        while i < NODE_COUNT {
            let node = table.add(i as usize);
            if !c_strchr((*node).label.as_ptr(), b'l' as c_char).is_null() {
                target_id = (*node).id;
                break;
            }
            i += 1;
        }

        let target = find_node_by_id(target_id);
        if target.is_null() || (*target).value == 0 {
            target_id = 1;
        }

        let tree_sum = calculate_tree_sum(1);

        let op_string = c"+*-%";
        let op_index = tree_sum % 4;
        let op_char = [*op_string.as_ptr().offset(op_index as isize), 0];
        let op = parse_operation(op_char.as_ptr());

        let _op_value = op as c_int;

        let func = get_operation_func(op);

        func(tree_sum, target_id, 0, 0)
    }
}
