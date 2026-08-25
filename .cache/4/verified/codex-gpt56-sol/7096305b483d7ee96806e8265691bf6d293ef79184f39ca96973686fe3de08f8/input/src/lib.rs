use std::ffi::{c_char, c_double, c_int};
use std::ptr;

const MAX_NODES: usize = 100;
const MAX_NAME_LEN: usize = 50;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Node {
    id: c_int,
    parent_id: c_int,
    name: [c_char; MAX_NAME_LEN],
    name_padding: [u8; 6],
    value: c_double,
    active: c_int,
    tail_padding: [u8; 4],
}

const EMPTY_NODE: Node = Node {
    id: 0,
    parent_id: 0,
    name: [0; MAX_NAME_LEN],
    name_padding: [0; 6],
    value: 0.0,
    active: 0,
    tail_padding: [0; 4],
};

static mut NODE_STORAGE: [Node; MAX_NODES] = [EMPTY_NODE; MAX_NODES];
static mut NODE_COUNT: c_int = 0;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_node(
    id: c_int,
    parent_id: c_int,
    name: *const c_char,
    value: c_double,
) -> c_int {
    if unsafe { NODE_COUNT } >= MAX_NODES as c_int {
        return -1;
    }

    let mut new_node = Node {
        id,
        parent_id,
        name: [0; MAX_NAME_LEN],
        name_padding: [0; 6],
        value,
        active: 1,
        tail_padding: [0; 4],
    };

    let mut index = 0;
    while index < MAX_NAME_LEN - 1 {
        let byte = unsafe { *name.add(index) };
        new_node.name[index] = byte;
        index += 1;
        if byte == 0 {
            break;
        }
    }
    new_node.name[MAX_NAME_LEN - 1] = 0;

    let node_index = unsafe { NODE_COUNT as usize };
    unsafe {
        NODE_STORAGE[node_index] = new_node;
        NODE_COUNT += 1;
        NODE_COUNT - 1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_node_by_id(id: c_int) -> *mut Node {
    let count = unsafe { NODE_COUNT };
    let storage = ptr::addr_of_mut!(NODE_STORAGE).cast::<Node>();

    let mut index = 0;
    while index < count {
        let node = unsafe { storage.add(index as usize) };
        if unsafe { (*node).id == id && (*node).active != 0 } {
            return node;
        }
        index += 1;
    }

    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_children_count(parent_id: c_int) -> c_int {
    let count = unsafe { NODE_COUNT };
    let storage = ptr::addr_of!(NODE_STORAGE).cast::<Node>();
    let mut children = 0_i32;

    let mut index = 0;
    while index < count {
        let node = unsafe { storage.add(index as usize) };
        if unsafe { (*node).parent_id == parent_id && (*node).active != 0 } {
            children += 1;
        }
        index += 1;
    }

    children
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn calculate_subtree_sum(node_id: c_int) -> c_double {
    let node = unsafe { find_node_by_id(node_id) };
    if node.is_null() {
        return 0.0;
    }

    let mut sum = unsafe { (*node).value };
    let count = unsafe { NODE_COUNT };
    let storage = ptr::addr_of!(NODE_STORAGE).cast::<Node>();

    let mut index = 0;
    while index < count {
        let child = unsafe { storage.add(index as usize) };
        if unsafe { (*child).parent_id == node_id && (*child).active != 0 } {
            sum += unsafe { calculate_subtree_sum((*child).id) };
        }
        index += 1;
    }

    sum
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_string(mut string: *mut c_char) -> c_int {
    let mut result = 0_i32;

    if unsafe { *string } != 0 {
        while unsafe { *string } != 0 {
            result = result.wrapping_add(unsafe { *string } as c_int);
            string = unsafe { string.add(1) };
        }
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(value: c_double) -> c_int {
    if value > c_int::MAX as c_double {
        return c_int::MAX;
    }
    if value < c_int::MIN as c_double {
        return c_int::MIN;
    }
    if value != value {
        return 0;
    }

    value as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn maxnmin(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result = 0_i32;

    unsafe {
        NODE_COUNT = 0;

        add_node(1, -1, c"root".as_ptr(), 10.5);
        add_node(2, 1, c"child1".as_ptr(), 20.7);
        add_node(3, 1, c"child2".as_ptr(), 15.3);
        add_node(4, 2, c"grandchild1".as_ptr(), 5.9);
        add_node(5, 2, c"grandchild2".as_ptr(), 8.2);
        add_node(6, 3, c"grandchild3".as_ptr(), 12.4);
    }

    let node_id = (param1 % 6) + 1;
    let selected_node = unsafe { find_node_by_id(node_id) };

    if !selected_node.is_null() {
        let name_ptr = unsafe { (*selected_node).name.as_mut_ptr() };
        if unsafe { *name_ptr } != 0 {
            result = result.wrapping_add(unsafe { process_string(name_ptr) });
        }

        let subtree_sum = unsafe { calculate_subtree_sum(node_id) };
        result = result.wrapping_add(safe_double_to_int(subtree_sum));
    }

    let second_node_id = (param2 % 6) + 1;
    let second_node = unsafe { find_node_by_id(second_node_id) };

    if !second_node.is_null() {
        let value_multiplied = unsafe { (*second_node).value } * f64::from(param3);
        result = result.wrapping_add(safe_double_to_int(value_multiplied));
    }

    let parent_id = (param4 % 3) + 1;
    let children = unsafe { get_children_count(parent_id) };
    result = result.wrapping_add(children.wrapping_mul(10));

    let numerator = f64::from(param1.wrapping_add(param2));
    let denominator = f64::from(param3.wrapping_add(1));
    let calculation = (numerator / denominator) * f64::from(param4);
    result = result.wrapping_add(safe_double_to_int(calculation));

    result
}
