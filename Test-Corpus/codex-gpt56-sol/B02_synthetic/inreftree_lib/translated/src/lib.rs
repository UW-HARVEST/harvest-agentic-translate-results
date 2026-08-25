use std::ffi::{c_char, c_int};
use std::ptr;

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

impl TreeNode {
    const ZERO: Self = Self {
        id: 0,
        value: 0,
        parent_id: 0,
        left_child_id: 0,
        right_child_id: 0,
        label: [0; 32],
    };
}

pub type OperationFunc = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[unsafe(no_mangle)]
pub static mut node_table: [TreeNode; MAX_NODES as usize] = [TreeNode::ZERO; MAX_NODES as usize];

#[unsafe(no_mangle)]
pub static mut node_count: c_int = 0;

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
    if b == 0 { 0 } else { a.wrapping_div(b) }
}

#[unsafe(no_mangle)]
pub extern "C" fn modulo_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 { 0 } else { a.wrapping_rem(b) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_node_by_id(id: c_int) -> *mut TreeNode {
    let mut i = 0;
    while i < unsafe { node_count } {
        let node = unsafe { (&raw mut node_table).cast::<TreeNode>().offset(i as isize) };
        if unsafe { (*node).id } == id {
            return node;
        }
        i += 1;
    }
    ptr::null_mut()
}

unsafe fn copy_label(destination: *mut c_char, source: *const c_char) {
    let mut found_nul = false;
    for i in 0..31 {
        let byte = if found_nul {
            0
        } else {
            let byte = unsafe { *source.add(i) };
            found_nul = byte == 0;
            byte
        };
        unsafe {
            *destination.add(i) = byte;
        }
    }
    unsafe {
        *destination.add(31) = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_tree_node(
    id: c_int,
    value: c_int,
    parent_id: c_int,
    label: *const c_char,
) -> c_int {
    if unsafe { node_count } >= MAX_NODES {
        return -1;
    }

    let node = unsafe {
        (&raw mut node_table)
            .cast::<TreeNode>()
            .offset(node_count as isize)
    };
    unsafe {
        (*node).id = id;
        (*node).value = value;
        (*node).parent_id = parent_id;
        (*node).left_child_id = -1;
        (*node).right_child_id = -1;
        copy_label((&raw mut (*node).label).cast::<c_char>(), label);
    }

    if parent_id != -1 {
        let parent = unsafe { find_node_by_id(parent_id) };
        if parent.is_null() || unsafe { (*parent).id } != parent_id {
            return -1;
        }

        if unsafe { (*parent).left_child_id } == -1 {
            unsafe {
                (*parent).left_child_id = id;
            }
        } else if unsafe { (*parent).right_child_id } == -1 {
            unsafe {
                (*parent).right_child_id = id;
            }
        }
    }

    unsafe {
        node_count += 1;
        node_count - 1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn calculate_tree_sum(node_id: c_int) -> c_int {
    let node = unsafe { find_node_by_id(node_id) };
    if node.is_null() || unsafe { (*node).id } != node_id {
        return 0;
    }

    let mut sum = unsafe { (*node).value };
    if unsafe { (*node).left_child_id } != -1 {
        sum = sum.wrapping_add(unsafe { calculate_tree_sum((*node).left_child_id) });
    }
    if unsafe { (*node).right_child_id } != -1 {
        sum = sum.wrapping_add(unsafe { calculate_tree_sum((*node).right_child_id) });
    }
    sum
}

unsafe fn contains_byte(mut string: *const c_char, needle: u8) -> bool {
    loop {
        let byte = unsafe { *string }.cast_unsigned();
        if byte == needle {
            return true;
        }
        if byte == 0 {
            return false;
        }
        string = unsafe { string.add(1) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_operation(op_str: *const c_char) -> c_int {
    if op_str.is_null() || unsafe { contains_byte(op_str, b'+') } {
        return OP_ADD;
    }
    if unsafe { contains_byte(op_str, b'*') } {
        return OP_MULTIPLY;
    }
    if unsafe { contains_byte(op_str, b'-') } {
        return OP_SUBTRACT;
    }
    if unsafe { contains_byte(op_str, b'/') } {
        return OP_DIVIDE;
    }
    if unsafe { contains_byte(op_str, b'%') } {
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
    unsafe {
        node_count = 0;

        add_tree_node(1, param1, -1, c"root".as_ptr());
        add_tree_node(2, param2, 1, c"left".as_ptr());
        add_tree_node(3, param3, 1, c"right".as_ptr());
        add_tree_node(4, param4, 2, c"left-left".as_ptr());
    }

    let mut target_id = -1;
    let mut i = 0;
    while i < unsafe { node_count } {
        let node = unsafe { (&raw mut node_table).cast::<TreeNode>().offset(i as isize) };
        if unsafe { contains_byte((&raw const (*node).label).cast::<c_char>(), b'l') } {
            target_id = unsafe { (*node).id };
            break;
        }
        i += 1;
    }

    let target = unsafe { find_node_by_id(target_id) };
    if target.is_null() || unsafe { (*target).value } == 0 {
        target_id = 1;
    }

    let tree_sum = unsafe { calculate_tree_sum(1) };
    let remainder = tree_sum % 4;
    let op_char = match remainder {
        0 => b'+',
        1 => b'*',
        2 => b'-',
        3 => b'%',
        // In the C build these indices read the three bytes preceding "+*-%".
        -1 => 0,
        -2 => b't',
        -3 => b'f',
        _ => unreachable!(),
    };
    let op_string = [op_char.cast_signed(), 0];
    let op = unsafe { parse_operation(op_string.as_ptr()) };
    let function = get_operation_func(op);

    unsafe { function(tree_sum, target_id, 0, 0) }
}
