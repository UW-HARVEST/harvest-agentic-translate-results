// tree.rs
//
// Faithful translation of c_src/src/tree.c and c_src/include/tree.h.
//
// The C implementation heap-allocates each `tree_node_t` and stores the raw
// pointer in the hashmap. Rust replaces the raw pointers with indices into an
// arena (`Tree::arena`); "freeing" a node clears its arena slot. The slot is
// never reused, which mirrors the fact that a freed pointer is never handed
// back out by this program.

use crate::cstdio;
use crate::hashmap::{Hashmap, TreeId};

pub const MAX_CHILDREN: usize = 32;
pub const MAX_DATA_LENGTH: usize = 256;

/// Mirrors `tree_node_t`.
#[derive(Clone)]
pub struct TreeNode {
    pub id: TreeId,
    pub parent_id: TreeId,
    pub child_ids: [TreeId; MAX_CHILDREN],
    pub child_count: i32,
    pub data: [u8; MAX_DATA_LENGTH],
}

impl TreeNode {
    /// Contents of a fresh `malloc(sizeof(tree_node_t))` are indeterminate in
    /// C; every field this program reads is assigned before use, so starting
    /// from zeroes is behaviourally equivalent.
    fn new() -> TreeNode {
        TreeNode {
            id: 0,
            parent_id: 0,
            child_ids: [0; MAX_CHILDREN],
            child_count: 0,
            data: [0; MAX_DATA_LENGTH],
        }
    }

    /// The bytes `printf("%s", node->data)` would emit: everything up to the
    /// first NUL terminator.
    pub fn data_cstr(&self) -> &[u8] {
        let end = self
            .data
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(MAX_DATA_LENGTH);
        &self.data[..end]
    }
}

/// Mirrors `tree_t`.
pub struct Tree {
    pub node_map: Hashmap,
    pub root_id: TreeId,
    pub has_root: i32,
    pub node_count: usize,
    /// Backing storage standing in for the individually `malloc`'d nodes.
    arena: Vec<Option<TreeNode>>,
}

/// `strncpy(dst, src, MAX_DATA_LENGTH - 1)` followed by
/// `dst[MAX_DATA_LENGTH - 1] = '\0'`, over a zero-initialised buffer.
fn strncpy_data(dst: &mut [u8; MAX_DATA_LENGTH], src: &[u8]) {
    let n = MAX_DATA_LENGTH - 1;
    let copy = if src.len() < n { src.len() } else { n };
    dst[..copy].copy_from_slice(&src[..copy]);
    // Remaining bytes are already NUL (strncpy zero-pads up to n).
    dst[MAX_DATA_LENGTH - 1] = 0;
}

impl Tree {
    /// `tree_create`
    pub fn create() -> Tree {
        Tree {
            node_map: Hashmap::create(),
            root_id: 0,
            has_root: 0,
            node_count: 0,
            arena: Vec::new(),
        }
    }

    /// `tree_free_node`
    fn free_node(&mut self, idx: usize) {
        self.arena[idx] = None;
    }

    /// `tree_delete`. Nothing observable happens, but the walk order is kept
    /// for fidelity.
    pub fn delete(mut self) {
        let capacity = self.node_map.capacity;
        let mut to_free = Vec::new();
        for i in 0..capacity {
            if self.node_map.entries[i].occupied && !self.node_map.entries[i].deleted {
                if let Some(idx) = self.node_map.entries[i].value {
                    to_free.push(idx);
                }
            }
        }
        for idx in to_free {
            self.free_node(idx);
        }
    }

    /// `tree_add_node`
    pub fn add_node(&mut self, id: TreeId, parent_id: TreeId, data: Option<&[u8]>) -> i32 {
        // Check if node already exists
        if self.contains(id) != 0 {
            c_eprintln!("Error: Node with ID {} already exists", id);
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
            let parent_idx = match self.get_node(parent_id) {
                Some(p) => p,
                None => {
                    c_eprintln!("Error: Parent node {} not found", parent_id);
                    // `free(node)`: nothing to do, `node` is dropped here.
                    return -1;
                }
            };

            if self.arena[parent_idx].as_ref().unwrap().child_count as usize >= MAX_CHILDREN {
                c_eprintln!("Error: Parent has maximum children");
                return -1;
            }

            let parent = self.arena[parent_idx].as_mut().unwrap();
            let slot = parent.child_count as usize;
            parent.child_ids[slot] = id;
            parent.child_count += 1;
        }

        // Add to hashmap
        let idx = self.arena.len();
        self.arena.push(Some(node));
        if self.node_map.put(id, Some(idx)) != 0 {
            c_eprintln!("Error: Failed to add node to hashmap");
            self.free_node(idx);
            return -1;
        }

        self.node_count += 1;
        0
    }

    /// `tree_remove_subtree`
    fn remove_subtree(&mut self, id: TreeId) -> i32 {
        let idx = match self.get_node(id) {
            Some(i) => i,
            None => return -1,
        };

        // Recursively remove all children first
        let child_count = self.arena[idx].as_ref().unwrap().child_count;
        for i in 0..child_count {
            let child_id = self.arena[idx].as_ref().unwrap().child_ids[i as usize];
            self.remove_subtree(child_id);
        }

        // Remove this node from hashmap
        if let Some(removed) = self.node_map.remove(id) {
            self.free_node(removed);
            self.node_count -= 1;
        }

        0
    }

    /// `tree_remove_node`
    pub fn remove_node(&mut self, id: TreeId) -> i32 {
        let idx = match self.get_node(id) {
            Some(i) => i,
            None => {
                c_eprintln!("Error: Node {} not found", id);
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
        let parent_id = self.arena[idx].as_ref().unwrap().parent_id;
        if let Some(parent_idx) = self.get_node(parent_id) {
            let parent = self.arena[parent_idx].as_mut().unwrap();
            for i in 0..parent.child_count {
                if parent.child_ids[i as usize] == id {
                    // Shift remaining children
                    for j in i..(parent.child_count - 1) {
                        parent.child_ids[j as usize] = parent.child_ids[(j + 1) as usize];
                    }
                    parent.child_count -= 1;
                    break;
                }
            }
        }

        // Remove this node and all descendants
        self.remove_subtree(id);

        0
    }

    /// `tree_get_node`
    pub fn get_node(&self, id: TreeId) -> Option<usize> {
        self.node_map.get(id)
    }

    /// Borrow a node the way C dereferences the pointer from `tree_get_node`.
    pub fn node(&self, id: TreeId) -> Option<&TreeNode> {
        self.get_node(id).and_then(|i| self.arena[i].as_ref())
    }

    /// `tree_contains`
    pub fn contains(&self, id: TreeId) -> i32 {
        i32::from(self.get_node(id).is_some())
    }

    /// `tree_size`
    pub fn size(&self) -> usize {
        self.node_count
    }

    /// `tree_print_helper`
    fn print_helper(&self, id: TreeId, depth: i32) {
        let node = match self.node(id) {
            Some(n) => n,
            None => return,
        };

        // Print indentation
        for _ in 0..depth {
            cstdio::out(b"  ");
        }

        cstdio::out(format!("[{}] ", node.id).as_bytes());
        cstdio::out(node.data_cstr());
        cstdio::out(b"\n");

        // Print children
        for i in 0..node.child_count {
            self.print_helper(node.child_ids[i as usize], depth + 1);
        }
    }

    /// `tree_print`
    pub fn print(&self) {
        if self.has_root == 0 {
            cstdio::out(b"(empty tree)\n");
            return;
        }

        self.print_helper(self.root_id, 0);
    }

    /// `tree_get_depth`
    pub fn get_depth(&self, id: TreeId) -> i32 {
        if self.contains(id) == 0 {
            return -1;
        }

        let mut depth = 0;
        let mut current_id = id;

        while current_id != self.root_id {
            let node = match self.node(current_id) {
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
        let node = match self.node(id) {
            Some(n) => n,
            None => return -1,
        };

        if node.child_count == 0 {
            return 0;
        }

        let mut max_height = 0;
        for i in 0..node.child_count {
            let child_height = self.get_height(node.child_ids[i as usize]);
            if child_height > max_height {
                max_height = child_height;
            }
        }

        max_height + 1
    }

    /// `tree_count_descendants`
    pub fn count_descendants(&self, id: TreeId) -> i32 {
        let node = match self.node(id) {
            Some(n) => n,
            None => return -1,
        };

        let mut count = 0;
        for i in 0..node.child_count {
            count += 1; // Count the child
            count += self.count_descendants(node.child_ids[i as usize]);
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

            let node = match self.node(current_id) {
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
