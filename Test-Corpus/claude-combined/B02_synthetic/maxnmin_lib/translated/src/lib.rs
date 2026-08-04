// Rust translation of c_src/src/lib.c
// Preserves byte-identical behavior with the original C library.

use std::ffi::c_char;
use std::ffi::c_double;
use std::ffi::c_int;
use std::sync::Mutex;

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

struct NodeStorage {
    nodes: [Node; MAX_NODES],
    count: usize,
}

impl NodeStorage {
    const fn new() -> Self {
        NodeStorage {
            nodes: [Node::zeroed(); MAX_NODES],
            count: 0,
        }
    }
}

// Global storage protected by a mutex. The C code uses unsynchronized statics;
// we use a Mutex here so concurrent FFI callers don't trigger UB on the Rust
// side, while preserving the same observable single-threaded semantics.
static STORAGE: Mutex<NodeStorage> = Mutex::new(NodeStorage::new());

// Mimic strncpy(dst, src, MAX_NAME_LEN - 1), then dst[MAX_NAME_LEN - 1] = '\0'
fn copy_name_into(dst: &mut [c_char; MAX_NAME_LEN], src: *const c_char) {
    // Copy up to MAX_NAME_LEN - 1 bytes, stopping at NUL. Pad remainder with NUL.
    // Then forcibly set the last byte to NUL.
    let n = MAX_NAME_LEN - 1;
    let mut hit_null = false;
    for i in 0..n {
        if hit_null {
            dst[i] = 0;
        } else {
            // Read one byte from src.
            let b = unsafe { *src.add(i) };
            dst[i] = b;
            if b == 0 {
                hit_null = true;
            }
        }
    }
    dst[MAX_NAME_LEN - 1] = 0;
}

fn add_node_impl(
    storage: &mut NodeStorage,
    id: c_int,
    parent_id: c_int,
    name: *const c_char,
    value: c_double,
) -> c_int {
    if storage.count >= MAX_NODES {
        return -1;
    }

    let mut new_node = Node {
        id,
        parent_id,
        name: [0; MAX_NAME_LEN],
        value,
        active: 1,
    };

    copy_name_into(&mut new_node.name, name);

    storage.nodes[storage.count] = new_node;
    storage.count += 1;
    (storage.count as c_int) - 1
}

#[unsafe(no_mangle)]
pub extern "C" fn add_node(
    id: c_int,
    parent_id: c_int,
    name: *const c_char,
    value: c_double,
) -> c_int {
    let mut storage = STORAGE.lock().unwrap();
    add_node_impl(&mut storage, id, parent_id, name, value)
}

fn find_node_index(storage: &NodeStorage, id: c_int) -> Option<usize> {
    for i in 0..storage.count {
        if storage.nodes[i].id == id && storage.nodes[i].active != 0 {
            return Some(i);
        }
    }
    None
}

#[unsafe(no_mangle)]
pub extern "C" fn find_node_by_id(id: c_int) -> *mut Node {
    // Return a pointer into the static storage, mimicking the C function.
    // We deliberately bypass the mutex here because the C function returns a
    // raw pointer to the static buffer with no synchronization either; the
    // caller must manage their own concurrency. Use a raw pointer to the
    // mutex-protected data.
    let mut storage = STORAGE.lock().unwrap();
    match find_node_index(&storage, id) {
        Some(i) => {
            let p: *mut Node = &mut storage.nodes[i] as *mut Node;
            p
        }
        None => std::ptr::null_mut(),
    }
}

fn get_children_count_impl(storage: &NodeStorage, parent_id: c_int) -> c_int {
    let mut count: c_int = 0;
    for i in 0..storage.count {
        if storage.nodes[i].parent_id == parent_id && storage.nodes[i].active != 0 {
            count += 1;
        }
    }
    count
}

#[unsafe(no_mangle)]
pub extern "C" fn get_children_count(parent_id: c_int) -> c_int {
    let storage = STORAGE.lock().unwrap();
    get_children_count_impl(&storage, parent_id)
}

fn calculate_subtree_sum_impl(storage: &NodeStorage, node_id: c_int) -> c_double {
    let idx = match find_node_index(storage, node_id) {
        Some(i) => i,
        None => return 0.0,
    };

    let mut sum: c_double = storage.nodes[idx].value;

    for i in 0..storage.count {
        if storage.nodes[i].parent_id == node_id && storage.nodes[i].active != 0 {
            sum += calculate_subtree_sum_impl(storage, storage.nodes[i].id);
        }
    }

    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn calculate_subtree_sum(node_id: c_int) -> c_double {
    let storage = STORAGE.lock().unwrap();
    calculate_subtree_sum_impl(&storage, node_id)
}

#[unsafe(no_mangle)]
pub extern "C" fn process_string(str_ptr: *mut c_char) -> c_int {
    let mut result: c_int = 0;

    // The C code performs `if (*str)` (read once), then loops while (*str).
    // Replicate that exactly with raw pointer reads.
    unsafe {
        if *str_ptr != 0 {
            let mut p = str_ptr;
            while *p != 0 {
                // (int)(*str) — char in C is sign-extended on most platforms;
                // signed char on x86_64 Linux. We must match this exactly.
                // On linux x86_64, c_char = i8.
                let ch = *p;
                result = result.wrapping_add(ch as c_int);
                p = p.add(1);
            }
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

    if d != d {
        return 0;
    }

    d as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn maxnmin(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    {
        let mut storage = STORAGE.lock().unwrap();
        storage.count = 0;

        // Helper closures aren't trivial here (need &mut), use direct calls.
        let names: [&[u8]; 6] = [
            b"root\0",
            b"child1\0",
            b"child2\0",
            b"grandchild1\0",
            b"grandchild2\0",
            b"grandchild3\0",
        ];
        let params: [(c_int, c_int, c_double); 6] = [
            (1, -1, 10.5),
            (2, 1, 20.7),
            (3, 1, 15.3),
            (4, 2, 5.9),
            (5, 2, 8.2),
            (6, 3, 12.4),
        ];
        for i in 0..6 {
            let (id, parent_id, value) = params[i];
            let name_ptr = names[i].as_ptr() as *const c_char;
            add_node_impl(&mut storage, id, parent_id, name_ptr, value);
        }
    }

    let node_id = (param1 % 6) + 1;

    // First node block
    {
        // Need mutable access to storage to obtain a *mut for selected_node.
        // We replicate what the C code does: take a pointer to the node and
        // read its name and id. We can do this in safe Rust by copying out
        // what we need under the lock.
        let storage = STORAGE.lock().unwrap();
        let idx_opt = find_node_index(&storage, node_id);
        if let Some(idx) = idx_opt {
            // name pointer in C: selected_node->name. Then `if (*name_ptr)`
            // checks whether the first byte is non-zero.
            let first_byte = storage.nodes[idx].name[0];
            // We'll release the lock before calling process_string to be safe,
            // but we need to capture the name bytes first because process_string
            // reads from a raw pointer.
            // The C original does the read while pointing into the static buffer.
            // We replicate: copy the name bytes (up to and including NUL) into
            // a local buffer.
            let mut name_copy: [c_char; MAX_NAME_LEN] = [0; MAX_NAME_LEN];
            name_copy.copy_from_slice(&storage.nodes[idx].name);
            drop(storage); // release lock before calling other functions

            if first_byte != 0 {
                let ptr = name_copy.as_mut_ptr();
                result = result.wrapping_add(process_string(ptr));
            }

            let subtree_sum = calculate_subtree_sum(node_id);
            let sum_as_int = safe_double_to_int(subtree_sum);
            result = result.wrapping_add(sum_as_int);
        }
    }

    let second_node_id = (param2 % 6) + 1;
    {
        let storage = STORAGE.lock().unwrap();
        let idx_opt = find_node_index(&storage, second_node_id);
        if let Some(idx) = idx_opt {
            // double value_multiplied = second_node->value * param3;
            // In C, param3 (int) is converted to double for the multiplication.
            let value_multiplied = storage.nodes[idx].value * (param3 as c_double);
            drop(storage);
            let converted_value = safe_double_to_int(value_multiplied);
            result = result.wrapping_add(converted_value);
        }
    }

    let parent_id = (param4 % 3) + 1;
    let children = get_children_count(parent_id);
    result = result.wrapping_add(children.wrapping_mul(10));

    // double calculation = (double)(param1 + param2) / (double)(param3 + 1);
    // In C, (param1 + param2) is computed in int (with possible signed
    // overflow UB) then cast to double. Likewise (param3 + 1).
    let sum12 = param1.wrapping_add(param2);
    let p3_plus_1 = param3.wrapping_add(1);
    let mut calculation = (sum12 as c_double) / (p3_plus_1 as c_double);
    // calculation *= param4;
    calculation *= param4 as c_double;

    let final_calc = safe_double_to_int(calculation);
    result = result.wrapping_add(final_calc);

    result
}
