// Rust translation of c_src/src/lib.c
// Must produce byte-identical results to the C implementation.

use std::os::raw::{c_char, c_double, c_int};

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

impl Node {
    const fn zeroed() -> Self {
        Node {
            id: 0,
            parent_id: 0,
            name: [0; MAX_NAME_LEN],
            value: 0.0,
            active: 0,
        }
    }
}

static mut NODE_STORAGE: [Node; MAX_NODES] = [Node::zeroed(); MAX_NODES];
static mut NODE_COUNT: c_int = 0;

/// # Safety
/// `name` must point to a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn add_node(
    id: c_int,
    parent_id: c_int,
    name: *const c_char,
    value: c_double,
) -> c_int {
    if NODE_COUNT as usize >= MAX_NODES {
        return -1;
    }

    let mut new_node = Node {
        id,
        parent_id,
        name: [0; MAX_NAME_LEN],
        value,
        active: 1,
    };

    // Replicate `strncpy(new_node.name, name, MAX_NAME_LEN - 1);`
    // strncpy copies up to n bytes; if src is shorter, the remainder is
    // padded with NULs. Since `new_node.name` was zero-initialized above,
    // padding is already done. We just need to copy up to (MAX_NAME_LEN - 1)
    // bytes, stopping at the source NUL terminator.
    let mut i = 0usize;
    while i < MAX_NAME_LEN - 1 {
        let b = *name.add(i);
        if b == 0 {
            break;
        }
        new_node.name[i] = b;
        i += 1;
    }
    // Explicit terminator (matches `new_node.name[MAX_NAME_LEN - 1] = '\0';`)
    new_node.name[MAX_NAME_LEN - 1] = 0;

    NODE_STORAGE[NODE_COUNT as usize] = new_node;
    NODE_COUNT += 1;
    NODE_COUNT - 1
}

/// # Safety
/// Returns a raw pointer into the static `NODE_STORAGE` array.
#[no_mangle]
pub unsafe extern "C" fn find_node_by_id(id: c_int) -> *mut Node {
    let mut i = 0i32;
    while i < NODE_COUNT {
        let n = &mut NODE_STORAGE[i as usize];
        if n.id == id && n.active != 0 {
            return n as *mut Node;
        }
        i += 1;
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn get_children_count(parent_id: c_int) -> c_int {
    let mut count: c_int = 0;
    let mut i = 0i32;
    while i < NODE_COUNT {
        let n = &NODE_STORAGE[i as usize];
        if n.parent_id == parent_id && n.active != 0 {
            count += 1;
        }
        i += 1;
    }
    count
}

#[no_mangle]
pub unsafe extern "C" fn calculate_subtree_sum(node_id: c_int) -> c_double {
    let node = find_node_by_id(node_id);
    if node.is_null() {
        return 0.0;
    }

    let mut sum: c_double = (*node).value;

    let mut i = 0i32;
    while i < NODE_COUNT {
        let n_parent_id = NODE_STORAGE[i as usize].parent_id;
        let n_active = NODE_STORAGE[i as usize].active;
        let n_id = NODE_STORAGE[i as usize].id;
        if n_parent_id == node_id && n_active != 0 {
            sum += calculate_subtree_sum(n_id);
        }
        i += 1;
    }

    sum
}

/// # Safety
/// `str` must point to a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn process_string(mut s: *mut c_char) -> c_int {
    let mut result: c_int = 0;

    if *s != 0 {
        while *s != 0 {
            // Match C's `result += (int)(*str);` — `char` may be signed,
            // so sign-extend through `i8` first.
            result = result.wrapping_add(*s as i8 as c_int);
            s = s.add(1);
        }
    }

    result
}

#[no_mangle]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    if d > c_int::MAX as c_double {
        return c_int::MAX;
    }
    if d < c_int::MIN as c_double {
        return c_int::MIN;
    }

    // NaN check — `d != d` in C.
    if d != d {
        return 0;
    }

    // C's (int) conversion truncates toward zero. Replicating that
    // for finite values within range.
    d as c_int
}

#[no_mangle]
pub unsafe extern "C" fn maxnmin(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int = 0;

    NODE_COUNT = 0;

    let root = b"root\0".as_ptr() as *const c_char;
    let child1 = b"child1\0".as_ptr() as *const c_char;
    let child2 = b"child2\0".as_ptr() as *const c_char;
    let grandchild1 = b"grandchild1\0".as_ptr() as *const c_char;
    let grandchild2 = b"grandchild2\0".as_ptr() as *const c_char;
    let grandchild3 = b"grandchild3\0".as_ptr() as *const c_char;

    add_node(1, -1, root, 10.5);
    add_node(2, 1, child1, 20.7);
    add_node(3, 1, child2, 15.3);
    add_node(4, 2, grandchild1, 5.9);
    add_node(5, 2, grandchild2, 8.2);
    add_node(6, 3, grandchild3, 12.4);

    let node_id = (param1 % 6) + 1;
    let selected_node = find_node_by_id(node_id);

    if !selected_node.is_null() {
        let name_ptr = (*selected_node).name.as_mut_ptr();

        if *name_ptr != 0 {
            result = result.wrapping_add(process_string(name_ptr));
        }

        let subtree_sum = calculate_subtree_sum(node_id);

        let sum_as_int = safe_double_to_int(subtree_sum);
        result = result.wrapping_add(sum_as_int);
    }

    let second_node_id = (param2 % 6) + 1;
    let second_node = find_node_by_id(second_node_id);

    if !second_node.is_null() {
        let value_multiplied = (*second_node).value * (param3 as c_double);

        let converted_value = safe_double_to_int(value_multiplied);
        result = result.wrapping_add(converted_value);
    }

    let parent_id = (param4 % 3) + 1;
    let children = get_children_count(parent_id);
    result = result.wrapping_add(children.wrapping_mul(10));

    let calculation =
        (param1.wrapping_add(param2) as c_double) / (param3.wrapping_add(1) as c_double);
    let calculation = calculation * (param4 as c_double);

    let final_calc = safe_double_to_int(calculation);
    result = result.wrapping_add(final_calc);

    result
}
