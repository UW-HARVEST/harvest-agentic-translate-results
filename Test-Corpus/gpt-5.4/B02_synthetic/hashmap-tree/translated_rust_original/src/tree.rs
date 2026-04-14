use crate::hashmap::{hashmap_contains, hashmap_create, hashmap_get, hashmap_put, hashmap_remove, hashmap_size, tree_id_t, hashmap_t};
use std::ffi::c_void;

pub const MAX_CHILDREN: usize = 32;
pub const MAX_DATA_LENGTH: usize = 256;

#[repr(C)]
pub struct tree_node_t {
    pub id: tree_id_t,
    pub parent_id: tree_id_t,
    pub child_ids: [tree_id_t; MAX_CHILDREN],
    pub child_count: i32,
    pub data: [u8; MAX_DATA_LENGTH],
}

pub struct tree_t {
    pub node_map: Box<hashmap_t>,
    pub root_id: tree_id_t,
    pub has_root: i32,
    pub node_count: usize,
}

fn write_data(buf: &mut [u8; MAX_DATA_LENGTH], data: Option<&str>) {
    buf.fill(0);
    if let Some(s) = data {
        let bytes = s.as_bytes();
        let len = bytes.len().min(MAX_DATA_LENGTH - 1);
        buf[..len].copy_from_slice(&bytes[..len]);
    }
}

pub fn node_data_as_str(node: &tree_node_t) -> &str {
    let len = node.data.iter().position(|&b| b == 0).unwrap_or(MAX_DATA_LENGTH);
    std::str::from_utf8(&node.data[..len]).unwrap_or("")
}

pub fn tree_create() -> Box<tree_t> {
    Box::new(tree_t {
        node_map: hashmap_create(),
        root_id: 0,
        has_root: 0,
        node_count: 0,
    })
}

pub fn tree_delete(tree: &mut tree_t) {
    let keys: Vec<tree_id_t> = tree.node_map.map.keys().copied().collect();
    for key in keys {
        let ptr = hashmap_remove(&mut tree.node_map, key);
        if !ptr.is_null() {
            unsafe {
                drop(Box::from_raw(ptr as *mut tree_node_t));
            }
        }
    }
    tree.has_root = 0;
    tree.root_id = 0;
    tree.node_count = 0;
}

pub fn tree_add_node(tree: &mut tree_t, id: tree_id_t, parent_id: tree_id_t, data: Option<&str>) -> i32 {
    if hashmap_contains(&tree.node_map, id) != 0 {
        eprintln!("Error: Node with ID {} already exists", id);
        return -1;
    }

    let mut node = Box::new(tree_node_t {
        id,
        parent_id,
        child_ids: [0; MAX_CHILDREN],
        child_count: 0,
        data: [0; MAX_DATA_LENGTH],
    });
    write_data(&mut node.data, data);

    if tree.has_root == 0 {
        tree.root_id = id;
        tree.has_root = 1;
        node.parent_id = 0;
    } else {
        let parent = tree_get_node(tree, parent_id);
        if parent.is_null() {
            eprintln!("Error: Parent node {} not found", parent_id);
            return -1;
        }
        let parent_ref = unsafe { &mut *parent };
        if parent_ref.child_count as usize >= MAX_CHILDREN {
            eprintln!("Error: Parent has maximum children");
            return -1;
        }
        parent_ref.child_ids[parent_ref.child_count as usize] = id;
        parent_ref.child_count += 1;
    }

    let raw = Box::into_raw(node) as *mut c_void;
    if hashmap_put(&mut tree.node_map, id, raw) != 0 {
        unsafe {
            drop(Box::from_raw(raw as *mut tree_node_t));
        }
        eprintln!("Error: Failed to add node to hashmap");
        return -1;
    }

    tree.node_count += 1;
    0
}

fn tree_remove_subtree(tree: &mut tree_t, id: tree_id_t) -> i32 {
    let node_ptr = tree_get_node(tree, id);
    if node_ptr.is_null() {
        return -1;
    }
    let child_ids = unsafe {
        let node = &*node_ptr;
        node.child_ids[..node.child_count as usize].to_vec()
    };
    for child_id in child_ids {
        tree_remove_subtree(tree, child_id);
    }
    let removed = hashmap_remove(&mut tree.node_map, id);
    if !removed.is_null() {
        unsafe {
            drop(Box::from_raw(removed as *mut tree_node_t));
        }
        tree.node_count -= 1;
    }
    0
}

pub fn tree_remove_node(tree: &mut tree_t, id: tree_id_t) -> i32 {
    let node_ptr = tree_get_node(tree, id);
    if node_ptr.is_null() {
        eprintln!("Error: Node {} not found", id);
        return -1;
    }

    if id == tree.root_id {
        tree_remove_subtree(tree, id);
        tree.has_root = 0;
        tree.root_id = 0;
        return 0;
    }

    let parent_id = unsafe { (*node_ptr).parent_id };
    let parent_ptr = tree_get_node(tree, parent_id);
    if !parent_ptr.is_null() {
        let parent = unsafe { &mut *parent_ptr };
        let count = parent.child_count as usize;
        if let Some(pos) = parent.child_ids[..count].iter().position(|&child| child == id) {
            for i in pos..count - 1 {
                parent.child_ids[i] = parent.child_ids[i + 1];
            }
            parent.child_count -= 1;
        }
    }

    tree_remove_subtree(tree, id)
}

pub fn tree_get_node(tree: &tree_t, id: tree_id_t) -> *mut tree_node_t {
    hashmap_get(&tree.node_map, id) as *mut tree_node_t
}

pub fn tree_contains(tree: &tree_t, id: tree_id_t) -> i32 {
    if tree_get_node(tree, id).is_null() { 0 } else { 1 }
}

pub fn tree_size(tree: &tree_t) -> usize {
    tree.node_count
}

fn tree_print_helper(tree: &tree_t, id: tree_id_t, depth: i32) {
    let node_ptr = tree_get_node(tree, id);
    if node_ptr.is_null() {
        return;
    }
    let node = unsafe { &*node_ptr };
    for _ in 0..depth {
        print!("  ");
    }
    println!("[{}] {}", node.id, node_data_as_str(node));
    for &child_id in &node.child_ids[..node.child_count as usize] {
        tree_print_helper(tree, child_id, depth + 1);
    }
}

pub fn tree_print(tree: &tree_t) {
    if tree.has_root == 0 {
        println!("(empty tree)");
        return;
    }
    tree_print_helper(tree, tree.root_id, 0);
}

pub fn tree_get_depth(tree: &tree_t, id: tree_id_t) -> i32 {
    if tree_contains(tree, id) == 0 {
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

pub fn tree_get_height(tree: &tree_t, id: tree_id_t) -> i32 {
    let node_ptr = tree_get_node(tree, id);
    if node_ptr.is_null() {
        return -1;
    }
    let node = unsafe { &*node_ptr };
    if node.child_count == 0 {
        return 0;
    }
    let mut max_height = 0;
    for &child_id in &node.child_ids[..node.child_count as usize] {
        let child_height = tree_get_height(tree, child_id);
        if child_height > max_height {
            max_height = child_height;
        }
    }
    max_height + 1
}

pub fn tree_count_descendants(tree: &tree_t, id: tree_id_t) -> i32 {
    let node_ptr = tree_get_node(tree, id);
    if node_ptr.is_null() {
        return -1;
    }
    let node = unsafe { &*node_ptr };
    let mut count = 0;
    for &child_id in &node.child_ids[..node.child_count as usize] {
        count += 1;
        count += tree_count_descendants(tree, child_id);
    }
    count
}

pub fn tree_find_path(tree: &tree_t, id: tree_id_t, path: &mut [tree_id_t], max_length: i32) -> i32 {
    if tree_contains(tree, id) == 0 {
        return -1;
    }
    let mut temp_path = Vec::new();
    let mut current_id = id;
    while temp_path.len() < 1000 {
        temp_path.push(current_id);
        if current_id == tree.root_id {
            break;
        }
        let node_ptr = tree_get_node(tree, current_id);
        if node_ptr.is_null() {
            return -1;
        }
        current_id = unsafe { (*node_ptr).parent_id };
    }
    let mut length = temp_path.len() as i32;
    if length > max_length {
        length = max_length;
    }
    for i in 0..length as usize {
        path[i] = temp_path[length as usize - 1 - i];
    }
    length
}
