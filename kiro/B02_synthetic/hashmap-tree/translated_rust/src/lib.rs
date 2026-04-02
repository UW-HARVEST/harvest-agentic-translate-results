// ============================================================
// Hashmap (open-addressing, FNV-1a, linear probing)
// ============================================================

const HASHMAP_INITIAL_CAPACITY: usize = 16;
const HASHMAP_LOAD_FACTOR: f64 = 0.75;

pub type TreeId = u64;

#[derive(Clone)]
struct HashmapEntry {
    key: TreeId,
    value: usize,
    occupied: bool,
    deleted: bool,
}

impl Default for HashmapEntry {
    fn default() -> Self {
        Self { key: 0, value: 0, occupied: false, deleted: false }
    }
}

pub struct Hashmap {
    entries: Vec<HashmapEntry>,
    capacity: usize,
    size: usize,
    deleted_count: usize,
}

fn hash_function(key: TreeId) -> u64 {
    let bytes = key.to_ne_bytes();
    let mut hash: u64 = 14695981039346656037;
    for &b in &bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

impl Hashmap {
    pub fn new() -> Self {
        Self {
            entries: vec![HashmapEntry::default(); HASHMAP_INITIAL_CAPACITY],
            capacity: HASHMAP_INITIAL_CAPACITY,
            size: 0,
            deleted_count: 0,
        }
    }

    fn should_resize(&self) -> bool {
        let load = (self.size + self.deleted_count) as f64 / self.capacity as f64;
        load > HASHMAP_LOAD_FACTOR
    }

    fn resize(&mut self) {
        let old_entries = std::mem::take(&mut self.entries);
        let old_capacity = self.capacity;
        self.capacity *= 2;
        self.entries = vec![HashmapEntry::default(); self.capacity];
        self.size = 0;
        self.deleted_count = 0;
        for i in 0..old_capacity {
            if old_entries[i].occupied && !old_entries[i].deleted {
                self.put(old_entries[i].key, old_entries[i].value);
            }
        }
    }

    pub fn put(&mut self, key: TreeId, value: usize) -> i32 {
        if self.should_resize() {
            self.resize();
        }
        let hash = hash_function(key);
        let index = (hash as usize) % self.capacity;
        for probe in 0..self.capacity {
            let current = (index + probe) % self.capacity;
            if !self.entries[current].occupied {
                self.entries[current].key = key;
                self.entries[current].value = value;
                self.entries[current].occupied = true;
                self.entries[current].deleted = false;
                self.size += 1;
                return 0;
            } else if self.entries[current].deleted {
                self.entries[current].key = key;
                self.entries[current].value = value;
                self.entries[current].deleted = false;
                self.size += 1;
                self.deleted_count -= 1;
                return 0;
            } else if self.entries[current].key == key {
                self.entries[current].value = value;
                return 0;
            }
        }
        -1
    }

    pub fn get(&self, key: TreeId) -> Option<usize> {
        let hash = hash_function(key);
        let index = (hash as usize) % self.capacity;
        for probe in 0..self.capacity {
            let current = (index + probe) % self.capacity;
            if !self.entries[current].occupied {
                return None;
            }
            if !self.entries[current].deleted && self.entries[current].key == key {
                return Some(self.entries[current].value);
            }
        }
        None
    }

    pub fn remove(&mut self, key: TreeId) -> Option<usize> {
        let hash = hash_function(key);
        let index = (hash as usize) % self.capacity;
        for probe in 0..self.capacity {
            let current = (index + probe) % self.capacity;
            if !self.entries[current].occupied {
                return None;
            }
            if !self.entries[current].deleted && self.entries[current].key == key {
                let value = self.entries[current].value;
                self.entries[current].deleted = true;
                self.size -= 1;
                self.deleted_count += 1;
                return Some(value);
            }
        }
        None
    }

    pub fn contains(&self, key: TreeId) -> bool {
        self.get(key).is_some()
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn clear(&mut self) {
        for i in 0..self.capacity {
            self.entries[i].occupied = false;
            self.entries[i].deleted = false;
        }
        self.size = 0;
        self.deleted_count = 0;
    }
}

// ============================================================
// Tree
// ============================================================

pub const MAX_CHILDREN: usize = 32;
pub const MAX_DATA_LENGTH: usize = 256;

pub struct TreeNode {
    pub id: TreeId,
    pub parent_id: TreeId,
    pub child_ids: [TreeId; MAX_CHILDREN],
    pub child_count: i32,
    pub data: String,
}

pub struct Tree {
    node_map: Hashmap,
    nodes: Vec<Option<TreeNode>>,
    pub root_id: TreeId,
    pub has_root: bool,
    pub node_count: usize,
    free_slots: Vec<usize>,
}

impl Tree {
    pub fn new() -> Self {
        Self {
            node_map: Hashmap::new(),
            nodes: Vec::new(),
            root_id: 0,
            has_root: false,
            node_count: 0,
            free_slots: Vec::new(),
        }
    }

    fn alloc_node(&mut self, node: TreeNode) -> usize {
        if let Some(idx) = self.free_slots.pop() {
            self.nodes[idx] = Some(node);
            idx
        } else {
            let idx = self.nodes.len();
            self.nodes.push(Some(node));
            idx
        }
    }

    fn free_node(&mut self, idx: usize) {
        self.nodes[idx] = None;
        self.free_slots.push(idx);
    }

    pub fn get_node(&self, id: TreeId) -> Option<&TreeNode> {
        self.node_map.get(id).and_then(|idx| self.nodes[idx].as_ref())
    }

    pub fn get_node_mut(&mut self, id: TreeId) -> Option<&mut TreeNode> {
        self.node_map.get(id).and_then(|idx| self.nodes[idx].as_mut())
    }

    pub fn contains(&self, id: TreeId) -> bool {
        self.node_map.contains(id)
    }

    pub fn size(&self) -> usize {
        self.node_count
    }

    pub fn add_node(&mut self, id: TreeId, parent_id: TreeId, data: &str) -> i32 {
        if self.contains(id) {
            eprint!("Error: Node with ID {} already exists\n", id);
            return -1;
        }

        let mut node = TreeNode {
            id,
            parent_id,
            child_ids: [0; MAX_CHILDREN],
            child_count: 0,
            data: String::new(),
        };

        let truncated: String = data.chars().take(MAX_DATA_LENGTH - 1).collect();
        node.data = truncated;

        if !self.has_root {
            self.root_id = id;
            self.has_root = true;
            node.parent_id = 0;
        } else {
            let parent_child_count = match self.get_node(parent_id) {
                Some(p) => p.child_count,
                None => {
                    eprint!("Error: Parent node {} not found\n", parent_id);
                    return -1;
                }
            };
            if parent_child_count >= MAX_CHILDREN as i32 {
                eprint!("Error: Parent has maximum children\n");
                return -1;
            }
            let parent = self.get_node_mut(parent_id).unwrap();
            parent.child_ids[parent.child_count as usize] = id;
            parent.child_count += 1;
        }

        let idx = self.alloc_node(node);
        if self.node_map.put(id, idx) != 0 {
            eprint!("Error: Failed to add node to hashmap\n");
            self.free_node(idx);
            return -1;
        }

        self.node_count += 1;
        0
    }

    pub fn remove_subtree(&mut self, id: TreeId) {
        let child_ids: Vec<TreeId> = match self.get_node(id) {
            Some(node) => {
                let mut v = Vec::new();
                for i in 0..node.child_count as usize {
                    v.push(node.child_ids[i]);
                }
                v
            }
            None => return,
        };

        for cid in child_ids {
            self.remove_subtree(cid);
        }

        if let Some(idx) = self.node_map.remove(id) {
            self.free_node(idx);
            self.node_count -= 1;
        }
    }

    pub fn remove_node(&mut self, id: TreeId) -> i32 {
        if self.get_node(id).is_none() {
            eprint!("Error: Node {} not found\n", id);
            return -1;
        }

        if id == self.root_id {
            self.remove_subtree(id);
            self.has_root = false;
            self.root_id = 0;
            return 0;
        }

        let parent_id = self.get_node(id).unwrap().parent_id;

        if let Some(parent) = self.get_node_mut(parent_id) {
            let mut i = 0;
            while i < parent.child_count as usize {
                if parent.child_ids[i] == id {
                    let mut j = i;
                    while j < (parent.child_count - 1) as usize {
                        parent.child_ids[j] = parent.child_ids[j + 1];
                        j += 1;
                    }
                    parent.child_count -= 1;
                    break;
                }
                i += 1;
            }
        }

        self.remove_subtree(id);
        0
    }

    pub fn print_helper(&self, id: TreeId, depth: i32) {
        let node = match self.get_node(id) {
            Some(n) => n,
            None => return,
        };

        for _ in 0..depth {
            print!("  ");
        }
        print!("[{}] {}\n", node.id, node.data);

        for i in 0..node.child_count as usize {
            self.print_helper(node.child_ids[i], depth + 1);
        }
    }

    pub fn print(&self) {
        if !self.has_root {
            print!("(empty tree)\n");
            return;
        }
        self.print_helper(self.root_id, 0);
    }

    pub fn get_depth(&self, id: TreeId) -> i32 {
        if !self.contains(id) {
            return -1;
        }
        let mut depth = 0;
        let mut current_id = id;
        while current_id != self.root_id {
            let node = match self.get_node(current_id) {
                Some(n) => n,
                None => return -1,
            };
            current_id = node.parent_id;
            depth += 1;
        }
        depth
    }

    pub fn get_height(&self, id: TreeId) -> i32 {
        let node = match self.get_node(id) {
            Some(n) => n,
            None => return -1,
        };
        if node.child_count == 0 {
            return 0;
        }
        let mut max_height = 0;
        for i in 0..node.child_count as usize {
            let ch = self.get_height(node.child_ids[i]);
            if ch > max_height {
                max_height = ch;
            }
        }
        max_height + 1
    }

    pub fn count_descendants(&self, id: TreeId) -> i32 {
        let node = match self.get_node(id) {
            Some(n) => n,
            None => return -1,
        };
        let mut count = 0;
        for i in 0..node.child_count as usize {
            count += 1;
            count += self.count_descendants(node.child_ids[i]);
        }
        count
    }

    pub fn find_path(&self, id: TreeId, path: &mut [TreeId], max_length: usize) -> i32 {
        if !self.contains(id) {
            return -1;
        }
        let mut temp_path = [0u64; 1000];
        let mut length = 0usize;
        let mut current_id = id;

        while length < 1000 {
            temp_path[length] = current_id;
            length += 1;
            if current_id == self.root_id {
                break;
            }
            let node = match self.get_node(current_id) {
                Some(n) => n,
                None => return -1,
            };
            current_id = node.parent_id;
        }

        if length > max_length {
            length = max_length;
        }

        for i in 0..length {
            path[i] = temp_path[length - 1 - i];
        }

        length as i32
    }
}

// ============================================================
// FFI exports matching C API
// ============================================================

use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_void};

// --- Hashmap FFI ---

/// C-compatible hashmap that stores void* values (like the C version)
struct CHashmap {
    inner: Hashmap,
    values: Vec<*mut c_void>, // parallel storage for void* values
}

#[no_mangle]
pub extern "C" fn hashmap_create() -> *mut CHashmap {
    let hm = Box::new(CHashmap {
        inner: Hashmap::new(),
        values: Vec::new(),
    });
    Box::into_raw(hm)
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_destroy(map: *mut CHashmap) {
    if !map.is_null() {
        drop(Box::from_raw(map));
    }
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_put(map: *mut CHashmap, key: u64, value: *mut c_void) -> c_int {
    if map.is_null() {
        return -1;
    }
    let m = &mut *map;
    // Check if key already exists - update value in place
    if let Some(idx) = m.inner.get(key) {
        m.values[idx] = value;
        return 0;
    }
    let idx = m.values.len();
    m.values.push(value);
    m.inner.put(key, idx)
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_get(map: *mut CHashmap, key: u64) -> *mut c_void {
    if map.is_null() {
        return std::ptr::null_mut();
    }
    let m = &*map;
    match m.inner.get(key) {
        Some(idx) => m.values[idx],
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_remove(map: *mut CHashmap, key: u64) -> *mut c_void {
    if map.is_null() {
        return std::ptr::null_mut();
    }
    let m = &mut *map;
    match m.inner.remove(key) {
        Some(idx) => m.values[idx],
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_contains(map: *mut CHashmap, key: u64) -> c_int {
    if map.is_null() {
        return 0;
    }
    if (*map).inner.contains(key) { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_size(map: *mut CHashmap) -> usize {
    if map.is_null() {
        return 0;
    }
    (*map).inner.size()
}

#[no_mangle]
pub unsafe extern "C" fn hashmap_clear(map: *mut CHashmap) {
    if map.is_null() {
        return;
    }
    (*map).inner.clear();
}

// --- Tree FFI ---

// C-compatible tree node layout matching the C struct
#[repr(C)]
pub struct CTreeNode {
    pub id: u64,
    pub parent_id: u64,
    pub child_ids: [u64; 32],
    pub child_count: c_int,
    pub data: [u8; 256],
}

struct CTree {
    inner: Tree,
    // Map from tree_id -> Box<CTreeNode> for FFI returns
    c_nodes: std::collections::HashMap<u64, Box<CTreeNode>>,
}

fn make_c_node(node: &TreeNode) -> CTreeNode {
    let mut cn = CTreeNode {
        id: node.id,
        parent_id: node.parent_id,
        child_ids: node.child_ids,
        child_count: node.child_count,
        data: [0u8; 256],
    };
    let bytes = node.data.as_bytes();
    let len = bytes.len().min(255);
    cn.data[..len].copy_from_slice(&bytes[..len]);
    cn
}

#[no_mangle]
pub extern "C" fn tree_create() -> *mut CTree {
    Box::into_raw(Box::new(CTree {
        inner: Tree::new(),
        c_nodes: std::collections::HashMap::new(),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn tree_delete(tree: *mut CTree) {
    if !tree.is_null() {
        drop(Box::from_raw(tree));
    }
}

#[no_mangle]
pub unsafe extern "C" fn tree_add_node(
    tree: *mut CTree,
    id: u64,
    parent_id: u64,
    data: *const c_char,
) -> c_int {
    if tree.is_null() {
        return -1;
    }
    let t = &mut *tree;
    let data_str = if data.is_null() {
        ""
    } else {
        CStr::from_ptr(data).to_str().unwrap_or("")
    };
    t.inner.add_node(id, parent_id, data_str)
}

#[no_mangle]
pub unsafe extern "C" fn tree_remove_node(tree: *mut CTree, id: u64) -> c_int {
    if tree.is_null() {
        return -1;
    }
    let t = &mut *tree;
    t.c_nodes.remove(&id);
    t.inner.remove_node(id)
}

#[no_mangle]
pub unsafe extern "C" fn tree_get_node(tree: *mut CTree, id: u64) -> *mut CTreeNode {
    if tree.is_null() {
        return std::ptr::null_mut();
    }
    let t = &mut *tree;
    match t.inner.get_node(id) {
        Some(node) => {
            let cn = Box::new(make_c_node(node));
            let ptr = &*cn as *const CTreeNode as *mut CTreeNode;
            t.c_nodes.insert(id, cn);
            ptr
        }
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn tree_contains(tree: *mut CTree, id: u64) -> c_int {
    if tree.is_null() {
        return 0;
    }
    if (*tree).inner.contains(id) { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn tree_size(tree: *mut CTree) -> usize {
    if tree.is_null() {
        return 0;
    }
    (*tree).inner.size()
}

#[no_mangle]
pub unsafe extern "C" fn tree_print(tree: *mut CTree) {
    if tree.is_null() {
        return;
    }
    (*tree).inner.print();
}

#[no_mangle]
pub unsafe extern "C" fn tree_get_depth(tree: *mut CTree, id: u64) -> c_int {
    if tree.is_null() {
        return -1;
    }
    (*tree).inner.get_depth(id)
}

#[no_mangle]
pub unsafe extern "C" fn tree_get_height(tree: *mut CTree, id: u64) -> c_int {
    if tree.is_null() {
        return -1;
    }
    (*tree).inner.get_height(id)
}

#[no_mangle]
pub unsafe extern "C" fn tree_count_descendants(tree: *mut CTree, id: u64) -> c_int {
    if tree.is_null() {
        return -1;
    }
    (*tree).inner.count_descendants(id)
}

#[no_mangle]
pub unsafe extern "C" fn tree_find_path(
    tree: *mut CTree,
    id: u64,
    path: *mut u64,
    max_length: c_int,
) -> c_int {
    if tree.is_null() || path.is_null() {
        return -1;
    }
    let t = &*tree;
    let slice = std::slice::from_raw_parts_mut(path, max_length as usize);
    t.inner.find_path(id, slice, max_length as usize)
}
