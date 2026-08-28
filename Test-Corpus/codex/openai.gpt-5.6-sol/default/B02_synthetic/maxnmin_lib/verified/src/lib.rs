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
    value: c_double,
    active: c_int,
}

impl Node {
    const ZERO: Self = Self {
        id: 0,
        parent_id: 0,
        name: [0; MAX_NAME_LEN],
        value: 0.0,
        active: 0,
    };
}

static mut NODE_STORAGE: [Node; MAX_NODES] = [Node::ZERO; MAX_NODES];
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

    let mut i = 0;
    while i < MAX_NAME_LEN - 1 {
        let byte = unsafe { *name.add(i) };
        new_node.name[i] = byte;
        if byte == 0 {
            i += 1;
            while i < MAX_NAME_LEN - 1 {
                new_node.name[i] = 0;
                i += 1;
            }
            break;
        }
        i += 1;
    }
    new_node.name[MAX_NAME_LEN - 1] = 0;

    let index = unsafe { NODE_COUNT as usize };
    let storage = ptr::addr_of_mut!(NODE_STORAGE).cast::<Node>();
    unsafe {
        storage.add(index).write(new_node);
        NODE_COUNT += 1;
        NODE_COUNT - 1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_node_by_id(id: c_int) -> *mut Node {
    let count = unsafe { NODE_COUNT };
    let storage = ptr::addr_of_mut!(NODE_STORAGE).cast::<Node>();
    let mut i = 0;

    while i < count {
        let node = unsafe { storage.add(i as usize) };
        if unsafe { (*node).id == id && (*node).active != 0 } {
            return node;
        }
        i += 1;
    }

    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_children_count(parent_id: c_int) -> c_int {
    let count = unsafe { NODE_COUNT };
    let storage = ptr::addr_of!(NODE_STORAGE).cast::<Node>();
    let mut children = 0_i32;
    let mut i = 0;

    while i < count {
        let node = unsafe { storage.add(i as usize) };
        if unsafe { (*node).parent_id == parent_id && (*node).active != 0 } {
            children = children.wrapping_add(1);
        }
        i += 1;
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
    let mut i = 0;

    while i < count {
        let child = unsafe { storage.add(i as usize) };
        if unsafe { (*child).parent_id == node_id && (*child).active != 0 } {
            sum += unsafe { calculate_subtree_sum((*child).id) };
        }
        i += 1;
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
pub extern "C" fn maxnmin(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result = 0_i32;

    unsafe {
        NODE_COUNT = 0;

        add_node(1, -1, c"root".as_ptr(), 10.5);
        add_node(2, 1, c"child1".as_ptr(), 20.7);
        add_node(3, 1, c"child2".as_ptr(), 15.3);
        add_node(4, 2, c"grandchild1".as_ptr(), 5.9);
        add_node(5, 2, c"grandchild2".as_ptr(), 8.2);
        add_node(6, 3, c"grandchild3".as_ptr(), 12.4);

        let node_id = (param1 % 6).wrapping_add(1);
        let selected_node = find_node_by_id(node_id);

        if !selected_node.is_null() {
            let name_ptr = ptr::addr_of_mut!((*selected_node).name).cast::<c_char>();

            if *name_ptr != 0 {
                result = result.wrapping_add(process_string(name_ptr));
            }

            let subtree_sum = calculate_subtree_sum(node_id);
            let sum_as_int = safe_double_to_int(subtree_sum);
            result = result.wrapping_add(sum_as_int);
        }

        let second_node_id = (param2 % 6).wrapping_add(1);
        let second_node = find_node_by_id(second_node_id);

        if !second_node.is_null() {
            let value_multiplied = (*second_node).value * param3 as c_double;
            let converted_value = safe_double_to_int(value_multiplied);
            result = result.wrapping_add(converted_value);
        }

        let parent_id = (param4 % 3).wrapping_add(1);
        let children = get_children_count(parent_id);
        result = result.wrapping_add(children.wrapping_mul(10));

        let calculation = (param1.wrapping_add(param2) as c_double)
            / (param3.wrapping_add(1) as c_double)
            * param4 as c_double;
        let final_calc = safe_double_to_int(calculation);
        result = result.wrapping_add(final_calc);
    }

    result
}
