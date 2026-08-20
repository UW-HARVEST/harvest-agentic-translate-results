//! Faithful translation of `c_src/src/tree.c` / `c_src/include/tree.h`.
//!
//! The C code keeps heap-allocated `tree_node_t *` values inside the hashmap.
//! Here the nodes live in an arena (`Vec<TreeNode>`) owned by the tree and the
//! hashmap stores arena indices, which gives the same aliasing/mutation
//! behaviour without raw pointers.

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
    fn new() -> TreeNode {
        TreeNode {
            id: 0,
            parent_id: 0,
            child_ids: [0; MAX_CHILDREN],
            child_count: 0,
            data: [0; MAX_DATA_LENGTH],
        }
    }

    /// The NUL-terminated contents of `data`, as `printf("%s", node->data)`
    /// would render them.
    pub fn data_bytes(&self) -> &[u8] {
        let end = self
            .data
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(MAX_DATA_LENGTH);
        &self.data[..end]
    }

    /// `strcmp(node->data, s) == 0`
    pub fn data_eq(&self, s: &str) -> bool {
        self.data_bytes() == s.as_bytes()
    }
}

/// `strncpy(node->data, data, MAX_DATA_LENGTH - 1)` followed by
/// `node->data[MAX_DATA_LENGTH - 1] = '\0'`.
fn strncpy_data(dst: &mut [u8; MAX_DATA_LENGTH], src: &str) {
    let n = MAX_DATA_LENGTH - 1;
    let src = src.as_bytes();
    let mut i = 0;
    while i < n && i < src.len() {
        dst[i] = src[i];
        i += 1;
    }
    // strncpy pads the remainder of the first n bytes with NUL.
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
    /// Stand-in for the individually `malloc`'d nodes.
    arena: Vec<TreeNode>,
}

impl Tree {
    /// `tree_t* tree_create(void)`
    pub fn create() -> Tree {
        Tree {
            node_map: Hashmap::create(),
            root_id: 0,
            has_root: 0,
            node_count: 0,
            arena: Vec::new(),
        }
    }

    /// `void tree_delete(tree_t *tree)` -- frees every node and the map.
    pub fn delete(self) {
        // Dropping the tree releases the arena and the hashmap storage.
    }

    /// `tree_node_t* tree_get_node(tree_t *tree, tree_id_t id)`; returns the
    /// arena index that stands in for the node pointer.
    pub fn get_node_idx(&self, id: TreeId) -> Option<usize> {
        self.node_map.get(id)
    }

    pub fn node(&self, idx: usize) -> &TreeNode {
        &self.arena[idx]
    }

    /// Convenience wrapper for `tree_get_node(...)` used by the tests.
    pub fn get_node(&self, id: TreeId) -> Option<&TreeNode> {
        self.get_node_idx(id).map(|i| &self.arena[i])
    }

    /// `int tree_contains(tree_t *tree, tree_id_t id)`
    pub fn contains(&self, id: TreeId) -> i32 {
        if self.get_node_idx(id).is_some() {
            1
        } else {
            0
        }
    }

    /// `size_t tree_size(tree_t *tree)`
    pub fn size(&self) -> usize {
        self.node_count
    }

    /// `int tree_add_node(tree_t *tree, tree_id_t id, tree_id_t parent_id, const char *data)`
    pub fn add_node(&mut self, id: TreeId, parent_id: TreeId, data: Option<&str>) -> i32 {
        // Check if node already exists
        if self.contains(id) != 0 {
            ceprintf!("Error: Node with ID {} already exists\n", id);
            return -1;
        }

        // Allocate new node
        let mut node = TreeNode::new();

        node.id = id;
        node.parent_id = parent_id;
        node.child_count = 0;

        match data {
            Some(d) => strncpy_data(&mut node.data, d),
            None => node.data[0] = 0,
        }

        // If this is the first node, make it the root
        if self.has_root == 0 {
            self.root_id = id;
            self.has_root = 1;
            node.parent_id = 0; // Root has no parent
        } else {
            // Find parent and add this node as a child
            let parent_idx = match self.get_node_idx(parent_id) {
                Some(i) => i,
                None => {
                    ceprintf!("Error: Parent node {} not found\n", parent_id);
                    return -1;
                }
            };

            if self.arena[parent_idx].child_count as usize >= MAX_CHILDREN {
                ceprintf!("Error: Parent has maximum children\n");
                return -1;
            }

            let slot = self.arena[parent_idx].child_count as usize;
            self.arena[parent_idx].child_ids[slot] = id;
            self.arena[parent_idx].child_count += 1;
        }

        // Add to hashmap
        let idx = self.arena.len();
        self.arena.push(node);
        if self.node_map.put(id, idx) != 0 {
            ceprintf!("Error: Failed to add node to hashmap\n");
            self.arena.pop();
            return -1;
        }

        self.node_count += 1;
        0
    }

    /// `static int tree_remove_subtree(tree_t *tree, tree_id_t id)`
    fn remove_subtree(&mut self, id: TreeId) -> i32 {
        let idx = match self.get_node_idx(id) {
            Some(i) => i,
            None => return -1,
        };

        // Recursively remove all children first
        let mut i = 0;
        while i < self.arena[idx].child_count {
            let child_id = self.arena[idx].child_ids[i as usize];
            self.remove_subtree(child_id);
            i += 1;
        }

        // Remove this node from hashmap
        if self.node_map.remove(id).is_some() {
            self.node_count -= 1;
        }

        0
    }

    /// `int tree_remove_node(tree_t *tree, tree_id_t id)`
    pub fn remove_node(&mut self, id: TreeId) -> i32 {
        let node_idx = match self.get_node_idx(id) {
            Some(i) => i,
            None => {
                ceprintf!("Error: Node {} not found\n", id);
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
        let parent_id = self.arena[node_idx].parent_id;
        if let Some(parent_idx) = self.get_node_idx(parent_id) {
            let mut i = 0;
            while i < self.arena[parent_idx].child_count {
                if self.arena[parent_idx].child_ids[i as usize] == id {
                    // Shift remaining children
                    let mut j = i;
                    while j < self.arena[parent_idx].child_count - 1 {
                        self.arena[parent_idx].child_ids[j as usize] =
                            self.arena[parent_idx].child_ids[(j + 1) as usize];
                        j += 1;
                    }
                    self.arena[parent_idx].child_count -= 1;
                    break;
                }
                i += 1;
            }
        }

        // Remove this node and all descendants
        self.remove_subtree(id);

        0
    }

    /// `static void tree_print_helper(tree_t *tree, tree_id_t id, int depth)`
    fn print_helper(&self, id: TreeId, depth: i32) {
        let idx = match self.get_node_idx(id) {
            Some(i) => i,
            None => return,
        };

        // Print indentation
        for _ in 0..depth {
            cprintf!("  ");
        }

        cprintf!("[{}] ", self.arena[idx].id);
        cio::out_bytes(self.arena[idx].data_bytes());
        cprintf!("\n");

        // Print children
        let mut i = 0;
        while i < self.arena[idx].child_count {
            let child_id = self.arena[idx].child_ids[i as usize];
            self.print_helper(child_id, depth + 1);
            i += 1;
        }
    }

    /// `void tree_print(tree_t *tree)`
    pub fn print(&self) {
        if self.has_root == 0 {
            cprintf!("(empty tree)\n");
            return;
        }

        self.print_helper(self.root_id, 0);
    }

    /// `int tree_get_depth(tree_t *tree, tree_id_t id)`
    pub fn get_depth(&self, id: TreeId) -> i32 {
        if self.contains(id) == 0 {
            return -1;
        }

        let mut depth: i32 = 0;
        let mut current_id = id;

        while current_id != self.root_id {
            let idx = match self.get_node_idx(current_id) {
                Some(i) => i,
                None => return -1,
            };
            current_id = self.arena[idx].parent_id;
            depth += 1;
        }

        depth
    }

    /// `int tree_get_height(tree_t *tree, tree_id_t id)`
    pub fn get_height(&self, id: TreeId) -> i32 {
        let idx = match self.get_node_idx(id) {
            Some(i) => i,
            None => return -1,
        };

        if self.arena[idx].child_count == 0 {
            return 0;
        }

        let mut max_height: i32 = 0;
        let mut i = 0;
        while i < self.arena[idx].child_count {
            let child_height = self.get_height(self.arena[idx].child_ids[i as usize]);
            if child_height > max_height {
                max_height = child_height;
            }
            i += 1;
        }

        max_height + 1
    }

    /// `int tree_count_descendants(tree_t *tree, tree_id_t id)`
    pub fn count_descendants(&self, id: TreeId) -> i32 {
        let idx = match self.get_node_idx(id) {
            Some(i) => i,
            None => return -1,
        };

        let mut count: i32 = 0;
        let mut i = 0;
        while i < self.arena[idx].child_count {
            count += 1; // Count the child
            count += self.count_descendants(self.arena[idx].child_ids[i as usize]);
            i += 1;
        }

        count
    }

    /// `int tree_find_path(tree_t *tree, tree_id_t id, tree_id_t *path, int max_length)`
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

            let idx = match self.get_node_idx(current_id) {
                Some(i) => i,
                None => return -1,
            };
            current_id = self.arena[idx].parent_id;
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
