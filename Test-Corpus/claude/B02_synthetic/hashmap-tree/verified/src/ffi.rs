// FFI module: C-ABI exports matching the C library exactly.
//
// This module re-implements hashmap and tree using `#[repr(C)]` structs and raw
// pointers so the Rust .so can be loaded via libloading and called with the
// same ABI as the C .so. Behavior must match the C source byte-for-byte.

use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

pub type tree_id_t = u64;

pub const HASHMAP_INITIAL_CAPACITY: usize = 16;
pub const HASHMAP_LOAD_FACTOR: f64 = 0.75;
pub const MAX_CHILDREN: usize = 32;
pub const MAX_DATA_LENGTH: usize = 256;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hashmap_entry_t {
    pub key: tree_id_t,
    pub value: *mut c_void,
    pub occupied: c_int,
    pub deleted: c_int,
}

impl hashmap_entry_t {
    const fn zeroed() -> Self {
        hashmap_entry_t {
            key: 0,
            value: ptr::null_mut(),
            occupied: 0,
            deleted: 0,
        }
    }
}

#[repr(C)]
pub struct hashmap_t {
    pub entries: *mut hashmap_entry_t,
    pub capacity: usize,
    pub size: usize,
    pub deleted_count: usize,
}

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
    pub node_count: usize,
}

// ---------- libc bindings used to mimic C's malloc/calloc/free/strncpy ----------
extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strncpy(dest: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
}

// ---------- Hash function ----------
fn hash_function(key: tree_id_t) -> u64 {
    // FNV-1a hash
    let mut hash: u64 = 14695981039346656037u64;
    let bytes = key.to_ne_bytes();
    for b in bytes.iter() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(1099511628211u64);
    }
    hash
}

unsafe fn should_resize(map: *mut hashmap_t) -> bool {
    let m = &*map;
    let load = (m.size + m.deleted_count) as f64 / m.capacity as f64;
    load > HASHMAP_LOAD_FACTOR
}

unsafe fn hashmap_resize_internal(map: *mut hashmap_t) -> c_int {
    let m = &mut *map;
    let old_capacity = m.capacity;
    let old_entries = m.entries;

    m.capacity *= 2;
    m.entries =
        calloc(m.capacity, std::mem::size_of::<hashmap_entry_t>()) as *mut hashmap_entry_t;
    if m.entries.is_null() {
        m.entries = old_entries;
        m.capacity = old_capacity;
        return -1;
    }

    m.size = 0;
    m.deleted_count = 0;

    // Rehash all entries
    for i in 0..old_capacity {
        let entry = &*old_entries.add(i);
        if entry.occupied != 0 && entry.deleted == 0 {
            hashmap_put(map, entry.key, entry.value);
        }
    }

    free(old_entries as *mut c_void);
    0
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_create() -> *mut hashmap_t {
    let map = malloc(std::mem::size_of::<hashmap_t>()) as *mut hashmap_t;
    if map.is_null() {
        return ptr::null_mut();
    }

    let m = &mut *map;
    m.capacity = HASHMAP_INITIAL_CAPACITY;
    m.size = 0;
    m.deleted_count = 0;
    m.entries =
        calloc(m.capacity, std::mem::size_of::<hashmap_entry_t>()) as *mut hashmap_entry_t;

    if m.entries.is_null() {
        free(map as *mut c_void);
        return ptr::null_mut();
    }

    map
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_destroy(map: *mut hashmap_t) {
    if !map.is_null() {
        let m = &*map;
        free(m.entries as *mut c_void);
        free(map as *mut c_void);
    }
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_put(
    map: *mut hashmap_t,
    key: tree_id_t,
    value: *mut c_void,
) -> c_int {
    if map.is_null() {
        return -1;
    }

    if should_resize(map) {
        if hashmap_resize_internal(map) != 0 {
            return -1;
        }
    }

    let m = &mut *map;
    let hash = hash_function(key);
    let index = (hash as usize) % m.capacity;

    for probe in 0..m.capacity {
        let current = (index + probe) % m.capacity;
        let entry = &mut *m.entries.add(current);

        if entry.occupied == 0 {
            entry.key = key;
            entry.value = value;
            entry.occupied = 1;
            entry.deleted = 0;
            m.size += 1;
            return 0;
        } else if entry.deleted != 0 {
            entry.key = key;
            entry.value = value;
            entry.deleted = 0;
            m.size += 1;
            m.deleted_count -= 1;
            return 0;
        } else if entry.key == key {
            entry.value = value;
            return 0;
        }
    }

    -1
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_get(map: *mut hashmap_t, key: tree_id_t) -> *mut c_void {
    if map.is_null() {
        return ptr::null_mut();
    }

    let m = &*map;
    let hash = hash_function(key);
    let index = (hash as usize) % m.capacity;

    for probe in 0..m.capacity {
        let current = (index + probe) % m.capacity;
        let entry = &*m.entries.add(current);

        if entry.occupied == 0 {
            return ptr::null_mut();
        }

        if entry.deleted == 0 && entry.key == key {
            return entry.value;
        }
    }

    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_remove(map: *mut hashmap_t, key: tree_id_t) -> *mut c_void {
    if map.is_null() {
        return ptr::null_mut();
    }

    let m = &mut *map;
    let hash = hash_function(key);
    let index = (hash as usize) % m.capacity;

    for probe in 0..m.capacity {
        let current = (index + probe) % m.capacity;
        let entry = &mut *m.entries.add(current);

        if entry.occupied == 0 {
            return ptr::null_mut();
        }

        if entry.deleted == 0 && entry.key == key {
            let value = entry.value;
            entry.deleted = 1;
            m.size -= 1;
            m.deleted_count += 1;
            return value;
        }
    }

    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_contains(map: *mut hashmap_t, key: tree_id_t) -> c_int {
    if hashmap_get(map, key).is_null() {
        0
    } else {
        1
    }
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_size(map: *mut hashmap_t) -> usize {
    if map.is_null() {
        0
    } else {
        (*map).size
    }
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_clear(map: *mut hashmap_t) {
    if map.is_null() {
        return;
    }

    let m = &mut *map;
    for i in 0..m.capacity {
        let entry = &mut *m.entries.add(i);
        entry.occupied = 0;
        entry.deleted = 0;
    }
    m.size = 0;
    m.deleted_count = 0;
}

// ---------- Tree functions ----------

#[no_mangle]
pub unsafe extern "C" fn tree_create() -> *mut tree_t {
    let tree = malloc(std::mem::size_of::<tree_t>()) as *mut tree_t;
    if tree.is_null() {
        return ptr::null_mut();
    }

    let t = &mut *tree;
    t.node_map = hashmap_create();
    if t.node_map.is_null() {
        free(tree as *mut c_void);
        return ptr::null_mut();
    }

    t.root_id = 0;
    t.has_root = 0;
    t.node_count = 0;

    tree
}

unsafe fn tree_free_node(node: *mut tree_node_t) {
    if !node.is_null() {
        free(node as *mut c_void);
    }
}

#[no_mangle]
pub unsafe extern "C" fn tree_delete(tree: *mut tree_t) {
    if tree.is_null() {
        return;
    }
    let t = &*tree;
    let m = &*t.node_map;

    for i in 0..m.capacity {
        let entry = &*m.entries.add(i);
        if entry.occupied != 0 && entry.deleted == 0 {
            tree_free_node(entry.value as *mut tree_node_t);
        }
    }

    hashmap_destroy(t.node_map);
    free(tree as *mut c_void);
}

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

    if tree_contains(tree, id) != 0 {
        eprintln!("Error: Node with ID {} already exists", id);
        return -1;
    }

    let node = malloc(std::mem::size_of::<tree_node_t>()) as *mut tree_node_t;
    if node.is_null() {
        eprintln!("Error: Failed to allocate node");
        return -1;
    }

    let n = &mut *node;
    n.id = id;
    n.parent_id = parent_id;
    n.child_count = 0;
    // Zero out child_ids (matches behavior since they're only used per child_count)
    for i in 0..MAX_CHILDREN {
        n.child_ids[i] = 0;
    }

    if !data.is_null() {
        strncpy(n.data.as_mut_ptr(), data, MAX_DATA_LENGTH - 1);
        n.data[MAX_DATA_LENGTH - 1] = 0;
    } else {
        n.data[0] = 0;
    }

    let t = &mut *tree;

    if t.has_root == 0 {
        t.root_id = id;
        t.has_root = 1;
        n.parent_id = 0;
    } else {
        let parent = tree_get_node(tree, parent_id);
        if parent.is_null() {
            eprintln!("Error: Parent node {} not found", parent_id);
            free(node as *mut c_void);
            return -1;
        }

        let p = &mut *parent;
        if p.child_count as usize >= MAX_CHILDREN {
            eprintln!("Error: Parent has maximum children");
            free(node as *mut c_void);
            return -1;
        }

        let idx = p.child_count as usize;
        p.child_ids[idx] = id;
        p.child_count += 1;
    }

    if hashmap_put(t.node_map, id, node as *mut c_void) != 0 {
        eprintln!("Error: Failed to add node to hashmap");
        free(node as *mut c_void);
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
    let cnt = n.child_count as usize;
    // Snapshot children
    let mut ids = [0u64; MAX_CHILDREN];
    for i in 0..cnt {
        ids[i] = n.child_ids[i];
    }

    for i in 0..cnt {
        tree_remove_subtree(tree, ids[i]);
    }

    let t = &mut *tree;
    let removed = hashmap_remove(t.node_map, id) as *mut tree_node_t;
    if !removed.is_null() {
        tree_free_node(removed);
        t.node_count -= 1;
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn tree_remove_node(tree: *mut tree_t, id: tree_id_t) -> c_int {
    if tree.is_null() {
        return -1;
    }

    let node = tree_get_node(tree, id);
    if node.is_null() {
        eprintln!("Error: Node {} not found", id);
        return -1;
    }

    let t = &mut *tree;

    if id == t.root_id {
        tree_remove_subtree(tree, id);
        t.has_root = 0;
        t.root_id = 0;
        return 0;
    }

    let n = &*node;
    let parent_id = n.parent_id;

    let parent = tree_get_node(tree, parent_id);
    if !parent.is_null() {
        let p = &mut *parent;
        let cnt = p.child_count as usize;
        let mut found: Option<usize> = None;
        for i in 0..cnt {
            if p.child_ids[i] == id {
                found = Some(i);
                break;
            }
        }
        if let Some(i) = found {
            for j in i..(cnt - 1) {
                p.child_ids[j] = p.child_ids[j + 1];
            }
            p.child_count -= 1;
        }
    }

    tree_remove_subtree(tree, id);
    0
}

#[no_mangle]
pub unsafe extern "C" fn tree_get_node(tree: *mut tree_t, id: tree_id_t) -> *mut tree_node_t {
    if tree.is_null() {
        return ptr::null_mut();
    }
    let t = &*tree;
    hashmap_get(t.node_map, id) as *mut tree_node_t
}

#[no_mangle]
pub unsafe extern "C" fn tree_contains(tree: *mut tree_t, id: tree_id_t) -> c_int {
    if tree_get_node(tree, id).is_null() {
        0
    } else {
        1
    }
}

#[no_mangle]
pub unsafe extern "C" fn tree_size(tree: *mut tree_t) -> usize {
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
    let n = &*node;
    for _ in 0..depth {
        print!("  ");
    }
    // Print data as C string
    let data_str = {
        let mut end = 0usize;
        while end < MAX_DATA_LENGTH && n.data[end] != 0 {
            end += 1;
        }
        let bytes: &[u8] =
            std::slice::from_raw_parts(n.data.as_ptr() as *const u8, end);
        std::str::from_utf8(bytes).unwrap_or("")
    };
    println!("[{}] {}", n.id, data_str);

    let cnt = n.child_count as usize;
    for i in 0..cnt {
        tree_print_helper(tree, n.child_ids[i], depth + 1);
    }
}

#[no_mangle]
pub unsafe extern "C" fn tree_print(tree: *mut tree_t) {
    if tree.is_null() || (*tree).has_root == 0 {
        println!("(empty tree)");
        return;
    }
    tree_print_helper(tree, (*tree).root_id, 0);
}

#[no_mangle]
pub unsafe extern "C" fn tree_get_depth(tree: *mut tree_t, id: tree_id_t) -> c_int {
    if tree.is_null() || tree_contains(tree, id) == 0 {
        return -1;
    }

    let t = &*tree;
    let mut depth: c_int = 0;
    let mut current_id = id;

    while current_id != t.root_id {
        let node = tree_get_node(tree, current_id);
        if node.is_null() {
            return -1;
        }
        current_id = (*node).parent_id;
        depth += 1;
    }

    depth
}

#[no_mangle]
pub unsafe extern "C" fn tree_get_height(tree: *mut tree_t, id: tree_id_t) -> c_int {
    let node = tree_get_node(tree, id);
    if node.is_null() {
        return -1;
    }
    let n = &*node;

    if n.child_count == 0 {
        return 0;
    }

    let cnt = n.child_count as usize;
    let mut max_height: c_int = 0;
    for i in 0..cnt {
        let child_height = tree_get_height(tree, n.child_ids[i]);
        if child_height > max_height {
            max_height = child_height;
        }
    }

    max_height + 1
}

#[no_mangle]
pub unsafe extern "C" fn tree_count_descendants(tree: *mut tree_t, id: tree_id_t) -> c_int {
    let node = tree_get_node(tree, id);
    if node.is_null() {
        return -1;
    }
    let n = &*node;

    let cnt = n.child_count as usize;
    let mut count: c_int = 0;
    for i in 0..cnt {
        count += 1;
        count += tree_count_descendants(tree, n.child_ids[i]);
    }
    count
}

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

    let t = &*tree;
    let mut temp_path = [0u64; 1000];
    let mut length: c_int = 0;
    let mut current_id = id;

    while length < 1000 {
        temp_path[length as usize] = current_id;
        length += 1;

        if current_id == t.root_id {
            break;
        }

        let node = tree_get_node(tree, current_id);
        if node.is_null() {
            return -1;
        }
        current_id = (*node).parent_id;
    }

    if length > max_length {
        length = max_length;
    }

    for i in 0..length as usize {
        *path.add(i) = temp_path[(length as usize) - 1 - i];
    }

    length
}
