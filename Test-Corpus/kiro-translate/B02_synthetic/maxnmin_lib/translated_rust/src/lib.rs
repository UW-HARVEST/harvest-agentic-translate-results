use std::ffi::c_int;
use std::os::raw::c_char;

const MAX_NODES: usize = 100;
const MAX_NAME_LEN: usize = 50;

#[derive(Clone, Copy)]
struct Node {
    id: c_int,
    parent_id: c_int,
    name: [u8; MAX_NAME_LEN],
    value: f64,
    active: c_int,
}

impl Node {
    const fn zeroed() -> Self {
        Node { id: 0, parent_id: 0, name: [0; MAX_NAME_LEN], value: 0.0, active: 0 }
    }
}

static mut NODE_STORAGE: [Node; MAX_NODES] = [Node::zeroed(); MAX_NODES];
static mut NODE_COUNT: usize = 0;

unsafe fn add_node(id: c_int, parent_id: c_int, name: *const c_char, value: f64) -> c_int {
    if NODE_COUNT >= MAX_NODES {
        return -1;
    }
    let mut new_node = Node { id, parent_id, name: [0; MAX_NAME_LEN], value, active: 1 };
    // strncpy behavior: copy up to MAX_NAME_LEN-1 bytes, ensure null termination
    let name_bytes = std::ffi::CStr::from_ptr(name).to_bytes();
    let copy_len = name_bytes.len().min(MAX_NAME_LEN - 1);
    new_node.name[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
    new_node.name[MAX_NAME_LEN - 1] = 0;
    NODE_STORAGE[NODE_COUNT] = new_node;
    NODE_COUNT += 1;
    (NODE_COUNT - 1) as c_int
}

unsafe fn find_node_by_id(id: c_int) -> *mut Node {
    for i in 0..NODE_COUNT {
        if NODE_STORAGE[i].id == id && NODE_STORAGE[i].active != 0 {
            return &mut NODE_STORAGE[i] as *mut Node;
        }
    }
    std::ptr::null_mut()
}

unsafe fn get_children_count(parent_id: c_int) -> c_int {
    let mut count = 0;
    for i in 0..NODE_COUNT {
        if NODE_STORAGE[i].parent_id == parent_id && NODE_STORAGE[i].active != 0 {
            count += 1;
        }
    }
    count
}

unsafe fn calculate_subtree_sum(node_id: c_int) -> f64 {
    let node = find_node_by_id(node_id);
    if node.is_null() {
        return 0.0;
    }
    let mut sum = (*node).value;
    for i in 0..NODE_COUNT {
        if NODE_STORAGE[i].parent_id == node_id && NODE_STORAGE[i].active != 0 {
            sum += calculate_subtree_sum(NODE_STORAGE[i].id);
        }
    }
    sum
}

fn process_string(mut str_ptr: *const u8) -> c_int {
    let mut result: c_int = 0;
    unsafe {
        if *str_ptr != 0 {
            while *str_ptr != 0 {
                result += *str_ptr as c_int;
                str_ptr = str_ptr.add(1);
            }
        }
    }
    result
}

fn safe_double_to_int(d: f64) -> c_int {
    if d > c_int::MAX as f64 {
        return c_int::MAX;
    }
    if d < c_int::MIN as f64 {
        return c_int::MIN;
    }
    if d != d {
        return 0;
    }
    d as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn maxnmin(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    unsafe {
        let mut result: c_int = 0;
        NODE_COUNT = 0;

        add_node(1, -1, b"root\0".as_ptr() as *const c_char, 10.5);
        add_node(2, 1, b"child1\0".as_ptr() as *const c_char, 20.7);
        add_node(3, 1, b"child2\0".as_ptr() as *const c_char, 15.3);
        add_node(4, 2, b"grandchild1\0".as_ptr() as *const c_char, 5.9);
        add_node(5, 2, b"grandchild2\0".as_ptr() as *const c_char, 8.2);
        add_node(6, 3, b"grandchild3\0".as_ptr() as *const c_char, 12.4);

        let node_id = (param1 % 6) + 1;
        let selected_node = find_node_by_id(node_id);

        if !selected_node.is_null() {
            let name_ptr = (*selected_node).name.as_ptr();
            if *name_ptr != 0 {
                result += process_string(name_ptr);
            }
            let subtree_sum = calculate_subtree_sum(node_id);
            let sum_as_int = safe_double_to_int(subtree_sum);
            result += sum_as_int;
        }

        let second_node_id = (param2 % 6) + 1;
        let second_node = find_node_by_id(second_node_id);

        if !second_node.is_null() {
            let value_multiplied = (*second_node).value * param3 as f64;
            let converted_value = safe_double_to_int(value_multiplied);
            result += converted_value;
        }

        let parent_id = (param4 % 3) + 1;
        let children = get_children_count(parent_id);
        result += children * 10;

        let calculation = (param1 + param2) as f64 / (param3 + 1) as f64 * param4 as f64;
        let final_calc = safe_double_to_int(calculation);
        result += final_calc;

        result
    }
}
