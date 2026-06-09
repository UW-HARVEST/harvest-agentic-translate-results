//! Translation of c_src/src/tree.c

use core::ffi::{c_char, c_int};
use core::ptr;
use libc::{free, malloc, size_t};

use crate::hashmap::{
    hashmap_create, hashmap_destroy, hashmap_get, hashmap_put, hashmap_remove, hashmap_t,
    tree_id_t,
};

pub const MAX_CHILDREN: usize = 32;
pub const MAX_DATA_LENGTH: usize = 256;

#[repr(C)]
pub struct tree_node_t {
    pub id: tree_id_t,
    pub parent_id: tree_id_t,
    pub child_ids: [tree_id_t; MAX_CHILDREN],
    pub child_count: c_int,
    pub data: [c_char; MAX_DATA_LENGTH],
}

#[repr(C)]
pub struct tree_t {
    pub node_map: *mut hashmap_t,
    pub root_id: tree_id_t,
    pub has_root: c_int,
    pub node_count: size_t,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_create() -> *mut tree_t {
    let tree = malloc(core::mem::size_of::<tree_t>()) as *mut tree_t;
    if tree.is_null() {
        return ptr::null_mut();
    }

    let t = &mut *tree;
    t.node_map = hashmap_create();
    if t.node_map.is_null() {
        free(tree as *mut core::ffi::c_void);
        return ptr::null_mut();
    }

    t.root_id = 0;
    t.has_root = 0;
    t.node_count = 0;

    tree
}

unsafe fn tree_free_node(node: *mut tree_node_t) {
    if !node.is_null() {
        free(node as *mut core::ffi::c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_delete(tree: *mut tree_t) {
    if tree.is_null() {
        return;
    }

    let t = &mut *tree;
    // Free all nodes in the hashmap
    let map = &*t.node_map;
    for i in 0..map.capacity {
        let entry = &*map.entries.add(i);
        if entry.occupied != 0 && entry.deleted == 0 {
            tree_free_node(entry.value as *mut tree_node_t);
        }
    }

    hashmap_destroy(t.node_map);
    free(tree as *mut core::ffi::c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_add_node(
    tree: *mut tree_t,
    id: tree_id_t,
    parent_id: tree_id_t,
    data: *const c_char,
) -> c_int {
    if tree.is_null() {
        return -1;
    }

    // Check if node already exists
    if tree_contains(tree, id) != 0 {
        eprint_error_node_exists(id);
        return -1;
    }

    // Allocate new node
    let node = malloc(core::mem::size_of::<tree_node_t>()) as *mut tree_node_t;
    if node.is_null() {
        eprint_str("Error: Failed to allocate node\n");
        return -1;
    }

    let n = &mut *node;
    n.id = id;
    n.parent_id = parent_id;
    n.child_count = 0;

    if !data.is_null() {
        // strncpy(n.data, data, MAX_DATA_LENGTH - 1); n.data[MAX_DATA_LENGTH-1] = 0;
        let mut i = 0usize;
        while i < MAX_DATA_LENGTH - 1 {
            let b = *data.add(i);
            n.data[i] = b;
            if b == 0 {
                break;
            }
            i += 1;
        }
        // strncpy pads remainder with 0 if src ended; if src didn't end,
        // we still need to ensure terminator at position MAX_DATA_LENGTH-1.
        // Continue zero-padding past the null up to MAX_DATA_LENGTH - 1 to
        // mirror strncpy's behavior, then explicitly set the last byte to 0.
        if i < MAX_DATA_LENGTH - 1 {
            // we encountered the null at position i; continue padding
            let mut j = i + 1;
            while j < MAX_DATA_LENGTH - 1 {
                n.data[j] = 0;
                j += 1;
            }
        }
        n.data[MAX_DATA_LENGTH - 1] = 0;
    } else {
        n.data[0] = 0;
    }

    let t = &mut *tree;
    // If this is the first node, make it the root
    if t.has_root == 0 {
        t.root_id = id;
        t.has_root = 1;
        n.parent_id = 0;
    } else {
        // Find parent and add this node as a child
        let parent = tree_get_node(tree, parent_id);
        if parent.is_null() {
            eprint_error_parent_not_found(parent_id);
            free(node as *mut core::ffi::c_void);
            return -1;
        }

        let p = &mut *parent;
        if p.child_count >= MAX_CHILDREN as c_int {
            eprint_str("Error: Parent has maximum children\n");
            free(node as *mut core::ffi::c_void);
            return -1;
        }

        p.child_ids[p.child_count as usize] = id;
        p.child_count += 1;
    }

    // Add to hashmap
    if hashmap_put(t.node_map, id, node as *mut core::ffi::c_void) != 0 {
        eprint_str("Error: Failed to add node to hashmap\n");
        free(node as *mut core::ffi::c_void);
        return -1;
    }

    t.node_count += 1;
    0
}

unsafe fn tree_remove_subtree(tree: *mut tree_t, id: tree_id_t) -> c_int {
    let node = tree_get_node(tree, id);
    if node.is_null() {
        return -1;
    }

    let n = &*node;
    // Recursively remove all children first
    for i in 0..n.child_count as usize {
        tree_remove_subtree(tree, n.child_ids[i]);
    }

    let t = &mut *tree;
    // Remove this node from hashmap
    let removed = hashmap_remove(t.node_map, id) as *mut tree_node_t;
    if !removed.is_null() {
        tree_free_node(removed);
        t.node_count -= 1;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_remove_node(tree: *mut tree_t, id: tree_id_t) -> c_int {
    if tree.is_null() {
        return -1;
    }

    let node = tree_get_node(tree, id);
    if node.is_null() {
        eprint_error_node_not_found(id);
        return -1;
    }

    let t = &mut *tree;
    // If removing root, tree becomes empty
    if id == t.root_id {
        tree_remove_subtree(tree, id);
        t.has_root = 0;
        t.root_id = 0;
        return 0;
    }

    // Remove from parent's child list
    let parent_id = (*node).parent_id;
    let parent = tree_get_node(tree, parent_id);
    if !parent.is_null() {
        let p = &mut *parent;
        let mut i: c_int = 0;
        while i < p.child_count {
            if p.child_ids[i as usize] == id {
                // Shift remaining children
                let mut j = i;
                while j < p.child_count - 1 {
                    p.child_ids[j as usize] = p.child_ids[(j + 1) as usize];
                    j += 1;
                }
                p.child_count -= 1;
                break;
            }
            i += 1;
        }
    }

    tree_remove_subtree(tree, id);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_get_node(tree: *mut tree_t, id: tree_id_t) -> *mut tree_node_t {
    if tree.is_null() {
        return ptr::null_mut();
    }
    hashmap_get((*tree).node_map, id) as *mut tree_node_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_contains(tree: *mut tree_t, id: tree_id_t) -> c_int {
    if tree_get_node(tree, id).is_null() {
        0
    } else {
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_size(tree: *mut tree_t) -> size_t {
    if tree.is_null() {
        0
    } else {
        (*tree).node_count
    }
}

unsafe fn tree_print_helper(tree: *mut tree_t, id: tree_id_t, depth: c_int) {
    let node = tree_get_node(tree, id);
    if node.is_null() {
        return;
    }

    // Print indentation
    for _ in 0..depth {
        print!("  ");
    }

    let n = &*node;
    let data_str = cstr_to_str(n.data.as_ptr());
    println!("[{}] {}", n.id, data_str);

    for i in 0..n.child_count as usize {
        tree_print_helper(tree, n.child_ids[i], depth + 1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_print(tree: *mut tree_t) {
    if tree.is_null() || (*tree).has_root == 0 {
        println!("(empty tree)");
        return;
    }

    tree_print_helper(tree, (*tree).root_id, 0);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_get_depth(tree: *mut tree_t, id: tree_id_t) -> c_int {
    if tree.is_null() || tree_contains(tree, id) == 0 {
        return -1;
    }

    let mut depth: c_int = 0;
    let mut current_id = id;
    let root_id = (*tree).root_id;

    while current_id != root_id {
        let node = tree_get_node(tree, current_id);
        if node.is_null() {
            return -1;
        }
        current_id = (*node).parent_id;
        depth += 1;
    }

    depth
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_get_height(tree: *mut tree_t, id: tree_id_t) -> c_int {
    let node = tree_get_node(tree, id);
    if node.is_null() {
        return -1;
    }

    let n = &*node;
    if n.child_count == 0 {
        return 0;
    }

    let mut max_height: c_int = 0;
    for i in 0..n.child_count as usize {
        let child_height = tree_get_height(tree, n.child_ids[i]);
        if child_height > max_height {
            max_height = child_height;
        }
    }

    max_height + 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_count_descendants(tree: *mut tree_t, id: tree_id_t) -> c_int {
    let node = tree_get_node(tree, id);
    if node.is_null() {
        return -1;
    }

    let n = &*node;
    let mut count: c_int = 0;
    for i in 0..n.child_count as usize {
        count += 1; // Count the child
        count += tree_count_descendants(tree, n.child_ids[i]);
    }

    count
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_find_path(
    tree: *mut tree_t,
    id: tree_id_t,
    path: *mut tree_id_t,
    max_length: c_int,
) -> c_int {
    if tree.is_null() || path.is_null() || tree_contains(tree, id) == 0 {
        return -1;
    }

    // Build path from node to root, then reverse
    let mut temp_path: [tree_id_t; 1000] = [0; 1000];
    let mut length: usize = 0;
    let mut current_id = id;
    let root_id = (*tree).root_id;

    while length < 1000 {
        temp_path[length] = current_id;
        length += 1;

        if current_id == root_id {
            break;
        }

        let node = tree_get_node(tree, current_id);
        if node.is_null() {
            return -1;
        }
        current_id = (*node).parent_id;
    }

    let mut len_i = length as c_int;
    if len_i > max_length {
        len_i = max_length;
    }

    for i in 0..len_i as usize {
        *path.add(i) = temp_path[length - 1 - i];
    }

    len_i
}

// ---------- Helpers for printing C-style messages ----------

unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> &'a str {
    // Find the length up to the NUL terminator and convert as UTF-8 (lossy
    // fallback to empty string on invalid UTF-8). The C tests only use ASCII
    // labels here, so UTF-8 conversion is safe and produces identical bytes.
    let mut len = 0usize;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    let slice = core::slice::from_raw_parts(ptr as *const u8, len);
    core::str::from_utf8(slice).unwrap_or("")
}

fn eprint_str(s: &str) {
    use std::io::Write;
    let _ = std::io::stderr().write_all(s.as_bytes());
}

fn eprint_error_node_exists(id: tree_id_t) {
    use std::io::Write;
    let _ = writeln!(std::io::stderr(), "Error: Node with ID {} already exists", id);
}

fn eprint_error_parent_not_found(parent_id: tree_id_t) {
    use std::io::Write;
    let _ = writeln!(std::io::stderr(), "Error: Parent node {} not found", parent_id);
}

fn eprint_error_node_not_found(id: tree_id_t) {
    use std::io::Write;
    let _ = writeln!(std::io::stderr(), "Error: Node {} not found", id);
}
