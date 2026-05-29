use std::collections::HashMap;
use std::sync::Arc;
use std::any::Any;
pub const BTREE_KEY_SIZE: usize = 10;
#[derive(Clone)]
pub struct BTreeKey {
    pub key: Vec<u8>,
    pub len: usize,
}
#[derive(Clone)]
pub struct Value {
    pub value: Vec<u8>,
    pub len: usize,
}
pub struct Entry {
    pub key: BTreeKey,
    pub value: Value,
}
pub struct EntryList {
    pub entries: Vec<Entry>,
    pub len: usize,
    pub cap: usize,
}
pub struct Node {
    pub key_hash: u32,
    pub p_key: [u8; BTREE_KEY_SIZE],
    pub key_len: usize,
    pub value: Value,
    pub child_left: Option<Arc<Node>>,
    pub child_right: Option<Arc<Node>>,
}
pub struct BTree {
    pub node: Option<Arc<Node>>,
}
impl BTree {
    pub fn new_btree() -> Self {
        BTree { node: None }
    }
    pub fn add_entry(&mut self, key: Vec<u8>, key_len: usize, value: Vec<u8>, value_len: usize) {
        let n_node = Node::new_node(key, key_len, value, value_len);
        match self.node.as_mut() {
            None => self.node = Some(n_node),
            Some(root_arc) => {
                let root_mut = Arc::get_mut(root_arc)
                    .expect("BTree::add_entry: Arc<Node> should be uniquely owned");
                root_mut.add_node(n_node);
            }
        }
    }
    pub fn list_entries(&self) -> EntryList {
        let cap = self.get_entry_count();
        let mut list = EntryList {
            entries: Vec::with_capacity(cap),
            len: 0,
            cap,
        };
        if let Some(node) = &self.node {
            node.list_node_entries(&mut list);
        }
        list
    }
    pub fn remove_entry(&mut self, key: &Vec<u8>, key_len: usize) {
        let key_len = min_size(BTREE_KEY_SIZE, key_len);
        let key_hash = calc_key_hash(key, key_len);
        if let Some(mut root) = self.node.take() {
            self.node = Node::delete_node(&mut root, key_hash, key.clone(), key_len);
        }
    }
    pub fn get_entry_count(&self) -> usize {
        match &self.node {
            None => 0,
            Some(node) => node.get_node_count(),
        }
    }
    pub fn find_entry(&self, key: &Vec<u8>, key_len: usize) -> Option<Value> {
        match &self.node {
            None => None,
            Some(node) => {
                let key_len = min_size(BTREE_KEY_SIZE, key_len);
                let key_hash = calc_key_hash(key, key_len);
                node.find_value(key_hash, key.clone(), key_len)
            }
        }
    }
    pub fn free_tree(&mut self) {
        self.node = None;
    }
}
impl Node {
    pub fn new_node(key: Vec<u8>, key_len: usize, value: Vec<u8>, value_len: usize) -> Arc<Self> {
        let kl = min_size(BTREE_KEY_SIZE, key_len);
        let mut p_key = [0u8; BTREE_KEY_SIZE];
        let copy_key_len = kl.min(key.len());
        if copy_key_len > 0 {
            p_key[..copy_key_len].copy_from_slice(&key[..copy_key_len]);
        }

        // Allocate value buffer of size value_len, zero-initialized, and copy
        // up to min(value_len, value.len()) bytes from input.
        let mut value_buf = vec![0u8; value_len];
        let copy_value_len = value_len.min(value.len());
        if copy_value_len > 0 {
            value_buf[..copy_value_len].copy_from_slice(&value[..copy_value_len]);
        }

        // Hash is computed over the bytes stored in p_key (only kl of them).
        let p_key_vec: Vec<u8> = p_key[..kl].to_vec();
        let key_hash = calc_key_hash(&p_key_vec, kl);

        Arc::new(Node {
            key_hash,
            p_key,
            key_len: kl,
            value: Value {
                value: value_buf,
                len: value_len,
            },
            child_left: None,
            child_right: None,
        })
    }
    pub fn add_node(&mut self, n_node: Arc<Node>) {
        if n_node.key_hash > self.key_hash {
            if self.child_right.is_none() {
                self.child_right = Some(n_node);
                return;
            }
            let right_arc = self.child_right.as_mut().unwrap();
            let right_mut = Arc::get_mut(right_arc)
                .expect("Node::add_node: child_right Arc should be uniquely owned");
            right_mut.add_node(n_node);
            return;
        }

        if n_node.key_hash == self.key_hash {
            // Compare n_node.key_len bytes of p_key (matching memcmp(self->p_key, n_node->p_key, n_node->key_len))
            let cmp_len = n_node.key_len.min(BTREE_KEY_SIZE);
            if self.p_key[..cmp_len] == n_node.p_key[..cmp_len] {
                // Same key: replace the value
                self.value = n_node.value.clone();
                return;
            }
        }

        if self.child_left.is_none() {
            self.child_left = Some(n_node);
            return;
        }
        let left_arc = self.child_left.as_mut().unwrap();
        let left_mut = Arc::get_mut(left_arc)
            .expect("Node::add_node: child_left Arc should be uniquely owned");
        left_mut.add_node(n_node);
    }
    pub fn free_node(&mut self) {
        self.child_left = None;
        self.child_right = None;
    }
    pub fn delete_node(root: &mut Arc<Node>, key_hash: u32, key: Vec<u8>, key_len: usize) -> Option<Arc<Node>> {
        let key_len = min_size(BTREE_KEY_SIZE, key_len);

        // We will set this to Some(replacement) (or None) if the current root
        // should be removed and replaced with that subtree.
        let mut replace_root_with: Option<Option<Arc<Node>>> = None;

        {
            let root_mut = Arc::get_mut(root)
                .expect("Node::delete_node: Arc<Node> should be uniquely owned");

            if key_hash < root_mut.key_hash {
                if let Some(mut left) = root_mut.child_left.take() {
                    let new_left = Self::delete_node(&mut left, key_hash, key.clone(), key_len);
                    root_mut.child_left = new_left;
                }
            } else if key_hash > root_mut.key_hash {
                if let Some(mut right) = root_mut.child_right.take() {
                    let new_right = Self::delete_node(&mut right, key_hash, key.clone(), key_len);
                    root_mut.child_right = new_right;
                }
            } else {
                // Found a node with matching hash; verify the actual key bytes match.
                let cmp_len = key_len.min(BTREE_KEY_SIZE).min(key.len());
                if cmp_len == key_len.min(BTREE_KEY_SIZE)
                    && root_mut.p_key[..cmp_len] == key[..cmp_len]
                {
                    if root_mut.child_left.is_none() {
                        replace_root_with = Some(root_mut.child_right.take());
                    } else if root_mut.child_right.is_none() {
                        replace_root_with = Some(root_mut.child_left.take());
                    } else {
                        // Two children: find the inorder successor (leftmost
                        // node in the right subtree).
                        let (succ_hash, succ_p_key, succ_key_len, succ_value) = {
                            let mut cur: &Node = root_mut.child_right.as_ref().unwrap();
                            while let Some(left) = &cur.child_left {
                                cur = left.as_ref();
                            }
                            (cur.key_hash, cur.p_key, cur.key_len, cur.value.clone())
                        };

                        // Copy successor's content into root
                        root_mut.key_hash = succ_hash;
                        root_mut.p_key = succ_p_key;
                        root_mut.key_len = succ_key_len;
                        root_mut.value = succ_value;

                        // Delete the successor from the right subtree.
                        if let Some(mut right) = root_mut.child_right.take() {
                            let new_right = Self::delete_node(
                                &mut right,
                                succ_hash,
                                succ_p_key.to_vec(),
                                succ_key_len,
                            );
                            root_mut.child_right = new_right;
                        }
                    }
                }
            }
        }

        if let Some(replacement) = replace_root_with {
            return replacement;
        }

        Some(root.clone())
    }
    pub fn get_node_count(&self) -> usize {
        let l = self
            .child_left
            .as_ref()
            .map_or(0, |n| n.get_node_count());
        let r = self
            .child_right
            .as_ref()
            .map_or(0, |n| n.get_node_count());
        1 + l + r
    }
    pub fn list_node_entries(&self, list: &mut EntryList) {
        if let Some(left) = &self.child_left {
            left.list_node_entries(list);
        }

        if list.len >= list.cap {
            return;
        }

        let key_bytes = self.p_key[..self.key_len].to_vec();
        let entry = Entry {
            key: BTreeKey {
                key: key_bytes,
                len: self.key_len,
            },
            value: self.value.clone(),
        };

        if list.entries.len() <= list.len {
            list.entries.push(entry);
        } else {
            list.entries[list.len] = entry;
        }
        list.len += 1;

        if let Some(right) = &self.child_right {
            right.list_node_entries(list);
        }
    }
    pub fn find_value(&self, key_hash: u32, key: Vec<u8>, key_len: usize) -> Option<Value> {
        let key_len = min_size(BTREE_KEY_SIZE, key_len);
        if self.key_hash == key_hash {
            let cmp_len = key_len.min(BTREE_KEY_SIZE).min(key.len());
            if cmp_len == key_len.min(BTREE_KEY_SIZE)
                && self.p_key[..cmp_len] == key[..cmp_len]
            {
                return Some(self.value.clone());
            }
        }

        if key_hash > self.key_hash {
            match &self.child_right {
                None => None,
                Some(c) => c.find_value(key_hash, key, key_len),
            }
        } else {
            match &self.child_left {
                None => None,
                Some(c) => c.find_value(key_hash, key, key_len),
            }
        }
    }
}
pub fn min_size(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
}
pub fn calc_key_hash(key: &Vec<u8>, key_len: usize) -> u32 {
    let mut key_sum: u32 = 0;
    let limit = key_len.min(key.len());
    for i in 0..limit {
        key_sum %= u32::MAX;
        let term = ((key[i] as u64) * ((i + 1) as u64)) % (u32::MAX as u64);
        key_sum = key_sum.wrapping_add(term as u32);
    }
    key_sum
}
pub fn btree_malloc<T: Any>(_: usize) -> T {
    // The C version `btree_malloc(size)` returns a zero-initialized memory
    // block of the requested size. In Rust we don't allocate raw memory
    // generically, but this signature is preserved for compatibility. It is
    // not used by the Rust BTree implementation itself (which uses Vec/Arc).
    //
    // SAFETY: Returning a zeroed value of an arbitrary T is generally unsafe;
    // however, this function is kept only for API parity with the C source
    // and isn't invoked elsewhere in this crate.
    unsafe { std::mem::zeroed() }
}
// Not needed in Rust, but we can implement it
pub fn btree_free<T: Any>(_: &T) {
    // No-op: Rust handles deallocation automatically through RAII when values
    // go out of scope. This function is kept solely for API parity with the
    // C implementation.
}
