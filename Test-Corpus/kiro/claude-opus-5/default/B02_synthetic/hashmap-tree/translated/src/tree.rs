//! Translation of `c_src/src/tree.c` / `c_src/include/tree.h`.
//!
//! The C code hands out `tree_node_t *` pointers stored as `void *` inside the
//! hashmap. Here nodes live in an arena (`Vec<TreeNode>`) and the hashmap stores
//! arena indices, which gives the same identity/aliasing semantics without unsafe
//! code. "Freeing" a node simply drops it out of the map; the arena slot is never
//! reused, matching the fact that the C program never observes recycled node
//! addresses.

use crate::cout::out_write;
use crate::hashmap::{Hashmap, TreeId};

pub const MAX_CHILDREN: usize = 32;
pub const MAX_DATA_LENGTH: usize = 256;

#[derive(Clone)]
pub struct TreeNode {
    pub id: TreeId,
    pub parent_id: TreeId,
    pub child_ids: [TreeId; MAX_CHILDREN],
    pub child_count: i32,
    pub data: [u8; MAX_DATA_LENGTH],
}

impl TreeNode {
    fn new() -> TreeNode {
        TreeNode {
            id: 0,
            parent_id: 0,
            child_ids: [0; MAX_CHILDREN],
            child_count: 0,
            data: [0; MAX_DATA_LENGTH],
        }
    }

    /// The bytes `printf("%s", node->data)` would emit: everything up to the first
    /// NUL byte.
    pub fn data_bytes(&self) -> &[u8] {
        let end = self
            .data
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(MAX_DATA_LENGTH);
        &self.data[..end]
    }
}

/// `strncpy(dst, src, MAX_DATA_LENGTH - 1)` followed by
/// `dst[MAX_DATA_LENGTH - 1] = '\0'`: copies at most 255 bytes, NUL-padding the
/// remainder of that range when `src` is shorter.
fn strncpy_data(dst: &mut [u8; MAX_DATA_LENGTH], src: &[u8]) {
    let n = MAX_DATA_LENGTH - 1;
    for i in 0..n {
        dst[i] = if i < src.len() { src[i] } else { 0 };
    }
    dst[MAX_DATA_LENGTH - 1] = 0;
}

pub struct Tree {
    pub node_map: Hashmap<usize>,
    /// Backing storage for the individually malloc'd nodes of the C version.
    pub nodes: Vec<TreeNode>,
    pub root_id: TreeId,
    pub has_root: i32,
    pub node_count: usize,
}

impl Tree {
    /// `tree_create`
    pub fn create() -> Tree {
        Tree {
            node_map: Hashmap::create(),
            nodes: Vec::new(),
            root_id: 0,
            has_root: 0,
            node_count: 0,
        }
    }

    /// `tree_delete`
    pub fn delete(self) {
        // Dropping the tree releases every node and the hashmap storage.
    }

    fn node_index(&self, id: TreeId) -> Option<usize> {
        self.node_map.get(id)
    }

    /// `tree_get_node`
    pub fn get_node(&self, id: TreeId) -> Option<&TreeNode> {
        self.node_index(id).map(|i| &self.nodes[i])
    }

    /// `tree_contains`
    pub fn contains(&self, id: TreeId) -> i32 {
        if self.get_node(id).is_some() {
            1
        } else {
            0
        }
    }

    /// `tree_size`
    pub fn size(&self) -> usize {
        self.node_count
    }

    /// `tree_add_node`
    pub fn add_node(&mut self, id: TreeId, parent_id: TreeId, data: Option<&str>) -> i32 {
        // Check if node already exists
        if self.contains(id) != 0 {
            crate::c_eprintf!("Error: Node with ID {} already exists\n", id);
            return -1;
        }

        // Allocate new node
        let mut node = TreeNode::new();

        node.id = id;
        node.parent_id = parent_id;
        node.child_count = 0;

        match data {
            Some(s) => strncpy_data(&mut node.data, s.as_bytes()),
            None => node.data[0] = 0,
        }

        // If this is the first node, make it the root
        if self.has_root == 0 {
            self.root_id = id;
            self.has_root = 1;
            node.parent_id = 0; // Root has no parent
        } else {
            // Find parent and add this node as a child
            let parent_index = match self.node_index(parent_id) {
                Some(i) => i,
                None => {
                    crate::c_eprintf!("Error: Parent node {} not found\n", parent_id);
                    return -1;
                }
            };

            if self.nodes[parent_index].child_count as usize >= MAX_CHILDREN {
                crate::c_eprintf!("Error: Parent has maximum children\n");
                return -1;
            }

            let slot = self.nodes[parent_index].child_count as usize;
            self.nodes[parent_index].child_ids[slot] = id;
            self.nodes[parent_index].child_count += 1;
        }

        // Add to hashmap
        let index = self.nodes.len();
        self.nodes.push(node);
        if self.node_map.put(id, index) != 0 {
            crate::c_eprintf!("Error: Failed to add node to hashmap\n");
            self.nodes.pop();
            return -1;
        }

        self.node_count += 1;
        0
    }

    /// `tree_remove_subtree`
    fn remove_subtree(&mut self, id: TreeId) -> i32 {
        let index = match self.node_index(id) {
            Some(i) => i,
            None => return -1,
        };

        // Recursively remove all children first
        let child_count = self.nodes[index].child_count;
        let child_ids = self.nodes[index].child_ids;
        for i in 0..child_count {
            self.remove_subtree(child_ids[i as usize]);
        }

        // Remove this node from hashmap
        if self.node_map.remove(id).is_some() {
            self.node_count -= 1;
        }

        0
    }

    /// `tree_remove_node`
    pub fn remove_node(&mut self, id: TreeId) -> i32 {
        let node_index = match self.node_index(id) {
            Some(i) => i,
            None => {
                crate::c_eprintf!("Error: Node {} not found\n", id);
                return -1;
            }
        };

        // If removing root, tree becomes empty
        if id == self.root_id {
            self.remove_subtree(id);
            self.has_root = 0;
            self.root_id = 0;
            return 0;
        }

        // Remove from parent's child list
        let parent_id = self.nodes[node_index].parent_id;
        if let Some(parent_index) = self.node_index(parent_id) {
            let mut i = 0;
            while i < self.nodes[parent_index].child_count {
                if self.nodes[parent_index].child_ids[i as usize] == id {
                    // Shift remaining children
                    let mut j = i;
                    while j < self.nodes[parent_index].child_count - 1 {
                        let next = self.nodes[parent_index].child_ids[(j + 1) as usize];
                        self.nodes[parent_index].child_ids[j as usize] = next;
                        j += 1;
                    }
                    self.nodes[parent_index].child_count -= 1;
                    break;
                }
                i += 1;
            }
        }

        // Remove this node and all descendants
        self.remove_subtree(id);

        0
    }

    /// `tree_print_helper`
    fn print_helper(&self, id: TreeId, depth: i32) {
        let node = match self.get_node(id) {
            Some(n) => n,
            None => return,
        };

        // Print indentation
        for _ in 0..depth {
            out_write(b"  ");
        }

        out_write(format!("[{}] ", node.id).as_bytes());
        out_write(node.data_bytes());
        out_write(b"\n");

        // Print children
        let child_count = node.child_count;
        let child_ids = node.child_ids;
        for i in 0..child_count {
            self.print_helper(child_ids[i as usize], depth + 1);
        }
    }

    /// `tree_print`
    pub fn print(&self) {
        if self.has_root == 0 {
            out_write(b"(empty tree)\n");
            return;
        }

        self.print_helper(self.root_id, 0);
    }

    /// `tree_get_depth`
    pub fn get_depth(&self, id: TreeId) -> i32 {
        if self.contains(id) == 0 {
            return -1;
        }

        let mut depth: i32 = 0;
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

    /// `tree_get_height`
    pub fn get_height(&self, id: TreeId) -> i32 {
        let node = match self.get_node(id) {
            Some(n) => n,
            None => return -1,
        };

        if node.child_count == 0 {
            return 0;
        }

        let child_count = node.child_count;
        let child_ids = node.child_ids;

        let mut max_height: i32 = 0;
        for i in 0..child_count {
            let child_height = self.get_height(child_ids[i as usize]);
            if child_height > max_height {
                max_height = child_height;
            }
        }

        max_height + 1
    }

    /// `tree_count_descendants`
    pub fn count_descendants(&self, id: TreeId) -> i32 {
        let node = match self.get_node(id) {
            Some(n) => n,
            None => return -1,
        };

        let child_count = node.child_count;
        let child_ids = node.child_ids;

        let mut count: i32 = 0;
        for i in 0..child_count {
            count += 1; // Count the child
            count += self.count_descendants(child_ids[i as usize]);
        }

        count
    }

    /// `tree_find_path`
    pub fn find_path(&self, id: TreeId, path: &mut [TreeId], max_length: i32) -> i32 {
        if self.contains(id) == 0 {
            return -1;
        }

        // Build path from node to root, then reverse
        let mut temp_path = [0u64; 1000];
        let mut length: i32 = 0;
        let mut current_id = id;

        while length < 1000 {
            temp_path[length as usize] = current_id;
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

        // Reverse into output path
        if length > max_length {
            length = max_length;
        }

        for i in 0..length {
            path[i as usize] = temp_path[(length - 1 - i) as usize];
        }

        length
    }
}
