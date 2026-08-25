use std::ffi::{c_char, c_int, c_void};
use std::ptr;

const HASHMAP_INITIAL_CAPACITY: usize = 16;
const HASHMAP_LOAD_FACTOR: f64 = 0.75;
const MAX_CHILDREN: usize = 32;
const MAX_DATA_LENGTH: usize = 256;

extern "C" {
    fn calloc(count: usize, size: usize) -> *mut c_void;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    fn free(pointer: *mut c_void);
    fn malloc(size: usize) -> *mut c_void;
    fn printf(format: *const c_char, ...) -> c_int;
    static mut stderr: *mut c_void;
}

#[repr(C)]
pub struct HashmapEntry {
    pub key: u64,
    pub value: *mut c_void,
    pub occupied: c_int,
    pub deleted: c_int,
}

#[repr(C)]
pub struct Hashmap {
    pub entries: *mut HashmapEntry,
    pub capacity: usize,
    pub size: usize,
    pub deleted_count: usize,
}

#[repr(C)]
pub struct TreeNode {
    pub id: u64,
    pub parent_id: u64,
    pub child_ids: [u64; MAX_CHILDREN],
    pub child_count: c_int,
    pub data: [c_char; MAX_DATA_LENGTH],
}

#[repr(C)]
pub struct Tree {
    pub node_map: *mut Hashmap,
    pub root_id: u64,
    pub has_root: c_int,
    pub node_count: usize,
}

fn hash_function(key: u64) -> u64 {
    let mut hash = 14_695_981_039_346_656_037_u64;
    for byte in key.to_ne_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash
}

unsafe fn should_resize(map: *mut Hashmap) -> bool {
    let map = &*map;
    (map.size + map.deleted_count) as f64 / map.capacity as f64 > HASHMAP_LOAD_FACTOR
}

unsafe fn hashmap_resize(map: *mut Hashmap) -> c_int {
    let old_capacity = (*map).capacity;
    let old_entries = (*map).entries;

    (*map).capacity *= 2;
    (*map).entries =
        calloc((*map).capacity, std::mem::size_of::<HashmapEntry>()).cast::<HashmapEntry>();
    if (*map).entries.is_null() {
        (*map).entries = old_entries;
        (*map).capacity = old_capacity;
        return -1;
    }

    (*map).size = 0;
    (*map).deleted_count = 0;
    for index in 0..old_capacity {
        let entry = &*old_entries.add(index);
        if entry.occupied != 0 && entry.deleted == 0 {
            hashmap_put(map, entry.key, entry.value);
        }
    }

    free(old_entries.cast::<c_void>());
    0
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_create() -> *mut Hashmap {
    let map = malloc(std::mem::size_of::<Hashmap>()).cast::<Hashmap>();
    if map.is_null() {
        return ptr::null_mut();
    }

    (*map).capacity = HASHMAP_INITIAL_CAPACITY;
    (*map).size = 0;
    (*map).deleted_count = 0;
    (*map).entries =
        calloc((*map).capacity, std::mem::size_of::<HashmapEntry>()).cast::<HashmapEntry>();

    if (*map).entries.is_null() {
        free(map.cast::<c_void>());
        return ptr::null_mut();
    }
    map
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_destroy(map: *mut Hashmap) {
    if !map.is_null() {
        free((*map).entries.cast::<c_void>());
        free(map.cast::<c_void>());
    }
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_put(map: *mut Hashmap, key: u64, value: *mut c_void) -> c_int {
    if map.is_null() {
        return -1;
    }

    if should_resize(map) && hashmap_resize(map) != 0 {
        return -1;
    }

    let index = hash_function(key) as usize % (*map).capacity;
    let mut probe = 0;
    while probe < (*map).capacity {
        let current = (index + probe) % (*map).capacity;
        let entry = &mut *(*map).entries.add(current);

        if entry.occupied == 0 {
            entry.key = key;
            entry.value = value;
            entry.occupied = 1;
            entry.deleted = 0;
            (*map).size += 1;
            return 0;
        } else if entry.deleted != 0 {
            entry.key = key;
            entry.value = value;
            entry.deleted = 0;
            (*map).size += 1;
            (*map).deleted_count -= 1;
            return 0;
        } else if entry.key == key {
            entry.value = value;
            return 0;
        }
        probe += 1;
    }
    -1
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_get(map: *mut Hashmap, key: u64) -> *mut c_void {
    if map.is_null() {
        return ptr::null_mut();
    }

    let index = hash_function(key) as usize % (*map).capacity;
    let mut probe = 0;
    while probe < (*map).capacity {
        let current = (index + probe) % (*map).capacity;
        let entry = &*(*map).entries.add(current);

        if entry.occupied == 0 {
            return ptr::null_mut();
        }
        if entry.deleted == 0 && entry.key == key {
            return entry.value;
        }
        probe += 1;
    }
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_remove(map: *mut Hashmap, key: u64) -> *mut c_void {
    if map.is_null() {
        return ptr::null_mut();
    }

    let index = hash_function(key) as usize % (*map).capacity;
    let mut probe = 0;
    while probe < (*map).capacity {
        let current = (index + probe) % (*map).capacity;
        let entry = &mut *(*map).entries.add(current);

        if entry.occupied == 0 {
            return ptr::null_mut();
        }
        if entry.deleted == 0 && entry.key == key {
            let value = entry.value;
            entry.deleted = 1;
            (*map).size -= 1;
            (*map).deleted_count += 1;
            return value;
        }
        probe += 1;
    }
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_contains(map: *mut Hashmap, key: u64) -> c_int {
    (!hashmap_get(map, key).is_null()) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_size(map: *mut Hashmap) -> usize {
    if map.is_null() {
        0
    } else {
        (*map).size
    }
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_clear(map: *mut Hashmap) {
    if map.is_null() {
        return;
    }

    for index in 0..(*map).capacity {
        let entry = &mut *(*map).entries.add(index);
        entry.occupied = 0;
        entry.deleted = 0;
    }
    (*map).size = 0;
    (*map).deleted_count = 0;
}

#[no_mangle]
pub unsafe extern "C" fn tree_create() -> *mut Tree {
    let tree = malloc(std::mem::size_of::<Tree>()).cast::<Tree>();
    if tree.is_null() {
        return ptr::null_mut();
    }

    (*tree).node_map = hashmap_create();
    if (*tree).node_map.is_null() {
        free(tree.cast::<c_void>());
        return ptr::null_mut();
    }

    (*tree).root_id = 0;
    (*tree).has_root = 0;
    (*tree).node_count = 0;
    tree
}

unsafe fn tree_free_node(node: *mut TreeNode) {
    if !node.is_null() {
        free(node.cast::<c_void>());
    }
}

#[no_mangle]
pub unsafe extern "C" fn tree_delete(tree: *mut Tree) {
    if tree.is_null() {
        return;
    }

    let map = (*tree).node_map;
    for index in 0..(*map).capacity {
        let entry = &*(*map).entries.add(index);
        if entry.occupied != 0 && entry.deleted == 0 {
            tree_free_node(entry.value.cast::<TreeNode>());
        }
    }
    hashmap_destroy(map);
    free(tree.cast::<c_void>());
}

unsafe fn copy_node_data(destination: *mut c_char, source: *const c_char) {
    if source.is_null() {
        *destination = 0;
        return;
    }

    let mut terminated = false;
    for index in 0..MAX_DATA_LENGTH - 1 {
        let byte = if terminated { 0 } else { *source.add(index) };
        *destination.add(index) = byte;
        terminated |= byte == 0;
    }
    *destination.add(MAX_DATA_LENGTH - 1) = 0;
}

#[no_mangle]
pub unsafe extern "C" fn tree_add_node(
    tree: *mut Tree,
    id: u64,
    parent_id: u64,
    data: *const c_char,
) -> c_int {
    if tree.is_null() {
        return -1;
    }
    if tree_contains(tree, id) != 0 {
        fprintf(
            stderr,
            b"Error: Node with ID %lu already exists\n\0"
                .as_ptr()
                .cast(),
            id,
        );
        return -1;
    }

    let node = malloc(std::mem::size_of::<TreeNode>()).cast::<TreeNode>();
    if node.is_null() {
        fprintf(
            stderr,
            b"Error: Failed to allocate node\n\0".as_ptr().cast(),
        );
        return -1;
    }

    (*node).id = id;
    (*node).parent_id = parent_id;
    (*node).child_count = 0;
    copy_node_data((*node).data.as_mut_ptr(), data);

    if (*tree).has_root == 0 {
        (*tree).root_id = id;
        (*tree).has_root = 1;
        (*node).parent_id = 0;
    } else {
        let parent = tree_get_node(tree, parent_id);
        if parent.is_null() {
            fprintf(
                stderr,
                b"Error: Parent node %lu not found\n\0".as_ptr().cast(),
                parent_id,
            );
            free(node.cast::<c_void>());
            return -1;
        }
        if (*parent).child_count >= MAX_CHILDREN as c_int {
            fprintf(
                stderr,
                b"Error: Parent has maximum children\n\0".as_ptr().cast(),
            );
            free(node.cast::<c_void>());
            return -1;
        }

        (*parent).child_ids[(*parent).child_count as usize] = id;
        (*parent).child_count += 1;
    }

    if hashmap_put((*tree).node_map, id, node.cast::<c_void>()) != 0 {
        fprintf(
            stderr,
            b"Error: Failed to add node to hashmap\n\0".as_ptr().cast(),
        );
        free(node.cast::<c_void>());
        return -1;
    }
    (*tree).node_count += 1;
    0
}

unsafe fn tree_remove_subtree(tree: *mut Tree, id: u64) -> c_int {
    let node = tree_get_node(tree, id);
    if node.is_null() {
        return -1;
    }

    for index in 0..(*node).child_count {
        tree_remove_subtree(tree, (*node).child_ids[index as usize]);
    }

    let removed = hashmap_remove((*tree).node_map, id).cast::<TreeNode>();
    if !removed.is_null() {
        tree_free_node(removed);
        (*tree).node_count -= 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn tree_remove_node(tree: *mut Tree, id: u64) -> c_int {
    if tree.is_null() {
        return -1;
    }

    let node = tree_get_node(tree, id);
    if node.is_null() {
        fprintf(stderr, b"Error: Node %lu not found\n\0".as_ptr().cast(), id);
        return -1;
    }

    if id == (*tree).root_id {
        tree_remove_subtree(tree, id);
        (*tree).has_root = 0;
        (*tree).root_id = 0;
        return 0;
    }

    let parent = tree_get_node(tree, (*node).parent_id);
    if !parent.is_null() {
        for index in 0..(*parent).child_count {
            if (*parent).child_ids[index as usize] == id {
                for shift in index..(*parent).child_count - 1 {
                    (*parent).child_ids[shift as usize] = (*parent).child_ids[(shift + 1) as usize];
                }
                (*parent).child_count -= 1;
                break;
            }
        }
    }

    tree_remove_subtree(tree, id);
    0
}

#[no_mangle]
pub unsafe extern "C" fn tree_get_node(tree: *mut Tree, id: u64) -> *mut TreeNode {
    if tree.is_null() {
        ptr::null_mut()
    } else {
        hashmap_get((*tree).node_map, id).cast::<TreeNode>()
    }
}

#[no_mangle]
pub unsafe extern "C" fn tree_contains(tree: *mut Tree, id: u64) -> c_int {
    (!tree_get_node(tree, id).is_null()) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn tree_size(tree: *mut Tree) -> usize {
    if tree.is_null() {
        0
    } else {
        (*tree).node_count
    }
}

unsafe fn tree_print_helper(tree: *mut Tree, id: u64, depth: c_int) {
    let node = tree_get_node(tree, id);
    if node.is_null() {
        return;
    }

    for _ in 0..depth {
        printf(b"  \0".as_ptr().cast());
    }
    printf(
        b"[%lu] %s\n\0".as_ptr().cast(),
        (*node).id,
        (*node).data.as_ptr(),
    );

    for index in 0..(*node).child_count {
        tree_print_helper(tree, (*node).child_ids[index as usize], depth + 1);
    }
}

#[no_mangle]
pub unsafe extern "C" fn tree_print(tree: *mut Tree) {
    if tree.is_null() || (*tree).has_root == 0 {
        printf(b"(empty tree)\n\0".as_ptr().cast());
        return;
    }
    tree_print_helper(tree, (*tree).root_id, 0);
}

#[no_mangle]
pub unsafe extern "C" fn tree_get_depth(tree: *mut Tree, id: u64) -> c_int {
    if tree.is_null() || tree_contains(tree, id) == 0 {
        return -1;
    }

    let mut depth = 0;
    let mut current_id = id;
    while current_id != (*tree).root_id {
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
pub unsafe extern "C" fn tree_get_height(tree: *mut Tree, id: u64) -> c_int {
    let node = tree_get_node(tree, id);
    if node.is_null() {
        return -1;
    }
    if (*node).child_count == 0 {
        return 0;
    }

    let mut max_height = 0;
    for index in 0..(*node).child_count {
        let child_height = tree_get_height(tree, (*node).child_ids[index as usize]);
        if child_height > max_height {
            max_height = child_height;
        }
    }
    max_height + 1
}

#[no_mangle]
pub unsafe extern "C" fn tree_count_descendants(tree: *mut Tree, id: u64) -> c_int {
    let node = tree_get_node(tree, id);
    if node.is_null() {
        return -1;
    }

    let mut count = 0;
    for index in 0..(*node).child_count {
        count += 1;
        count += tree_count_descendants(tree, (*node).child_ids[index as usize]);
    }
    count
}

#[no_mangle]
pub unsafe extern "C" fn tree_find_path(
    tree: *mut Tree,
    id: u64,
    path: *mut u64,
    max_length: c_int,
) -> c_int {
    if tree.is_null() || path.is_null() || tree_contains(tree, id) == 0 {
        return -1;
    }

    let mut temp_path = [0_u64; 1000];
    let mut length = 0;
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

    if length > max_length {
        length = max_length;
    }
    for index in 0..length {
        *path.add(index as usize) = temp_path[(length - 1 - index) as usize];
    }
    length
}
