use std::cell::UnsafeCell;
use std::ffi::{c_char, c_double, c_int};
use std::ptr;

const MAX_NODES: usize = 100;
const MAX_NAME_LEN: usize = 50;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Node {
    pub id: c_int,
    pub parent_id: c_int,
    pub name: [c_char; MAX_NAME_LEN],
    pub value: c_double,
    pub active: c_int,
}

const EMPTY_NODE: Node = Node {
    id: 0,
    parent_id: 0,
    name: [0; MAX_NAME_LEN],
    value: 0.0,
    active: 0,
};

struct Global<T>(UnsafeCell<T>);

unsafe impl<T> Sync for Global<T> {}

static NODE_STORAGE: Global<[Node; MAX_NODES]> = Global(UnsafeCell::new([EMPTY_NODE; MAX_NODES]));
static NODE_COUNT: Global<c_int> = Global(UnsafeCell::new(0));

unsafe fn node_count() -> c_int {
    unsafe { *NODE_COUNT.0.get() }
}

unsafe fn set_node_count(value: c_int) {
    unsafe {
        *NODE_COUNT.0.get() = value;
    }
}

unsafe fn node_ptr(index: usize) -> *mut Node {
    unsafe { (NODE_STORAGE.0.get() as *mut Node).add(index) }
}

#[unsafe(no_mangle)]
pub extern "C" fn add_node(
    id: c_int,
    parent_id: c_int,
    name: *const c_char,
    value: c_double,
) -> c_int {
    unsafe {
        let count = node_count();
        if count >= MAX_NODES as c_int {
            return -1;
        }

        let mut new_node = Node {
            id,
            parent_id,
            name: [0; MAX_NAME_LEN],
            value,
            active: 1,
        };

        let mut stopped = false;
        for i in 0..(MAX_NAME_LEN - 1) {
            if stopped {
                new_node.name[i] = 0;
            } else {
                let ch = ptr::read(name.add(i));
                new_node.name[i] = ch;
                if ch == 0 {
                    stopped = true;
                }
            }
        }
        new_node.name[MAX_NAME_LEN - 1] = 0;

        let index = count as usize;
        ptr::write(node_ptr(index), new_node);
        set_node_count(count + 1);
        node_count() - 1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn find_node_by_id(id: c_int) -> *mut Node {
    unsafe {
        let count = node_count();
        let mut i = 0;
        while i < count {
            let node = node_ptr(i as usize);
            if (*node).id == id && (*node).active != 0 {
                return node;
            }
            i += 1;
        }
        ptr::null_mut()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_children_count(parent_id: c_int) -> c_int {
    unsafe {
        let mut count = 0;
        let node_count = node_count();
        let mut i = 0;
        while i < node_count {
            let node = node_ptr(i as usize);
            if (*node).parent_id == parent_id && (*node).active != 0 {
                count += 1;
            }
            i += 1;
        }
        count
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn calculate_subtree_sum(node_id: c_int) -> c_double {
    unsafe {
        let node = find_node_by_id(node_id);
        if node.is_null() {
            return 0.0;
        }

        let mut sum = (*node).value;
        let count = node_count();
        let mut i = 0;
        while i < count {
            let child = node_ptr(i as usize);
            if (*child).parent_id == node_id && (*child).active != 0 {
                sum += calculate_subtree_sum((*child).id);
            }
            i += 1;
        }

        sum
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn process_string(mut str: *mut c_char) -> c_int {
    unsafe {
        let mut result: c_int = 0;

        if ptr::read(str) != 0 {
            while ptr::read(str) != 0 {
                result = result.wrapping_add(ptr::read(str) as c_int);
                str = str.add(1);
            }
        }

        result
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    if d > c_int::MAX as c_double {
        return c_int::MAX;
    }
    if d < c_int::MIN as c_double {
        return c_int::MIN;
    }

    if d != d {
        return 0;
    }

    d as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn maxnmin(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    unsafe {
        let mut result: c_int = 0;

        set_node_count(0);

        add_node(1, -1, b"root\0".as_ptr() as *const c_char, 10.5);
        add_node(2, 1, b"child1\0".as_ptr() as *const c_char, 20.7);
        add_node(3, 1, b"child2\0".as_ptr() as *const c_char, 15.3);
        add_node(4, 2, b"grandchild1\0".as_ptr() as *const c_char, 5.9);
        add_node(5, 2, b"grandchild2\0".as_ptr() as *const c_char, 8.2);
        add_node(6, 3, b"grandchild3\0".as_ptr() as *const c_char, 12.4);

        let node_id = (param1 % 6) + 1;
        let selected_node = find_node_by_id(node_id);

        if !selected_node.is_null() {
            let name_ptr = (*selected_node).name.as_mut_ptr();

            if ptr::read(name_ptr) != 0 {
                result = result.wrapping_add(process_string(name_ptr));
            }

            let subtree_sum = calculate_subtree_sum(node_id);

            let sum_as_int = safe_double_to_int(subtree_sum);
            result = result.wrapping_add(sum_as_int);
        }

        let second_node_id = (param2 % 6) + 1;
        let second_node = find_node_by_id(second_node_id);

        if !second_node.is_null() {
            let value_multiplied = (*second_node).value * param3 as c_double;

            let converted_value = safe_double_to_int(value_multiplied);
            result = result.wrapping_add(converted_value);
        }

        let parent_id = (param4 % 3) + 1;
        let children = get_children_count(parent_id);
        result = result.wrapping_add(children.wrapping_mul(10));

        let mut calculation =
            (param1.wrapping_add(param2) as c_double) / (param3.wrapping_add(1) as c_double);
        calculation *= param4 as c_double;

        let final_calc = safe_double_to_int(calculation);
        result = result.wrapping_add(final_calc);

        result
    }
}
