//! Faithful translation of c_src/src/tree.c and c_src/include/tree.h.
//!
//! The C code stores `tree_node_t *` pointers in the hashmap and mutates nodes
//! through those pointers.  Here nodes live in an arena (`Vec<TreeNode>`) owned
//! by the tree and the hashmap stores arena indices, which gives the same
//! aliasing behavior without unsafe code.  Nodes "freed" by the C code are
//! simply left in the arena; they are unreachable because their id is removed
//! from the map, exactly as in C.

use crate::cio;
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
    fn new() -> Self {
        TreeNode {
            id: 0,
            parent_id: 0,
            child_ids: [0; MAX_CHILDREN],
            child_count: 0,
            data: [0; MAX_DATA_LENGTH],
        }
    }

    /// The bytes of `data` up to (excluding) the terminating NUL, i.e. what
    /// printf("%s") would emit.
    pub fn data_cstr(&self) -> &[u8] {
        let end = self
            .data
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(MAX_DATA_LENGTH);
        &self.data[..end]
    }
}

/// strncpy(dst, src, MAX_DATA_LENGTH - 1) followed by dst[MAX_DATA_LENGTH-1] = '\0'
fn strncpy_bounded(dst: &mut [u8; MAX_DATA_LENGTH], src: &[u8]) {
    let n = MAX_DATA_LENGTH - 1;
    let mut i = 0;
    while i < n && i < src.len() {
        dst[i] = src[i];
        i += 1;
    }
    // strncpy zero-pads the remainder of the n bytes.
    while i < n {
        dst[i] = 0;
        i += 1;
    }
    dst[MAX_DATA_LENGTH - 1] = 0;
}

pub struct Tree {
    pub node_map: Hashmap<usize>,
    pub root_id: TreeId,
    pub has_root: i32,
    pub node_count: usize,
    nodes: Vec<TreeNode>,
}

impl Tree {
    /// tree_create()
    pub fn create() -> Self {
        Tree {
            node_map: Hashmap::create(),
            root_id: 0,
            has_root: 0,
            node_count: 0,
            nodes: Vec::new(),
        }
    }

    /// tree_add_node()
    pub fn add_node(&mut self, id: TreeId, parent_id: TreeId, data: Option<&str>) -> i32 {
        // Check if node already exists
        if self.contains(id) != 0 {
            c_eprintf!("Error: Node with ID {} already exists\n", id);
            return -1;
        }

        // Allocate new node
        let mut node = TreeNode::new();

        node.id = id;
        node.parent_id = parent_id;
        node.child_count = 0;

        if let Some(d) = data {
            strncpy_bounded(&mut node.data, d.as_bytes());
        } else {
            node.data[0] = 0;
        }

        // If this is the first node, make it the root
        if self.has_root == 0 {
            self.root_id = id;
            self.has_root = 1;
            node.parent_id = 0; // Root has no parent
        } else {
            // Find parent and add this node as a child
            let parent = match self.get_node(parent_id) {
                Some(p) => p,
                None => {
                    c_eprintf!("Error: Parent node {} not found\n", parent_id);
                    return -1;
                }
            };

            if self.nodes[parent].child_count as usize >= MAX_CHILDREN {
                c_eprintf!("Error: Parent has maximum children\n");
                return -1;
            }

            let slot = self.nodes[parent].child_count as usize;
            self.nodes[parent].child_ids[slot] = id;
            self.nodes[parent].child_count += 1;
        }

        // Add to hashmap
        let index = self.nodes.len();
        self.nodes.push(node);
        if self.node_map.put(id, index) != 0 {
            c_eprintf!("Error: Failed to add node to hashmap\n");
            return -1;
        }

        self.node_count += 1;
        0
    }

    /// tree_remove_subtree() (static helper in tree.c)
    fn remove_subtree(&mut self, id: TreeId) -> i32 {
        let node = match self.get_node(id) {
            Some(n) => n,
            None => return -1,
        };

        // Recursively remove all children first
        let child_count = self.nodes[node].child_count;
        for i in 0..child_count {
            let child_id = self.nodes[node].child_ids[i as usize];
            self.remove_subtree(child_id);
        }

        // Remove this node from hashmap
        let removed = self.node_map.remove(id);
        if removed.is_some() {
            self.node_count = self.node_count.wrapping_sub(1);
        }

        0
    }

    /// tree_remove_node()
    pub fn remove_node(&mut self, id: TreeId) -> i32 {
        let node = match self.get_node(id) {
            Some(n) => n,
            None => {
                c_eprintf!("Error: Node {} not found\n", id);
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
        let parent_id = self.nodes[node].parent_id;
        if let Some(parent) = self.get_node(parent_id) {
            let mut i = 0;
            while i < self.nodes[parent].child_count {
                if self.nodes[parent].child_ids[i as usize] == id {
                    // Shift remaining children
                    let mut j = i;
                    while j < self.nodes[parent].child_count - 1 {
                        self.nodes[parent].child_ids[j as usize] =
                            self.nodes[parent].child_ids[(j + 1) as usize];
                        j += 1;
                    }
                    self.nodes[parent].child_count -= 1;
                    break;
                }
                i += 1;
            }
        }

        // Remove this node and all descendants
        self.remove_subtree(id);

        0
    }

    /// tree_get_node(): returns the arena index of the node, if present.
    pub fn get_node(&self, id: TreeId) -> Option<usize> {
        self.node_map.get(id)
    }

    /// Borrow a node by its arena index.
    pub fn node(&self, index: usize) -> &TreeNode {
        &self.nodes[index]
    }

    /// tree_contains()
    pub fn contains(&self, id: TreeId) -> i32 {
        if self.get_node(id).is_some() {
            1
        } else {
            0
        }
    }

    /// tree_size()
    pub fn size(&self) -> usize {
        self.node_count
    }

    /// tree_print_helper() (static helper in tree.c)
    fn print_helper(&self, id: TreeId, depth: i32) {
        let node = match self.get_node(id) {
            Some(n) => n,
            None => return,
        };

        // Print indentation
        for _i in 0..depth {
            c_printf!("  ");
        }

        c_printf!("[{}] ", self.nodes[node].id);
        cio::out_bytes(self.nodes[node].data_cstr());
        c_printf!("\n");

        // Print children
        let child_count = self.nodes[node].child_count;
        for i in 0..child_count {
            let child_id = self.nodes[node].child_ids[i as usize];
            self.print_helper(child_id, depth + 1);
        }
    }

    /// tree_print()
    pub fn print(&self) {
        if self.has_root == 0 {
            c_printf!("(empty tree)\n");
            return;
        }

        self.print_helper(self.root_id, 0);
    }

    /// tree_get_depth()
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
            current_id = self.nodes[node].parent_id;
            depth += 1;
        }

        depth
    }

    /// tree_get_height()
    pub fn get_height(&self, id: TreeId) -> i32 {
        let node = match self.get_node(id) {
            Some(n) => n,
            None => return -1,
        };

        if self.nodes[node].child_count == 0 {
            return 0;
        }

        let mut max_height: i32 = 0;
        for i in 0..self.nodes[node].child_count {
            let child_height = self.get_height(self.nodes[node].child_ids[i as usize]);
            if child_height > max_height {
                max_height = child_height;
            }
        }

        max_height + 1
    }

    /// tree_count_descendants()
    pub fn count_descendants(&self, id: TreeId) -> i32 {
        let node = match self.get_node(id) {
            Some(n) => n,
            None => return -1,
        };

        let mut count: i32 = 0;
        for i in 0..self.nodes[node].child_count {
            count += 1; // Count the child
            count += self.count_descendants(self.nodes[node].child_ids[i as usize]);
        }

        count
    }

    /// tree_find_path()
    pub fn find_path(&self, id: TreeId, path: &mut [TreeId], max_length: i32) -> i32 {
        if self.contains(id) == 0 {
            return -1;
        }

        // Build path from node to root, then reverse
        let mut temp_path: [TreeId; 1000] = [0; 1000];
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
            current_id = self.nodes[node].parent_id;
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
