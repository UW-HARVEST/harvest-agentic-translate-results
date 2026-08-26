use std::ffi::{c_char, c_double, c_int};
use std::ptr;

const MAX_NODES: usize = 100;
const MAX_NAME_LEN: usize = 50;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Node {
    pub id: c_int,
    pub parent_id: c_int,
    pub name: [c_char; MAX_NAME_LEN],
    pub value: c_double,
    pub active: c_int,
}

const ZERO_NODE: Node = Node {
    id: 0,
    parent_id: 0,
    name: [0; MAX_NAME_LEN],
    value: 0.0,
    active: 0,
};

static mut NODE_STORAGE: [Node; MAX_NODES] = [ZERO_NODE; MAX_NODES];
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
        value,
        active: 1,
    };

    let mut i = 0usize;
    while i < MAX_NAME_LEN - 1 {
        let ch = unsafe { *name.add(i) };
        new_node.name[i] = ch;
        if ch == 0 {
            break;
        }
        i += 1;
    }
    new_node.name[MAX_NAME_LEN - 1] = 0;

    let index = unsafe { NODE_COUNT as usize };
    unsafe {
        NODE_STORAGE[index] = new_node;
        NODE_COUNT += 1;
        NODE_COUNT - 1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_node_by_id(id: c_int) -> *mut Node {
    let mut i = 0;
    while i < unsafe { NODE_COUNT } {
        let node = unsafe { ptr::addr_of_mut!(NODE_STORAGE[i as usize]) };
        if unsafe { (*node).id == id && (*node).active != 0 } {
            return node;
        }
        i += 1;
    }
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_children_count(parent_id: c_int) -> c_int {
    let mut count = 0;
    let mut i = 0;
    while i < unsafe { NODE_COUNT } {
        let node = unsafe { ptr::addr_of!(NODE_STORAGE[i as usize]) };
        if unsafe { (*node).parent_id == parent_id && (*node).active != 0 } {
            count += 1;
        }
        i += 1;
    }
    count
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn calculate_subtree_sum(node_id: c_int) -> c_double {
    let node = unsafe { find_node_by_id(node_id) };
    if node.is_null() {
        return 0.0;
    }

    let mut sum = unsafe { (*node).value };
    let mut i = 0;
    while i < unsafe { NODE_COUNT } {
        let current = unsafe { ptr::addr_of!(NODE_STORAGE[i as usize]) };
        if unsafe { (*current).parent_id == node_id && (*current).active != 0 } {
            sum += unsafe { calculate_subtree_sum((*current).id) };
        }
        i += 1;
    }
    sum
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_string(mut str_: *mut c_char) -> c_int {
    let mut result: c_int = 0;

    if unsafe { *str_ } != 0 {
        while unsafe { *str_ } != 0 {
            result = result.wrapping_add(unsafe { *str_ as c_int });
            str_ = unsafe { str_.add(1) };
        }
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    if d > c_int::MAX as c_double {
        return c_int::MAX;
    }
    if d < c_int::MIN as c_double {
        return c_int::MIN;
    }
    if d.is_nan() {
        return 0;
    }
    d as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn maxnmin(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int = 0;

    unsafe {
        NODE_COUNT = 0;
    }

    unsafe {
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
        let name_ptr = unsafe { ptr::addr_of_mut!((*selected_node).name).cast::<c_char>() };

        if unsafe { *name_ptr } != 0 {
            result = result.wrapping_add(unsafe { process_string(name_ptr) });
        }

        let subtree_sum = unsafe { calculate_subtree_sum(node_id) };
        let sum_as_int = safe_double_to_int(subtree_sum);
        result = result.wrapping_add(sum_as_int);
    }

    let second_node_id = (param2 % 6) + 1;
    let second_node = unsafe { find_node_by_id(second_node_id) };

    if !second_node.is_null() {
        let value_multiplied = unsafe { (*second_node).value } * param3 as c_double;
        let converted_value = safe_double_to_int(value_multiplied);
        result = result.wrapping_add(converted_value);
    }

    let parent_id = (param4 % 3) + 1;
    let children = unsafe { get_children_count(parent_id) };
    result = result.wrapping_add(children.wrapping_mul(10));

    let mut calculation = (param1.wrapping_add(param2)) as c_double / (param3.wrapping_add(1)) as c_double;
    calculation *= param4 as c_double;

    let final_calc = safe_double_to_int(calculation);
    result = result.wrapping_add(final_calc);

    result
}
