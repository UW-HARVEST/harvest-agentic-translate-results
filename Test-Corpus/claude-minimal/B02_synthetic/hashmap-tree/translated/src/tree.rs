// tree.rs - Rust translation of tree.c/.h
use crate::hashmap::{
    hashmap_create, hashmap_destroy, hashmap_get, hashmap_put, hashmap_remove, Hashmap, TreeId,
};
use std::ffi::c_void;

pub const MAX_CHILDREN: usize = 32;
pub const MAX_DATA_LENGTH: usize = 256;

#[repr(C)]
pub struct TreeNode {
    pub id: TreeId,
    pub parent_id: TreeId,
    pub child_ids: [TreeId; MAX_CHILDREN],
    pub child_count: i32,
    pub data: [u8; MAX_DATA_LENGTH],
}

impl TreeNode {
    pub fn new() -> Self {
        TreeNode {
            id: 0,
            parent_id: 0,
            child_ids: [0; MAX_CHILDREN],
            child_count: 0,
            data: [0u8; MAX_DATA_LENGTH],
        }
    }

    pub fn data_str(&self) -> String {
        let end = self.data.iter().position(|&b| b == 0).unwrap_or(MAX_DATA_LENGTH);
        String::from_utf8_lossy(&self.data[..end]).into_owned()
    }
}

pub struct Tree {
    pub node_map: Box<Hashmap>,
    pub root_id: TreeId,
    pub has_root: bool,
    pub node_count: usize,
}

pub fn tree_create() -> Box<Tree> {
    Box::new(Tree {
        node_map: hashmap_create(),
        root_id: 0,
        has_root: false,
        node_count: 0,
    })
}

fn tree_free_node(node_ptr: *mut TreeNode) {
    if !node_ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(node_ptr);
        }
    }
}

pub fn tree_delete(tree: Box<Tree>) {
    let mut tree = tree;
    // Free all nodes in the hashmap
    let cap = tree.node_map.capacity;
    for i in 0..cap {
        let entry = tree.node_map.entries[i];
        if entry.occupied && !entry.deleted {
            tree_free_node(entry.value as *mut TreeNode);
        }
    }
    let map = std::mem::replace(&mut tree.node_map, hashmap_create());
    hashmap_destroy(map);
    drop(tree);
}

pub fn tree_add_node(
    tree: &mut Tree,
    id: TreeId,
    parent_id: TreeId,
    data: Option<&str>,
) -> i32 {
    // Check if node already exists
    if tree_contains(tree, id) {
        eprintln!("Error: Node with ID {} already exists", id);
        return -1;
    }

    // Allocate new node
    let mut node = Box::new(TreeNode::new());
    node.id = id;
    node.parent_id = parent_id;
    node.child_count = 0;

    if let Some(s) = data {
        let bytes = s.as_bytes();
        let copy_len = std::cmp::min(bytes.len(), MAX_DATA_LENGTH - 1);
        node.data[..copy_len].copy_from_slice(&bytes[..copy_len]);
        node.data[copy_len] = 0;
    } else {
        node.data[0] = 0;
    }

    if !tree.has_root {
        tree.root_id = id;
        tree.has_root = true;
        node.parent_id = 0;
    } else {
        let parent_ptr = hashmap_get(&tree.node_map, parent_id) as *mut TreeNode;
        if parent_ptr.is_null() {
            eprintln!("Error: Parent node {} not found", parent_id);
            // node will be dropped automatically
            return -1;
        }
        let parent = unsafe { &mut *parent_ptr };
        if parent.child_count >= MAX_CHILDREN as i32 {
            eprintln!("Error: Parent has maximum children");
            return -1;
        }
        parent.child_ids[parent.child_count as usize] = id;
        parent.child_count += 1;
    }

    let node_ptr = Box::into_raw(node);
    if hashmap_put(&mut tree.node_map, id, node_ptr as *mut c_void) != 0 {
        eprintln!("Error: Failed to add node to hashmap");
        // Free the node since we won't be tracking it
        unsafe {
            let _ = Box::from_raw(node_ptr);
        }
        return -1;
    }
    tree.node_count += 1;
    0
}

fn tree_remove_subtree(tree: &mut Tree, id: TreeId) -> i32 {
    let node_ptr = hashmap_get(&tree.node_map, id) as *mut TreeNode;
    if node_ptr.is_null() {
        return -1;
    }
    let (child_count, child_ids) = unsafe {
        let node = &*node_ptr;
        (node.child_count, node.child_ids)
    };

    // Recursively remove all children first
    for i in 0..child_count as usize {
        tree_remove_subtree(tree, child_ids[i]);
    }

    // Remove this node from hashmap
    let removed = hashmap_remove(&mut tree.node_map, id) as *mut TreeNode;
    if !removed.is_null() {
        tree_free_node(removed);
        tree.node_count -= 1;
    }
    0
}

pub fn tree_remove_node(tree: &mut Tree, id: TreeId) -> i32 {
    let node_ptr = hashmap_get(&tree.node_map, id) as *mut TreeNode;
    if node_ptr.is_null() {
        eprintln!("Error: Node {} not found", id);
        return -1;
    }

    if id == tree.root_id {
        tree_remove_subtree(tree, id);
        tree.has_root = false;
        tree.root_id = 0;
        return 0;
    }

    let parent_id = unsafe { (*node_ptr).parent_id };
    let parent_ptr = hashmap_get(&tree.node_map, parent_id) as *mut TreeNode;
    if !parent_ptr.is_null() {
        unsafe {
            let parent = &mut *parent_ptr;
            for i in 0..parent.child_count as usize {
                if parent.child_ids[i] == id {
                    let n = parent.child_count as usize;
                    for j in i..n - 1 {
                        parent.child_ids[j] = parent.child_ids[j + 1];
                    }
                    parent.child_count -= 1;
                    break;
                }
            }
        }
    }

    tree_remove_subtree(tree, id);
    0
}

pub fn tree_get_node(tree: &Tree, id: TreeId) -> *mut TreeNode {
    hashmap_get(&tree.node_map, id) as *mut TreeNode
}

pub fn tree_contains(tree: &Tree, id: TreeId) -> bool {
    !tree_get_node(tree, id).is_null()
}

pub fn tree_size(tree: &Tree) -> usize {
    tree.node_count
}

fn tree_print_helper(tree: &Tree, id: TreeId, depth: i32) {
    let node_ptr = tree_get_node(tree, id);
    if node_ptr.is_null() {
        return;
    }
    let node = unsafe { &*node_ptr };

    for _ in 0..depth {
        print!("  ");
    }
    println!("[{}] {}", node.id, node.data_str());

    for i in 0..node.child_count as usize {
        tree_print_helper(tree, node.child_ids[i], depth + 1);
    }
}

pub fn tree_print(tree: &Tree) {
    if !tree.has_root {
        println!("(empty tree)");
        return;
    }
    tree_print_helper(tree, tree.root_id, 0);
}

pub fn tree_get_depth(tree: &Tree, id: TreeId) -> i32 {
    if !tree_contains(tree, id) {
        return -1;
    }
    let mut depth = 0;
    let mut current_id = id;
    while current_id != tree.root_id {
        let node_ptr = tree_get_node(tree, current_id);
        if node_ptr.is_null() {
            return -1;
        }
        current_id = unsafe { (*node_ptr).parent_id };
        depth += 1;
    }
    depth
}

pub fn tree_get_height(tree: &Tree, id: TreeId) -> i32 {
    let node_ptr = tree_get_node(tree, id);
    if node_ptr.is_null() {
        return -1;
    }
    let (child_count, child_ids) = unsafe {
        let node = &*node_ptr;
        (node.child_count, node.child_ids)
    };
    if child_count == 0 {
        return 0;
    }
    let mut max_height = 0;
    for i in 0..child_count as usize {
        let child_height = tree_get_height(tree, child_ids[i]);
        if child_height > max_height {
            max_height = child_height;
        }
    }
    max_height + 1
}

pub fn tree_count_descendants(tree: &Tree, id: TreeId) -> i32 {
    let node_ptr = tree_get_node(tree, id);
    if node_ptr.is_null() {
        return -1;
    }
    let (child_count, child_ids) = unsafe {
        let node = &*node_ptr;
        (node.child_count, node.child_ids)
    };
    let mut count = 0;
    for i in 0..child_count as usize {
        count += 1;
        count += tree_count_descendants(tree, child_ids[i]);
    }
    count
}

pub fn tree_find_path(
    tree: &Tree,
    id: TreeId,
    path: &mut [TreeId],
    max_length: i32,
) -> i32 {
    if !tree_contains(tree, id) {
        return -1;
    }
    let mut temp_path = [0u64; 1000];
    let mut length = 0usize;
    let mut current_id = id;

    while length < 1000 {
        temp_path[length] = current_id;
        length += 1;
        if current_id == tree.root_id {
            break;
        }
        let node_ptr = tree_get_node(tree, current_id);
        if node_ptr.is_null() {
            return -1;
        }
        current_id = unsafe { (*node_ptr).parent_id };
    }

    let mut out_len = length;
    if out_len > max_length as usize {
        out_len = max_length as usize;
    }

    for i in 0..out_len {
        path[i] = temp_path[length - 1 - i];
    }

    out_len as i32
}
