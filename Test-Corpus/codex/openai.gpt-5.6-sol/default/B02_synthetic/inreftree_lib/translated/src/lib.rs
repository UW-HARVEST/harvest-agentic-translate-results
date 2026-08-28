use std::ffi::{c_char, c_int};
use std::ptr::{addr_of_mut, null_mut};

const MAX_NODES: c_int = 50;

const OP_ADD: c_int = 1;
const OP_MULTIPLY: c_int = 2;
const OP_SUBTRACT: c_int = 3;
const OP_DIVIDE: c_int = 4;
const OP_MODULO: c_int = 5;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TreeNode {
    pub id: c_int,
    pub value: c_int,
    pub parent_id: c_int,
    pub left_child_id: c_int,
    pub right_child_id: c_int,
    pub label: [c_char; 32],
}

const EMPTY_NODE: TreeNode = TreeNode {
    id: 0,
    value: 0,
    parent_id: 0,
    left_child_id: 0,
    right_child_id: 0,
    label: [0; 32],
};

#[unsafe(no_mangle)]
pub static mut node_table: [TreeNode; MAX_NODES as usize] = [EMPTY_NODE; MAX_NODES as usize];

#[unsafe(no_mangle)]
pub static mut node_count: c_int = 0;

pub type OperationFunc = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

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
    if b == 0 { 0 } else { a / b }
}

#[unsafe(no_mangle)]
pub extern "C" fn modulo_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 { 0 } else { a % b }
}

#[inline]
fn table_ptr() -> *mut TreeNode {
    addr_of_mut!(node_table).cast::<TreeNode>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_node_by_id(id: c_int) -> *mut TreeNode {
    let count = unsafe { node_count };
    let mut i = 0;

    while i < count {
        let node = unsafe { table_ptr().offset(i as isize) };
        if unsafe { (*node).id == id } {
            return node;
        }
        i += 1;
    }

    null_mut()
}

unsafe fn copy_label(destination: *mut c_char, source: *const c_char) {
    let mut i = 0usize;
    let mut reached_end = false;

    while i < 31 {
        let byte = if reached_end {
            0
        } else {
            let byte = unsafe { *source.add(i) };
            if byte == 0 {
                reached_end = true;
            }
            byte
        };
        unsafe { *destination.add(i) = byte };
        i += 1;
    }
    unsafe { *destination.add(31) = 0 };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_tree_node(
    id: c_int,
    value: c_int,
    parent_id: c_int,
    label: *const c_char,
) -> c_int {
    let count = unsafe { node_count };
    if count >= MAX_NODES {
        return -1;
    }

    let node = unsafe { table_ptr().offset(count as isize) };
    unsafe {
        (*node).id = id;
        (*node).value = value;
        (*node).parent_id = parent_id;
        (*node).left_child_id = -1;
        (*node).right_child_id = -1;
        copy_label(addr_of_mut!((*node).label).cast::<c_char>(), label);
    }

    if parent_id != -1 {
        let parent = unsafe { find_node_by_id(parent_id) };
        if parent.is_null() || unsafe { (*parent).id != parent_id } {
            return -1;
        }

        if unsafe { (*parent).left_child_id == -1 } {
            unsafe { (*parent).left_child_id = id };
        } else if unsafe { (*parent).right_child_id == -1 } {
            unsafe { (*parent).right_child_id = id };
        }
    }

    unsafe { node_count = count + 1 };
    count
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn calculate_tree_sum(node_id: c_int) -> c_int {
    let node = unsafe { find_node_by_id(node_id) };

    if node.is_null() || unsafe { (*node).id != node_id } {
        return 0;
    }

    let mut sum = unsafe { (*node).value };
    let left_child_id = unsafe { (*node).left_child_id };
    if left_child_id != -1 {
        sum = sum.wrapping_add(unsafe { calculate_tree_sum(left_child_id) });
    }

    let right_child_id = unsafe { (*node).right_child_id };
    if right_child_id != -1 {
        sum = sum.wrapping_add(unsafe { calculate_tree_sum(right_child_id) });
    }

    sum
}

unsafe fn contains_byte(string: *const c_char, needle: c_char) -> bool {
    let mut current = string;
    loop {
        let byte = unsafe { *current };
        if byte == needle {
            return true;
        }
        if byte == 0 {
            return false;
        }
        current = unsafe { current.add(1) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_operation(op_str: *const c_char) -> c_int {
    if op_str.is_null() || unsafe { contains_byte(op_str, b'+' as c_char) } {
        return OP_ADD;
    }
    if unsafe { contains_byte(op_str, b'*' as c_char) } {
        return OP_MULTIPLY;
    }
    if unsafe { contains_byte(op_str, b'-' as c_char) } {
        return OP_SUBTRACT;
    }
    if unsafe { contains_byte(op_str, b'/' as c_char) } {
        return OP_DIVIDE;
    }
    if unsafe { contains_byte(op_str, b'%' as c_char) } {
        return OP_MODULO;
    }
    OP_ADD
}

#[unsafe(no_mangle)]
pub extern "C" fn get_operation_func(op: c_int) -> OperationFunc {
    match op {
        OP_ADD => add_op,
        OP_MULTIPLY => multiply_op,
        OP_SUBTRACT => subtract_op,
        OP_DIVIDE => divide_op,
        OP_MODULO => modulo_op,
        _ => add_op,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn inreftree(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    unsafe { node_count = 0 };

    unsafe {
        add_tree_node(1, param1, -1, c"root".as_ptr());
        add_tree_node(2, param2, 1, c"left".as_ptr());
        add_tree_node(3, param3, 1, c"right".as_ptr());
        add_tree_node(4, param4, 2, c"left-left".as_ptr());
    }

    let mut target_id = -1;
    let count = unsafe { node_count };
    let mut i = 0;
    while i < count {
        let node = unsafe { table_ptr().offset(i as isize) };
        let label = unsafe { addr_of_mut!((*node).label).cast::<c_char>() };
        if unsafe { contains_byte(label, b'l' as c_char) } {
            target_id = unsafe { (*node).id };
            break;
        }
        i += 1;
    }

    let target = unsafe { find_node_by_id(target_id) };
    if target.is_null() || unsafe { (*target).value == 0 } {
        target_id = 1;
    }

    let tree_sum = unsafe { calculate_tree_sum(1) };

    let op_string = b"+*-%";
    let op_index = tree_sum % 4;
    let op_char = [
        unsafe { *op_string.as_ptr().offset(op_index as isize) } as c_char,
        0,
    ];
    let op = unsafe { parse_operation(op_char.as_ptr()) };

    let func = get_operation_func(op);
    unsafe { func(tree_sum, target_id, 0, 0) }
}
