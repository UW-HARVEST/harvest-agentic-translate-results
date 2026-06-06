// tree.rs - faithful translation of tree.c

use crate::hashmap::{Hashmap, TreeId};
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
    pub fn new() -> Box<Self> {
        Box::new(TreeNode {
            id: 0,
            parent_id: 0,
            child_ids: [0; MAX_CHILDREN],
            child_count: 0,
            data: [0; MAX_DATA_LENGTH],
        })
    }

    pub fn data_str(&self) -> &str {
        let end = self.data.iter().position(|&b| b == 0).unwrap_or(MAX_DATA_LENGTH);
        std::str::from_utf8(&self.data[..end]).unwrap_or("")
    }
}

pub struct Tree {
    pub node_map: Hashmap,
    pub root_id: TreeId,
    pub has_root: i32,
    pub node_count: usize,
}

impl Tree {
    pub fn new() -> Box<Self> {
        Box::new(Tree {
            node_map: Hashmap::new(),
            root_id: 0,
            has_root: 0,
            node_count: 0,
        })
    }

    pub fn delete(self: Box<Self>) {
        // Free all nodes (Box::from_raw to drop)
        for i in 0..self.node_map.capacity {
            if self.node_map.entries[i].occupied != 0 && self.node_map.entries[i].deleted == 0 {
                let ptr = self.node_map.entries[i].value as *mut TreeNode;
                if !ptr.is_null() {
                    unsafe {
                        let _ = Box::from_raw(ptr);
                    }
                }
            }
        }
        // Box drops self
    }

    pub fn add_node(&mut self, id: TreeId, parent_id: TreeId, data: &str) -> i32 {
        // Check if node already exists
        if self.contains(id) {
            eprintln!("Error: Node with ID {} already exists", id);
            return -1;
        }

        // Allocate new node
        let mut node = TreeNode::new();
        node.id = id;
        node.parent_id = parent_id;
        node.child_count = 0;

        // strncpy with MAX_DATA_LENGTH - 1 (null terminator handled)
        let bytes = data.as_bytes();
        let copy_len = bytes.len().min(MAX_DATA_LENGTH - 1);
        node.data[..copy_len].copy_from_slice(&bytes[..copy_len]);
        node.data[copy_len] = 0;

        // If this is the first node, make it the root
        if self.has_root == 0 {
            self.root_id = id;
            self.has_root = 1;
            node.parent_id = 0;
        } else {
            // Find parent and add this node as a child
            let parent_ptr = self.node_map.get(parent_id) as *mut TreeNode;
            if parent_ptr.is_null() {
                eprintln!("Error: Parent node {} not found", parent_id);
                // node dropped automatically
                return -1;
            }

            unsafe {
                let parent = &mut *parent_ptr;
                if (parent.child_count as usize) >= MAX_CHILDREN {
                    eprintln!("Error: Parent has maximum children");
                    return -1;
                }

                parent.child_ids[parent.child_count as usize] = id;
                parent.child_count += 1;
            }
        }

        // Add to hashmap - leak the box and store raw pointer
        let raw = Box::into_raw(node);
        if self.node_map.put(id, raw as *mut c_void) != 0 {
            eprintln!("Error: Failed to add node to hashmap");
            unsafe {
                let _ = Box::from_raw(raw);
            }
            return -1;
        }

        self.node_count += 1;
        0
    }

    fn remove_subtree(&mut self, id: TreeId) -> i32 {
        let node_ptr = self.node_map.get(id) as *mut TreeNode;
        if node_ptr.is_null() {
            return -1;
        }

        // Capture children to recurse safely
        let (child_count, child_ids) = unsafe {
            let n = &*node_ptr;
            (n.child_count, n.child_ids)
        };

        for i in 0..child_count as usize {
            self.remove_subtree(child_ids[i]);
        }

        // Remove this node from hashmap
        let removed = self.node_map.remove(id) as *mut TreeNode;
        if !removed.is_null() {
            unsafe {
                let _ = Box::from_raw(removed);
            }
            self.node_count -= 1;
        }

        0
    }

    pub fn remove_node(&mut self, id: TreeId) -> i32 {
        let node_ptr = self.node_map.get(id) as *mut TreeNode;
        if node_ptr.is_null() {
            eprintln!("Error: Node {} not found", id);
            return -1;
        }

        // If removing root, tree becomes empty
        if id == self.root_id {
            self.remove_subtree(id);
            self.has_root = 0;
            self.root_id = 0;
            return 0;
        }

        // Remove from parent's child list
        let parent_id = unsafe { (*node_ptr).parent_id };
        let parent_ptr = self.node_map.get(parent_id) as *mut TreeNode;
        if !parent_ptr.is_null() {
            unsafe {
                let parent = &mut *parent_ptr;
                let mut found = -1i32;
                for i in 0..parent.child_count {
                    if parent.child_ids[i as usize] == id {
                        found = i;
                        break;
                    }
                }
                if found >= 0 {
                    let i = found;
                    let mut j = i;
                    while j < parent.child_count - 1 {
                        parent.child_ids[j as usize] = parent.child_ids[(j + 1) as usize];
                        j += 1;
                    }
                    parent.child_count -= 1;
                }
            }
        }

        // Remove this node and all descendants
        self.remove_subtree(id);

        0
    }

    pub fn get_node(&self, id: TreeId) -> *mut TreeNode {
        self.node_map.get(id) as *mut TreeNode
    }

    pub fn contains(&self, id: TreeId) -> bool {
        !self.get_node(id).is_null()
    }

    pub fn size(&self) -> usize {
        self.node_count
    }

    fn print_helper(&self, id: TreeId, depth: i32) {
        let node_ptr = self.get_node(id);
        if node_ptr.is_null() {
            return;
        }
        let node = unsafe { &*node_ptr };

        // Print indentation
        for _ in 0..depth {
            print!("  ");
        }

        println!("[{}] {}", node.id, node.data_str());

        // Print children
        for i in 0..node.child_count as usize {
            self.print_helper(node.child_ids[i], depth + 1);
        }
    }

    pub fn print(&self) {
        if self.has_root == 0 {
            println!("(empty tree)");
            return;
        }
        self.print_helper(self.root_id, 0);
    }

    pub fn get_depth(&self, id: TreeId) -> i32 {
        if !self.contains(id) {
            return -1;
        }

        let mut depth = 0i32;
        let mut current_id = id;

        while current_id != self.root_id {
            let node_ptr = self.get_node(current_id);
            if node_ptr.is_null() {
                return -1;
            }
            current_id = unsafe { (*node_ptr).parent_id };
            depth += 1;
        }

        depth
    }

    pub fn get_height(&self, id: TreeId) -> i32 {
        let node_ptr = self.get_node(id);
        if node_ptr.is_null() {
            return -1;
        }
        let node = unsafe { &*node_ptr };

        if node.child_count == 0 {
            return 0;
        }

        let mut max_height = 0i32;
        for i in 0..node.child_count as usize {
            let child_height = self.get_height(node.child_ids[i]);
            if child_height > max_height {
                max_height = child_height;
            }
        }

        max_height + 1
    }

    pub fn count_descendants(&self, id: TreeId) -> i32 {
        let node_ptr = self.get_node(id);
        if node_ptr.is_null() {
            return -1;
        }
        let node = unsafe { &*node_ptr };

        let mut count = 0i32;
        for i in 0..node.child_count as usize {
            count += 1;
            count += self.count_descendants(node.child_ids[i]);
        }

        count
    }

    pub fn find_path(&self, id: TreeId, path: &mut [TreeId], max_length: i32) -> i32 {
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

            let node_ptr = self.get_node(current_id);
            if node_ptr.is_null() {
                return -1;
            }
            current_id = unsafe { (*node_ptr).parent_id };
        }

        let mut len = length as i32;
        if len > max_length {
            len = max_length;
        }

        for i in 0..len as usize {
            path[i] = temp_path[length - 1 - i];
        }

        len
    }
}
