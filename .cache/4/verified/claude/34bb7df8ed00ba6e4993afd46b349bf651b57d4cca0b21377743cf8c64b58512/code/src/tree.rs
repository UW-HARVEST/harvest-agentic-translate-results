//! Faithful translation of `c_src/src/tree.c` / `c_src/include/tree.h`.
//!
//! Nodes are individually `malloc`'d and stored in the hashmap as `void *`,
//! exactly like the original.  Fields the C code leaves uninitialised (most
//! notably `child_ids`, and `data[1..]` when `data == NULL`) are left
//! uninitialised here as well.

#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_void};
use core::mem::size_of;
use core::ptr;

use crate::cio;
use crate::hashmap::{
    hashmap_create, hashmap_destroy, hashmap_get, hashmap_put, hashmap_remove, hashmap_t, tree_id_t,
};

/// `#define MAX_CHILDREN 32`
pub const MAX_CHILDREN: usize = 32;
/// `#define MAX_DATA_LENGTH 256`
pub const MAX_DATA_LENGTH: usize = 256;

/// ```c
/// typedef struct tree_node {
///     tree_id_t id;
///     tree_id_t parent_id;
///     tree_id_t child_ids[MAX_CHILDREN];
///     int child_count;
///     char data[MAX_DATA_LENGTH];
/// } tree_node_t;
/// ```
#[repr(C)]
pub struct tree_node_t {
    pub id: tree_id_t,
    pub parent_id: tree_id_t,
    pub child_ids: [tree_id_t; MAX_CHILDREN],
    pub child_count: c_int,
    pub data: [c_char; MAX_DATA_LENGTH],
}

/// ```c
/// typedef struct {
///     hashmap_t *node_map;
///     tree_id_t root_id;
///     int has_root;
///     size_t node_count;
/// } tree_t;
/// ```
#[repr(C)]
pub struct tree_t {
    pub node_map: *mut hashmap_t,
    pub root_id: tree_id_t,
    pub has_root: c_int,
    pub node_count: usize,
}

// Layout parity with the reference platform (x86-64 LP64).
const _: () = assert!(size_of::<tree_node_t>() == 536);
const _: () = assert!(size_of::<tree_t>() == 32);

/// `tree_t* tree_create(void)`
#[no_mangle]
pub unsafe extern "C" fn tree_create() -> *mut tree_t {
    let tree = libc::malloc(size_of::<tree_t>()) as *mut tree_t;
    if tree.is_null() {
        return ptr::null_mut();
    }

    (*tree).node_map = hashmap_create();
    if (*tree).node_map.is_null() {
        libc::free(tree as *mut c_void);
        return ptr::null_mut();
    }

    (*tree).root_id = 0;
    (*tree).has_root = 0;
    (*tree).node_count = 0;

    tree
}

/// ```c
/// static void tree_free_node(tree_node_t *node) {
///     if (node) { free(node); }
/// }
/// ```
unsafe fn tree_free_node(node: *mut tree_node_t) {
    if !node.is_null() {
        libc::free(node as *mut c_void);
    }
}

/// `void tree_delete(tree_t *tree)`
#[no_mangle]
pub unsafe extern "C" fn tree_delete(tree: *mut tree_t) {
    if tree.is_null() {
        return;
    }

    // Free all nodes in the hashmap
    let map = (*tree).node_map;
    let mut i: usize = 0;
    while i < (*map).capacity {
        let e = (*map).entries.add(i);
        if (*e).occupied != 0 && (*e).deleted == 0 {
            tree_free_node((*e).value as *mut tree_node_t);
        }
        i += 1;
    }

    hashmap_destroy((*tree).node_map);
    libc::free(tree as *mut c_void);
}

/// `strncpy(dst, src, n)` -- copies at most `n` bytes, stopping after a NUL,
/// and NUL-pads the remainder of the `n` bytes.
unsafe fn strncpy_n(dst: *mut c_char, src: *const c_char, n: usize) {
    let mut i: usize = 0;
    while i < n && *src.add(i) != 0 {
        *dst.add(i) = *src.add(i);
        i += 1;
    }
    while i < n {
        *dst.add(i) = 0;
        i += 1;
    }
}

/// The NUL-terminated contents of `data`, as `printf("%s", node->data)` renders
/// them.
unsafe fn c_str_bytes(p: *const c_char) -> &'static [u8] {
    let mut len: usize = 0;
    while *p.add(len) != 0 {
        len += 1;
    }
    core::slice::from_raw_parts(p as *const u8, len)
}

/// `int tree_add_node(tree_t *tree, tree_id_t id, tree_id_t parent_id, const char *data)`
#[no_mangle]
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
        ceprintf!("Error: Node with ID {} already exists\n", id);
        return -1;
    }

    // Allocate new node
    let node = libc::malloc(size_of::<tree_node_t>()) as *mut tree_node_t;
    if node.is_null() {
        ceprintf!("Error: Failed to allocate node\n");
        return -1;
    }

    (*node).id = id;
    (*node).parent_id = parent_id;
    (*node).child_count = 0;

    if !data.is_null() {
        let dst = ptr::addr_of_mut!((*node).data) as *mut c_char;
        strncpy_n(dst, data, MAX_DATA_LENGTH - 1);
        *dst.add(MAX_DATA_LENGTH - 1) = 0;
    } else {
        let dst = ptr::addr_of_mut!((*node).data) as *mut c_char;
        *dst = 0;
    }

    // If this is the first node, make it the root
    if (*tree).has_root == 0 {
        (*tree).root_id = id;
        (*tree).has_root = 1;
        (*node).parent_id = 0; // Root has no parent
    } else {
        // Find parent and add this node as a child
        let parent = tree_get_node(tree, parent_id);
        if parent.is_null() {
            ceprintf!("Error: Parent node {} not found\n", parent_id);
            libc::free(node as *mut c_void);
            return -1;
        }

        if (*parent).child_count as usize >= MAX_CHILDREN {
            ceprintf!("Error: Parent has maximum children\n");
            libc::free(node as *mut c_void);
            return -1;
        }

        let slot = (*parent).child_count;
        (*parent).child_count = slot + 1;
        *ptr::addr_of_mut!((*parent).child_ids[slot as usize]) = id;
    }

    // Add to hashmap
    if hashmap_put((*tree).node_map, id, node as *mut c_void) != 0 {
        ceprintf!("Error: Failed to add node to hashmap\n");
        libc::free(node as *mut c_void);
        return -1;
    }

    (*tree).node_count = (*tree).node_count.wrapping_add(1);
    0
}

/// `static int tree_remove_subtree(tree_t *tree, tree_id_t id)`
unsafe fn tree_remove_subtree(tree: *mut tree_t, id: tree_id_t) -> c_int {
    let node = tree_get_node(tree, id);
    if node.is_null() {
        return -1;
    }

    // Recursively remove all children first
    let mut i: c_int = 0;
    while i < (*node).child_count {
        tree_remove_subtree(tree, (*node).child_ids[i as usize]);
        i += 1;
    }

    // Remove this node from hashmap
    let removed = hashmap_remove((*tree).node_map, id) as *mut tree_node_t;
    if !removed.is_null() {
        tree_free_node(removed);
        // C wraps this `size_t`; a hand-built `tree_t` can make it underflow.
        (*tree).node_count = (*tree).node_count.wrapping_sub(1);
    }

    0
}

/// `int tree_remove_node(tree_t *tree, tree_id_t id)`
#[no_mangle]
pub unsafe extern "C" fn tree_remove_node(tree: *mut tree_t, id: tree_id_t) -> c_int {
    if tree.is_null() {
        return -1;
    }

    let node = tree_get_node(tree, id);
    if node.is_null() {
        ceprintf!("Error: Node {} not found\n", id);
        return -1;
    }

    // If removing root, tree becomes empty
    if id == (*tree).root_id {
        tree_remove_subtree(tree, id);
        (*tree).has_root = 0;
        (*tree).root_id = 0;
        return 0;
    }

    // Remove from parent's child list
    let parent = tree_get_node(tree, (*node).parent_id);
    if !parent.is_null() {
        let mut i: c_int = 0;
        while i < (*parent).child_count {
            if (*parent).child_ids[i as usize] == id {
                // Shift remaining children
                let mut j: c_int = i;
                while j < (*parent).child_count - 1 {
                    (*parent).child_ids[j as usize] = (*parent).child_ids[(j + 1) as usize];
                    j += 1;
                }
                (*parent).child_count = (*parent).child_count.wrapping_sub(1);
                break;
            }
            i += 1;
        }
    }

    // Remove this node and all descendants
    tree_remove_subtree(tree, id);

    0
}

/// `tree_node_t* tree_get_node(tree_t *tree, tree_id_t id)`
#[no_mangle]
pub unsafe extern "C" fn tree_get_node(tree: *mut tree_t, id: tree_id_t) -> *mut tree_node_t {
    if tree.is_null() {
        return ptr::null_mut();
    }

    hashmap_get((*tree).node_map, id) as *mut tree_node_t
}

/// `int tree_contains(tree_t *tree, tree_id_t id)`
#[no_mangle]
pub unsafe extern "C" fn tree_contains(tree: *mut tree_t, id: tree_id_t) -> c_int {
    (!tree_get_node(tree, id).is_null()) as c_int
}

/// `size_t tree_size(tree_t *tree)`
#[no_mangle]
pub unsafe extern "C" fn tree_size(tree: *mut tree_t) -> usize {
    if !tree.is_null() {
        (*tree).node_count
    } else {
        0
    }
}

/// `static void tree_print_helper(tree_t *tree, tree_id_t id, int depth)`
unsafe fn tree_print_helper(tree: *mut tree_t, id: tree_id_t, depth: c_int) {
    let node = tree_get_node(tree, id);
    if node.is_null() {
        return;
    }

    // Print indentation
    let mut i: c_int = 0;
    while i < depth {
        cprintf!("  ");
        i += 1;
    }

    // printf("[%lu] %s\n", node->id, node->data);
    cprintf!("[{}] ", (*node).id);
    cio::out_bytes(c_str_bytes(ptr::addr_of!((*node).data) as *const c_char));
    cio::out_bytes(b"\n");

    // Print children
    let mut i: c_int = 0;
    while i < (*node).child_count {
        tree_print_helper(tree, (*node).child_ids[i as usize], depth + 1);
        i += 1;
    }
}

/// `void tree_print(tree_t *tree)`
#[no_mangle]
pub unsafe extern "C" fn tree_print(tree: *mut tree_t) {
    if tree.is_null() || (*tree).has_root == 0 {
        cprintf!("(empty tree)\n");
        return;
    }

    tree_print_helper(tree, (*tree).root_id, 0);
}

/// `int tree_get_depth(tree_t *tree, tree_id_t id)`
#[no_mangle]
pub unsafe extern "C" fn tree_get_depth(tree: *mut tree_t, id: tree_id_t) -> c_int {
    if tree.is_null() || tree_contains(tree, id) == 0 {
        return -1;
    }

    let mut depth: c_int = 0;
    let mut current_id = id;

    while current_id != (*tree).root_id {
        let node = tree_get_node(tree, current_id);
        if node.is_null() {
            return -1;
        }
        current_id = (*node).parent_id;
        depth = depth.wrapping_add(1);
    }

    depth
}

/// `int tree_get_height(tree_t *tree, tree_id_t id)`
#[no_mangle]
pub unsafe extern "C" fn tree_get_height(tree: *mut tree_t, id: tree_id_t) -> c_int {
    let node = tree_get_node(tree, id);
    if node.is_null() {
        return -1;
    }

    if (*node).child_count == 0 {
        return 0;
    }

    let mut max_height: c_int = 0;
    let mut i: c_int = 0;
    while i < (*node).child_count {
        let child_height = tree_get_height(tree, (*node).child_ids[i as usize]);
        if child_height > max_height {
            max_height = child_height;
        }
        i += 1;
    }

    max_height.wrapping_add(1)
}

/// `int tree_count_descendants(tree_t *tree, tree_id_t id)`
#[no_mangle]
pub unsafe extern "C" fn tree_count_descendants(tree: *mut tree_t, id: tree_id_t) -> c_int {
    let node = tree_get_node(tree, id);
    if node.is_null() {
        return -1;
    }

    let mut count: c_int = 0;
    let mut i: c_int = 0;
    while i < (*node).child_count {
        count = count.wrapping_add(1); // Count the child
        count = count.wrapping_add(tree_count_descendants(tree, (*node).child_ids[i as usize]));
        i += 1;
    }

    count
}

/// `int tree_find_path(tree_t *tree, tree_id_t id, tree_id_t *path, int max_length)`
#[no_mangle]
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
    let mut temp_path = [0u64; 1000];
    let mut length: c_int = 0;
    let mut current_id = id;

    while length < 1000 {
        temp_path[length as usize] = current_id;
        length += 1;

        if current_id == (*tree).root_id {
            break;
        }

        let node = tree_get_node(tree, current_id);
        if node.is_null() {
            return -1;
        }
        current_id = (*node).parent_id;
    }

    // Reverse into output path
    if length > max_length {
        length = max_length;
    }

    let mut i: c_int = 0;
    while i < length {
        *path.add(i as usize) = temp_path[(length - 1 - i) as usize];
        i += 1;
    }

    length
}
