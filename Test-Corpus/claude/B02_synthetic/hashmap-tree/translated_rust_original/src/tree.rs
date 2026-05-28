// Translated from c_src/src/tree.c

use crate::hashmap::{Hashmap, TreeId};

pub const MAX_CHILDREN: usize = 32;
pub const MAX_DATA_LENGTH: usize = 256;

pub struct TreeNode {
    pub id: TreeId,
    pub parent_id: TreeId,
    pub child_ids: [TreeId; MAX_CHILDREN],
    pub child_count: i32,
    pub data: [u8; MAX_DATA_LENGTH],
}

impl TreeNode {
    pub fn new(id: TreeId, parent_id: TreeId, data: &str) -> Self {
        let mut node = TreeNode {
            id,
            parent_id,
            child_ids: [0u64; MAX_CHILDREN],
            child_count: 0,
            data: [0u8; MAX_DATA_LENGTH],
        };
        // Mimic C's strncpy(node->data, data, MAX_DATA_LENGTH - 1) followed by
        // explicit null terminator.
        let bytes = data.as_bytes();
        let n = std::cmp::min(bytes.len(), MAX_DATA_LENGTH - 1);
        node.data[..n].copy_from_slice(&bytes[..n]);
        node.data[MAX_DATA_LENGTH - 1] = 0;
        node
    }

    pub fn data_str(&self) -> &str {
        let end = self
            .data
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.data.len());
        std::str::from_utf8(&self.data[..end]).unwrap_or("")
    }
}

pub struct Tree {
    pub node_map: Hashmap<TreeNode>,
    pub root_id: TreeId,
    pub has_root: bool,
    pub node_count: usize,
}

impl Tree {
    pub fn new() -> Self {
        Tree {
            node_map: Hashmap::new(),
            root_id: 0,
            has_root: false,
            node_count: 0,
        }
    }

    pub fn add_node(&mut self, id: TreeId, parent_id: TreeId, data: &str) -> i32 {
        // Check if node already exists
        if self.contains(id) {
            eprintln!("Error: Node with ID {} already exists", id);
            return -1;
        }

        let mut node = TreeNode::new(id, parent_id, data);

        if !self.has_root {
            // First node becomes root
            self.root_id = id;
            self.has_root = true;
            node.parent_id = 0;
        } else {
            // Find parent and add this node as a child
            let parent = self.node_map.get_mut(parent_id);
            match parent {
                None => {
                    eprintln!("Error: Parent node {} not found", parent_id);
                    return -1;
                }
                Some(parent) => {
                    if (parent.child_count as usize) >= MAX_CHILDREN {
                        eprintln!("Error: Parent has maximum children");
                        return -1;
                    }
                    let idx = parent.child_count as usize;
                    parent.child_ids[idx] = id;
                    parent.child_count += 1;
                }
            }
        }

        if self.node_map.put(id, node) != 0 {
            eprintln!("Error: Failed to add node to hashmap");
            return -1;
        }

        self.node_count += 1;
        0
    }

    fn remove_subtree(&mut self, id: TreeId) -> i32 {
        // Collect children first (avoid mutating while iterating)
        let child_info = match self.node_map.get(id) {
            None => return -1,
            Some(node) => {
                let cnt = node.child_count as usize;
                let mut ids = [0u64; MAX_CHILDREN];
                ids[..cnt].copy_from_slice(&node.child_ids[..cnt]);
                (cnt, ids)
            }
        };
        let (cnt, ids) = child_info;
        for i in 0..cnt {
            self.remove_subtree(ids[i]);
        }

        let removed = self.node_map.remove(id);
        if removed.is_some() {
            self.node_count -= 1;
        }
        0
    }

    pub fn remove_node(&mut self, id: TreeId) -> i32 {
        let parent_id = match self.node_map.get(id) {
            None => {
                eprintln!("Error: Node {} not found", id);
                return -1;
            }
            Some(node) => node.parent_id,
        };

        // If removing root, tree becomes empty
        if id == self.root_id {
            self.remove_subtree(id);
            self.has_root = false;
            self.root_id = 0;
            return 0;
        }

        // Remove from parent's child list
        if let Some(parent) = self.node_map.get_mut(parent_id) {
            let cnt = parent.child_count as usize;
            let mut found: Option<usize> = None;
            for i in 0..cnt {
                if parent.child_ids[i] == id {
                    found = Some(i);
                    break;
                }
            }
            if let Some(i) = found {
                for j in i..(cnt - 1) {
                    parent.child_ids[j] = parent.child_ids[j + 1];
                }
                parent.child_count -= 1;
            }
        }

        self.remove_subtree(id);
        0
    }

    pub fn get_node(&self, id: TreeId) -> Option<&TreeNode> {
        self.node_map.get(id)
    }

    pub fn contains(&self, id: TreeId) -> bool {
        self.node_map.contains(id)
    }

    pub fn size(&self) -> usize {
        self.node_count
    }

    fn print_helper(&self, id: TreeId, depth: i32) {
        let node = match self.node_map.get(id) {
            None => return,
            Some(n) => n,
        };

        for _ in 0..depth {
            print!("  ");
        }
        println!("[{}] {}", node.id, node.data_str());

        let cnt = node.child_count as usize;
        let ids: [TreeId; MAX_CHILDREN] = node.child_ids;
        for i in 0..cnt {
            self.print_helper(ids[i], depth + 1);
        }
    }

    pub fn print(&self) {
        if !self.has_root {
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
            let node = match self.node_map.get(current_id) {
                None => return -1,
                Some(n) => n,
            };
            current_id = node.parent_id;
            depth += 1;
        }

        depth
    }

    pub fn get_height(&self, id: TreeId) -> i32 {
        let node = match self.node_map.get(id) {
            None => return -1,
            Some(n) => n,
        };

        if node.child_count == 0 {
            return 0;
        }

        let cnt = node.child_count as usize;
        let ids: [TreeId; MAX_CHILDREN] = node.child_ids;

        let mut max_height = 0i32;
        for i in 0..cnt {
            let child_height = self.get_height(ids[i]);
            if child_height > max_height {
                max_height = child_height;
            }
        }

        max_height + 1
    }

    pub fn count_descendants(&self, id: TreeId) -> i32 {
        let node = match self.node_map.get(id) {
            None => return -1,
            Some(n) => n,
        };

        let cnt = node.child_count as usize;
        let ids: [TreeId; MAX_CHILDREN] = node.child_ids;

        let mut count = 0i32;
        for i in 0..cnt {
            count += 1;
            count += self.count_descendants(ids[i]);
        }
        count
    }

    pub fn find_path(&self, id: TreeId, path: &mut [TreeId], max_length: i32) -> i32 {
        if !self.contains(id) {
            return -1;
        }

        let mut temp_path = [0u64; 1000];
        let mut length: i32 = 0;
        let mut current_id = id;

        while length < 1000 {
            temp_path[length as usize] = current_id;
            length += 1;

            if current_id == self.root_id {
                break;
            }

            let node = match self.node_map.get(current_id) {
                None => return -1,
                Some(n) => n,
            };
            current_id = node.parent_id;
        }

        if length > max_length {
            length = max_length;
        }

        for i in 0..length as usize {
            path[i] = temp_path[(length as usize) - 1 - i];
        }

        length
    }
}
