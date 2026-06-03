// tree.rs - translation of tree.c
//
// Tree built on top of the open-addressing Hashmap. Nodes are heap-allocated
// (Box::into_raw) so that mutable borrows of one node don't alias with the
// Hashmap. We free them via Box::from_raw on removal.

use crate::hashmap::{Hashmap, TreeId};
use std::io::Write;

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

pub struct Tree {
    pub node_map: Box<Hashmap>,
    pub root_id: TreeId,
    pub has_root: bool,
    pub node_count: usize,
}

impl Tree {
    pub fn create() -> Box<Tree> {
        Box::new(Tree {
            node_map: Hashmap::create(),
            root_id: 0,
            has_root: false,
            node_count: 0,
        })
    }

    pub fn delete(tree: Box<Tree>) {
        // Iterate over all entries and free node values.
        let cap = tree.node_map.capacity;
        for i in 0..cap {
            let e = &tree.node_map.entries[i];
            if e.occupied && !e.deleted {
                let ptr = e.value;
                if !ptr.is_null() {
                    unsafe {
                        let _ = Box::from_raw(ptr);
                    }
                }
            }
        }
        // Box<Tree> drops here, freeing the Hashmap.
        drop(tree);
    }

    pub fn add_node(&mut self, id: TreeId, parent_id: TreeId, data: Option<&str>) -> i32 {
        if self.contains(id) {
            // fprintf to stderr matching C's "%lu"
            eprintln!("Error: Node with ID {} already exists", id);
            return -1;
        }

        // Allocate new node
        let mut node = Box::new(TreeNode {
            id,
            parent_id,
            child_ids: [0u64; MAX_CHILDREN],
            child_count: 0,
            data: [0u8; MAX_DATA_LENGTH],
        });

        // Copy data using strncpy semantics: copy up to MAX_DATA_LENGTH-1 bytes
        // and ensure last byte is NUL.
        if let Some(s) = data {
            let bytes = s.as_bytes();
            let copy_len = std::cmp::min(bytes.len(), MAX_DATA_LENGTH - 1);
            node.data[..copy_len].copy_from_slice(&bytes[..copy_len]);
            // strncpy pads with NUL up to n bytes; remaining already 0 from init.
            node.data[MAX_DATA_LENGTH - 1] = 0;
        } else {
            node.data[0] = 0;
        }

        if !self.has_root {
            self.root_id = id;
            self.has_root = true;
            node.parent_id = 0;
        } else {
            // Find parent
            let parent_ptr = self.node_map.get(parent_id);
            if parent_ptr.is_null() {
                eprintln!("Error: Parent node {} not found", parent_id);
                // Drop node implicitly
                return -1;
            }
            // Safety: parent_ptr was inserted as Box::into_raw and is exclusively
            // held in the hashmap. We have &mut self so no aliasing borrow exists.
            let parent: &mut TreeNode = unsafe { &mut *parent_ptr };
            if parent.child_count as usize >= MAX_CHILDREN {
                eprintln!("Error: Parent has maximum children");
                return -1;
            }
            let cc = parent.child_count as usize;
            parent.child_ids[cc] = id;
            parent.child_count += 1;
        }

        let raw = Box::into_raw(node);
        if self.node_map.put(id, raw) != 0 {
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
        let node_ptr = self.node_map.get(id);
        if node_ptr.is_null() {
            return -1;
        }
        // Snapshot the children first (since we'll recurse).
        let (children, count) = unsafe {
            let node: &TreeNode = &*node_ptr;
            (node.child_ids, node.child_count)
        };

        for i in 0..count as usize {
            self.remove_subtree(children[i]);
        }

        let removed = self.node_map.remove(id);
        if !removed.is_null() {
            unsafe {
                let _ = Box::from_raw(removed);
            }
            self.node_count -= 1;
        }
        0
    }

    pub fn remove_node(&mut self, id: TreeId) -> i32 {
        let node_ptr = self.node_map.get(id);
        if node_ptr.is_null() {
            eprintln!("Error: Node {} not found", id);
            return -1;
        }

        if id == self.root_id {
            self.remove_subtree(id);
            self.has_root = false;
            self.root_id = 0;
            return 0;
        }

        let parent_id = unsafe { (*node_ptr).parent_id };
        let parent_ptr = self.node_map.get(parent_id);
        if !parent_ptr.is_null() {
            let parent: &mut TreeNode = unsafe { &mut *parent_ptr };
            let mut i = 0i32;
            while i < parent.child_count {
                if parent.child_ids[i as usize] == id {
                    let mut j = i;
                    while j < parent.child_count - 1 {
                        parent.child_ids[j as usize] = parent.child_ids[(j + 1) as usize];
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

    pub fn get_node(&self, id: TreeId) -> *mut TreeNode {
        self.node_map.get(id)
    }

    pub fn contains(&self, id: TreeId) -> bool {
        !self.get_node(id).is_null()
    }

    pub fn size(&self) -> usize {
        self.node_count
    }

    fn print_helper<W: Write>(&self, w: &mut W, id: TreeId, depth: i32) {
        let node_ptr = self.get_node(id);
        if node_ptr.is_null() {
            return;
        }
        let node: &TreeNode = unsafe { &*node_ptr };

        for _ in 0..depth {
            write!(w, "  ").unwrap();
        }

        // Print [id] data\n where data is C string (up to NUL).
        let data_bytes = {
            let mut end = 0;
            while end < MAX_DATA_LENGTH && node.data[end] != 0 {
                end += 1;
            }
            &node.data[..end]
        };
        write!(w, "[{}] ", node.id).unwrap();
        w.write_all(data_bytes).unwrap();
        writeln!(w).unwrap();

        for i in 0..node.child_count as usize {
            self.print_helper(w, node.child_ids[i], depth + 1);
        }
    }

    pub fn print<W: Write>(&self, out: &mut W) {
        if !self.has_root {
            writeln!(out, "(empty tree)").unwrap();
            return;
        }
        self.print_helper(out, self.root_id, 0);
    }

    #[allow(dead_code)]
    pub fn get_depth(&self, id: TreeId) -> i32 {
        if !self.contains(id) {
            return -1;
        }
        let mut depth = 0i32;
        let mut current_id = id;
        while current_id != self.root_id {
            let n = self.get_node(current_id);
            if n.is_null() {
                return -1;
            }
            current_id = unsafe { (*n).parent_id };
            depth += 1;
        }
        depth
    }

    #[allow(dead_code)]
    pub fn get_height(&self, id: TreeId) -> i32 {
        let n = self.get_node(id);
        if n.is_null() {
            return -1;
        }
        let node: &TreeNode = unsafe { &*n };
        if node.child_count == 0 {
            return 0;
        }
        let mut max_height = 0i32;
        for i in 0..node.child_count as usize {
            let ch = self.get_height(node.child_ids[i]);
            if ch > max_height {
                max_height = ch;
            }
        }
        max_height + 1
    }

    #[allow(dead_code)]
    pub fn count_descendants(&self, id: TreeId) -> i32 {
        let n = self.get_node(id);
        if n.is_null() {
            return -1;
        }
        let node: &TreeNode = unsafe { &*n };
        let mut count = 0i32;
        for i in 0..node.child_count as usize {
            count += 1;
            count += self.count_descendants(node.child_ids[i]);
        }
        count
    }

    #[allow(dead_code)]
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
            let n = self.get_node(current_id);
            if n.is_null() {
                return -1;
            }
            current_id = unsafe { (*n).parent_id };
        }
        let mut out_len = length as i32;
        if out_len > max_length {
            out_len = max_length;
        }
        for i in 0..out_len as usize {
            path[i] = temp_path[length - 1 - i];
        }
        out_len
    }
}
