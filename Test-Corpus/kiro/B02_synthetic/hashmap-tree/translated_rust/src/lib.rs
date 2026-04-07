use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

const MAX_CHILDREN: usize = 32;
const MAX_DATA_LENGTH: usize = 256;
const HASHMAP_INITIAL_CAPACITY: usize = 16;
const HASHMAP_LOAD_FACTOR: f64 = 0.75;

type TreeIdT = u64;

#[repr(C)]
struct HashmapEntry {
    key: TreeIdT,
    value: *mut c_void,
    occupied: c_int,
    deleted: c_int,
}

#[repr(C)]
pub struct HashmapT {
    entries: *mut HashmapEntry,
    capacity: usize,
    size: usize,
    deleted_count: usize,
}

#[repr(C)]
pub struct TreeNodeT {
    id: TreeIdT,
    parent_id: TreeIdT,
    child_ids: [TreeIdT; MAX_CHILDREN],
    child_count: c_int,
    data: [u8; MAX_DATA_LENGTH],
}

#[repr(C)]
pub struct TreeT {
    node_map: *mut HashmapT,
    root_id: TreeIdT,
    has_root: c_int,
    node_count: usize,
}

fn hash_function(key: TreeIdT) -> u64 {
    let bytes = key.to_ne_bytes();
    let mut hash: u64 = 14695981039346656037;
    for &b in &bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

unsafe fn should_resize(map: *mut HashmapT) -> bool {
    let load = ((*map).size + (*map).deleted_count) as f64 / (*map).capacity as f64;
    load > HASHMAP_LOAD_FACTOR
}

unsafe fn hashmap_resize_internal(map: *mut HashmapT) -> c_int {
    let old_capacity = (*map).capacity;
    let old_entries = (*map).entries;

    (*map).capacity *= 2;
    let layout = std::alloc::Layout::from_size_align(
        (*map).capacity * std::mem::size_of::<HashmapEntry>(),
        std::mem::align_of::<HashmapEntry>(),
    ).unwrap();
    let new_ptr = std::alloc::alloc_zeroed(layout) as *mut HashmapEntry;
    if new_ptr.is_null() {
        (*map).entries = old_entries;
        (*map).capacity = old_capacity;
        return -1;
    }
    (*map).entries = new_ptr;
    (*map).size = 0;
    (*map).deleted_count = 0;

    for i in 0..old_capacity {
        let e = &*old_entries.add(i);
        if e.occupied != 0 && e.deleted == 0 {
            hashmap_put(map, e.key, e.value);
        }
    }

    let old_layout = std::alloc::Layout::from_size_align(
        old_capacity * std::mem::size_of::<HashmapEntry>(),
        std::mem::align_of::<HashmapEntry>(),
    ).unwrap();
    std::alloc::dealloc(old_entries as *mut u8, old_layout);
    0
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_create() -> *mut HashmapT {
    let map = Box::into_raw(Box::new(std::mem::zeroed::<HashmapT>()));
    (*map).capacity = HASHMAP_INITIAL_CAPACITY;
    (*map).size = 0;
    (*map).deleted_count = 0;
    let layout = std::alloc::Layout::from_size_align(
        HASHMAP_INITIAL_CAPACITY * std::mem::size_of::<HashmapEntry>(),
        std::mem::align_of::<HashmapEntry>(),
    ).unwrap();
    let entries = std::alloc::alloc_zeroed(layout) as *mut HashmapEntry;
    if entries.is_null() {
        drop(Box::from_raw(map));
        return ptr::null_mut();
    }
    (*map).entries = entries;
    map
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_destroy(map: *mut HashmapT) {
    if map.is_null() { return; }
    if !(*map).entries.is_null() {
        let layout = std::alloc::Layout::from_size_align(
            (*map).capacity * std::mem::size_of::<HashmapEntry>(),
            std::mem::align_of::<HashmapEntry>(),
        ).unwrap();
        std::alloc::dealloc((*map).entries as *mut u8, layout);
    }
    drop(Box::from_raw(map));
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_put(map: *mut HashmapT, key: TreeIdT, value: *mut c_void) -> c_int {
    if map.is_null() { return -1; }

    if should_resize(map) {
        if hashmap_resize_internal(map) != 0 {
            return -1;
        }
    }

    let hash = hash_function(key);
    let mut probe: usize = 0;
    while probe < (*map).capacity {
        let current = (hash as usize + probe) % (*map).capacity;
        let e = &mut *(*map).entries.add(current);
        if e.occupied == 0 {
            e.key = key;
            e.value = value;
            e.occupied = 1;
            e.deleted = 0;
            (*map).size += 1;
            return 0;
        } else if e.deleted != 0 {
            e.key = key;
            e.value = value;
            e.deleted = 0;
            (*map).size += 1;
            (*map).deleted_count -= 1;
            return 0;
        } else if e.key == key {
            e.value = value;
            return 0;
        }
        probe += 1;
    }
    -1
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_get(map: *mut HashmapT, key: TreeIdT) -> *mut c_void {
    if map.is_null() { return ptr::null_mut(); }
    let hash = hash_function(key);
    let mut probe: usize = 0;
    while probe < (*map).capacity {
        let current = (hash as usize + probe) % (*map).capacity;
        let e = &*(*map).entries.add(current);
        if e.occupied == 0 { return ptr::null_mut(); }
        if e.deleted == 0 && e.key == key { return e.value; }
        probe += 1;
    }
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_remove(map: *mut HashmapT, key: TreeIdT) -> *mut c_void {
    if map.is_null() { return ptr::null_mut(); }
    let hash = hash_function(key);
    let mut probe: usize = 0;
    while probe < (*map).capacity {
        let current = (hash as usize + probe) % (*map).capacity;
        let e = &mut *(*map).entries.add(current);
        if e.occupied == 0 { return ptr::null_mut(); }
        if e.deleted == 0 && e.key == key {
            let value = e.value;
            e.deleted = 1;
            (*map).size -= 1;
            (*map).deleted_count += 1;
            return value;
        }
        probe += 1;
    }
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_contains(map: *mut HashmapT, key: TreeIdT) -> c_int {
    if hashmap_get(map, key).is_null() { 0 } else { 1 }
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_size(map: *mut HashmapT) -> usize {
    if map.is_null() { 0 } else { (*map).size }
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_clear(map: *mut HashmapT) {
    if map.is_null() { return; }
    for i in 0..(*map).capacity {
        let e = &mut *(*map).entries.add(i);
        e.occupied = 0;
        e.deleted = 0;
    }
    (*map).size = 0;
    (*map).deleted_count = 0;
}

// ---- Tree functions ----

#[no_mangle]
pub unsafe extern "C" fn tree_create() -> *mut TreeT {
    let tree = Box::into_raw(Box::new(std::mem::zeroed::<TreeT>()));
    let node_map = hashmap_create();
    if node_map.is_null() {
        drop(Box::from_raw(tree));
        return ptr::null_mut();
    }
    (*tree).node_map = node_map;
    (*tree).root_id = 0;
    (*tree).has_root = 0;
    (*tree).node_count = 0;
    tree
}

#[no_mangle]
pub unsafe extern "C" fn tree_delete(tree: *mut TreeT) {
    if tree.is_null() { return; }
    let map = (*tree).node_map;
    for i in 0..(*map).capacity {
        let e = &*(*map).entries.add(i);
        if e.occupied != 0 && e.deleted == 0 {
            drop(Box::from_raw(e.value as *mut TreeNodeT));
        }
    }
    hashmap_destroy(map);
    drop(Box::from_raw(tree));
}

#[no_mangle]
pub unsafe extern "C" fn tree_add_node(
    tree: *mut TreeT, id: TreeIdT, parent_id: TreeIdT, data: *const c_char,
) -> c_int {
    if tree.is_null() { return -1; }

    if tree_contains(tree, id) != 0 {
        eprint!("Error: Node with ID {} already exists\n", id);
        return -1;
    }

    let node = Box::into_raw(Box::new(std::mem::zeroed::<TreeNodeT>()));
    (*node).id = id;
    (*node).parent_id = parent_id;
    (*node).child_count = 0;

    if !data.is_null() {
        let src = CStr::from_ptr(data).to_bytes();
        let len = src.len().min(MAX_DATA_LENGTH - 1);
        ptr::copy_nonoverlapping(src.as_ptr(), (*node).data.as_mut_ptr(), len);
        (*node).data[len] = 0;
    } else {
        (*node).data[0] = 0;
    }

    if (*tree).has_root == 0 {
        (*tree).root_id = id;
        (*tree).has_root = 1;
        (*node).parent_id = 0;
    } else {
        let parent = tree_get_node(tree, parent_id);
        if parent.is_null() {
            eprint!("Error: Parent node {} not found\n", parent_id);
            drop(Box::from_raw(node));
            return -1;
        }
        if (*parent).child_count >= MAX_CHILDREN as c_int {
            eprint!("Error: Parent has maximum children\n");
            drop(Box::from_raw(node));
            return -1;
        }
        (*parent).child_ids[(*parent).child_count as usize] = id;
        (*parent).child_count += 1;
    }

    if hashmap_put((*tree).node_map, id, node as *mut c_void) != 0 {
        eprint!("Error: Failed to add node to hashmap\n");
        drop(Box::from_raw(node));
        return -1;
    }

    (*tree).node_count += 1;
    0
}

unsafe fn tree_remove_subtree_internal(tree: *mut TreeT, id: TreeIdT) -> c_int {
    let node = tree_get_node(tree, id);
    if node.is_null() { return -1; }

    // Must copy child info before removing, since we'll free the node
    let child_count = (*node).child_count;
    let mut child_ids = [0u64; MAX_CHILDREN];
    for i in 0..child_count as usize {
        child_ids[i] = (*node).child_ids[i];
    }

    for i in 0..child_count as usize {
        tree_remove_subtree_internal(tree, child_ids[i]);
    }

    let removed = hashmap_remove((*tree).node_map, id);
    if !removed.is_null() {
        drop(Box::from_raw(removed as *mut TreeNodeT));
        (*tree).node_count -= 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn tree_remove_node(tree: *mut TreeT, id: TreeIdT) -> c_int {
    if tree.is_null() { return -1; }

    let node = tree_get_node(tree, id);
    if node.is_null() {
        eprint!("Error: Node {} not found\n", id);
        return -1;
    }

    if id == (*tree).root_id {
        tree_remove_subtree_internal(tree, id);
        (*tree).has_root = 0;
        (*tree).root_id = 0;
        return 0;
    }

    let parent_id = (*node).parent_id;
    let parent = tree_get_node(tree, parent_id);
    if !parent.is_null() {
        let mut found = false;
        for i in 0..(*parent).child_count as usize {
            if (*parent).child_ids[i] == id {
                for j in i..(*parent).child_count as usize - 1 {
                    (*parent).child_ids[j] = (*parent).child_ids[j + 1];
                }
                (*parent).child_count -= 1;
                found = true;
                break;
            }
        }
        let _ = found;
    }

    tree_remove_subtree_internal(tree, id);
    0
}

#[no_mangle]
pub unsafe extern "C" fn tree_get_node(tree: *mut TreeT, id: TreeIdT) -> *mut TreeNodeT {
    if tree.is_null() { return ptr::null_mut(); }
    hashmap_get((*tree).node_map, id) as *mut TreeNodeT
}

#[no_mangle]
pub unsafe extern "C" fn tree_contains(tree: *mut TreeT, id: TreeIdT) -> c_int {
    if tree_get_node(tree, id).is_null() { 0 } else { 1 }
}

#[no_mangle]
pub unsafe extern "C" fn tree_size(tree: *mut TreeT) -> usize {
    if tree.is_null() { 0 } else { (*tree).node_count }
}

unsafe fn tree_print_helper(tree: *mut TreeT, id: TreeIdT, depth: c_int) {
    let node = tree_get_node(tree, id);
    if node.is_null() { return; }
    for _ in 0..depth {
        print!("  ");
    }
    let data_str = CStr::from_ptr((*node).data.as_ptr() as *const c_char);
    print!("[{}] {}\n", (*node).id, data_str.to_str().unwrap_or(""));
    for i in 0..(*node).child_count as usize {
        tree_print_helper(tree, (*node).child_ids[i], depth + 1);
    }
}

#[no_mangle]
pub unsafe extern "C" fn tree_print(tree: *mut TreeT) {
    if tree.is_null() || (*tree).has_root == 0 {
        print!("(empty tree)\n");
        return;
    }
    tree_print_helper(tree, (*tree).root_id, 0);
}

#[no_mangle]
pub unsafe extern "C" fn tree_get_depth(tree: *mut TreeT, id: TreeIdT) -> c_int {
    if tree.is_null() || tree_contains(tree, id) == 0 { return -1; }
    let mut depth = 0;
    let mut current_id = id;
    while current_id != (*tree).root_id {
        let node = tree_get_node(tree, current_id);
        if node.is_null() { return -1; }
        current_id = (*node).parent_id;
        depth += 1;
    }
    depth
}

#[no_mangle]
pub unsafe extern "C" fn tree_get_height(tree: *mut TreeT, id: TreeIdT) -> c_int {
    let node = tree_get_node(tree, id);
    if node.is_null() { return -1; }
    if (*node).child_count == 0 { return 0; }
    let mut max_height = 0;
    for i in 0..(*node).child_count as usize {
        let h = tree_get_height(tree, (*node).child_ids[i]);
        if h > max_height { max_height = h; }
    }
    max_height + 1
}

#[no_mangle]
pub unsafe extern "C" fn tree_count_descendants(tree: *mut TreeT, id: TreeIdT) -> c_int {
    let node = tree_get_node(tree, id);
    if node.is_null() { return -1; }
    let mut count = 0;
    for i in 0..(*node).child_count as usize {
        count += 1;
        count += tree_count_descendants(tree, (*node).child_ids[i]);
    }
    count
}

#[no_mangle]
pub unsafe extern "C" fn tree_find_path(
    tree: *mut TreeT, id: TreeIdT, path: *mut TreeIdT, max_length: c_int,
) -> c_int {
    if tree.is_null() || path.is_null() || tree_contains(tree, id) == 0 {
        return -1;
    }
    let mut temp_path = [0u64; 1000];
    let mut length: usize = 0;
    let mut current_id = id;
    while length < 1000 {
        temp_path[length] = current_id;
        length += 1;
        if current_id == (*tree).root_id { break; }
        let node = tree_get_node(tree, current_id);
        if node.is_null() { return -1; }
        current_id = (*node).parent_id;
    }
    let out_len = length.min(max_length as usize);
    for i in 0..out_len {
        *path.add(i) = temp_path[length - 1 - i];
    }
    out_len as c_int
}
