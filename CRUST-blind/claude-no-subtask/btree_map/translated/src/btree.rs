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
        if self.node.is_none() {
            self.node = Some(n_node);
            return;
        }
        let root = self.node.as_mut().unwrap();
        let inner = Arc::get_mut(root).expect("BTree root has multiple references");
        inner.add_node(n_node);
    }
    pub fn list_entries(&self) -> EntryList {
        let cap = self.get_entry_count();
        let mut list = EntryList {
            entries: Vec::with_capacity(cap),
            len: 0,
            cap,
        };
        if let Some(node) = self.node.as_ref() {
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
        match self.node.as_ref() {
            Some(n) => n.get_node_count(),
            None => 0,
        }
    }
    pub fn find_entry(&self, key: &Vec<u8>, key_len: usize) -> Option<Value> {
        let node = self.node.as_ref()?;
        let key_len = min_size(BTREE_KEY_SIZE, key_len);
        let key_hash = calc_key_hash(key, key_len);
        node.find_value(key_hash, key.clone(), key_len)
    }
    pub fn free_tree(&mut self) {
        // Rust handles cleanup automatically when Arc count drops to 0.
        self.node = None;
    }
}
impl Node {
    pub fn new_node(key: Vec<u8>, key_len: usize, value: Vec<u8>, value_len: usize) -> Arc<Self> {
        let key_len = min_size(BTREE_KEY_SIZE, key_len);
        let mut p_key = [0u8; BTREE_KEY_SIZE];
        let copy_key_len = std::cmp::min(key_len, key.len());
        if copy_key_len > 0 {
            p_key[..copy_key_len].copy_from_slice(&key[..copy_key_len]);
        }

        // Compute hash over the bounded p_key contents (matches C: hash of node->p_key, node->key_len)
        let p_key_vec = p_key[..key_len].to_vec();
        let key_hash = calc_key_hash(&p_key_vec, key_len);

        // Allocate value buffer and copy value contents (matches C btree_malloc + memcpy semantics).
        let mut value_buf = vec![0u8; value_len];
        let copy_value_len = std::cmp::min(value_len, value.len());
        if copy_value_len > 0 {
            value_buf[..copy_value_len].copy_from_slice(&value[..copy_value_len]);
        }

        Arc::new(Node {
            key_hash,
            p_key,
            key_len,
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
            let right = self.child_right.as_mut().unwrap();
            let inner = Arc::get_mut(right).expect("child_right has multiple references");
            inner.add_node(n_node);
            return;
        }

        if n_node.key_hash == self.key_hash {
            let cmp_len = std::cmp::min(n_node.key_len, BTREE_KEY_SIZE);
            if self.p_key[..cmp_len] == n_node.p_key[..cmp_len] {
                // Same key: update value. Matches C semantics where existing value buffer
                // is overwritten and len adjusted.
                self.value.len = n_node.value.len;
                self.value.value = n_node.value.value.clone();
                // n_node is dropped at end of scope, freeing its resources.
                return;
            }
        }

        if self.child_left.is_none() {
            self.child_left = Some(n_node);
            return;
        }
        let left = self.child_left.as_mut().unwrap();
        let inner = Arc::get_mut(left).expect("child_left has multiple references");
        inner.add_node(n_node);
    }
    pub fn free_node(&mut self) {
        // Rust automatically frees memory when Arc reference count reaches zero,
        // and recursively drops children. Just clear children to release them.
        self.child_left = None;
        self.child_right = None;
    }
    pub fn delete_node(root: &mut Arc<Node>, key_hash: u32, key: Vec<u8>, key_len: usize) -> Option<Arc<Node>> {
        let key_len = min_size(BTREE_KEY_SIZE, key_len);
        let root_hash = root.key_hash;

        if key_hash < root_hash {
            // Recurse into left child
            let left_taken = Arc::get_mut(root).expect("delete_node: multi-ref root").child_left.take();
            if let Some(mut left) = left_taken {
                let new_left = Node::delete_node(&mut left, key_hash, key, key_len);
                Arc::get_mut(root).expect("delete_node: multi-ref root").child_left = new_left;
            }
            return Some(root.clone());
        }

        if key_hash > root_hash {
            // Recurse into right child
            let right_taken = Arc::get_mut(root).expect("delete_node: multi-ref root").child_right.take();
            if let Some(mut right) = right_taken {
                let new_right = Node::delete_node(&mut right, key_hash, key, key_len);
                Arc::get_mut(root).expect("delete_node: multi-ref root").child_right = new_right;
            }
            return Some(root.clone());
        }

        // Hash matches: check actual key bytes
        let cmp_len = std::cmp::min(std::cmp::min(key_len, key.len()), BTREE_KEY_SIZE);
        let key_matches = root.p_key[..cmp_len] == key[..cmp_len];

        if !key_matches {
            // Hash collision but different key: keep root unchanged (matches C behavior)
            return Some(root.clone());
        }

        // Need to delete this node
        {
            let inner = Arc::get_mut(root).expect("delete_node: multi-ref root");

            if inner.child_left.is_none() {
                // Replace with right child (or None if also missing)
                return inner.child_right.take();
            }
            if inner.child_right.is_none() {
                // Replace with left child
                return inner.child_left.take();
            }

            // Two children: find inorder successor (leftmost of right subtree)
            let (succ_hash, succ_pkey, succ_keylen, succ_value) = {
                let mut current = inner.child_right.as_ref().unwrap();
                while let Some(left) = current.child_left.as_ref() {
                    current = left;
                }
                (current.key_hash, current.p_key, current.key_len, current.value.clone())
            };

            // Copy successor's content into root
            inner.key_hash = succ_hash;
            inner.p_key = succ_pkey;
            inner.key_len = succ_keylen;
            inner.value = succ_value;

            // Delete the successor from right subtree
            let succ_key_vec = succ_pkey[..succ_keylen].to_vec();
            if let Some(mut right) = inner.child_right.take() {
                let new_right = Node::delete_node(&mut right, succ_hash, succ_key_vec, succ_keylen);
                inner.child_right = new_right;
            }
        }

        Some(root.clone())
    }
    pub fn get_node_count(&self) -> usize {
        let left = self.child_left.as_ref().map(|n| n.get_node_count()).unwrap_or(0);
        let right = self.child_right.as_ref().map(|n| n.get_node_count()).unwrap_or(0);
        1 + left + right
    }
    pub fn list_node_entries(&self, list: &mut EntryList) {
        if let Some(left) = self.child_left.as_ref() {
            left.list_node_entries(list);
        }

        if list.len >= list.cap {
            return;
        }

        let key_bytes = self.p_key[..self.key_len].to_vec();
        let value_bytes = self.value.value[..self.value.len.min(self.value.value.len())].to_vec();
        let entry = Entry {
            key: BTreeKey {
                key: key_bytes,
                len: self.key_len,
            },
            value: Value {
                value: value_bytes,
                len: self.value.len,
            },
        };
        list.entries.push(entry);
        list.len += 1;

        if let Some(right) = self.child_right.as_ref() {
            right.list_node_entries(list);
        }
    }
    pub fn find_value(&self, key_hash: u32, key: Vec<u8>, key_len: usize) -> Option<Value> {
        let key_len = min_size(BTREE_KEY_SIZE, key_len);
        let cmp_len = std::cmp::min(std::cmp::min(key_len, key.len()), BTREE_KEY_SIZE);
        if self.key_hash == key_hash {
            if self.p_key[..cmp_len] == key[..cmp_len] {
                return Some(self.value.clone());
            }
        }

        if key_hash > self.key_hash {
            return match self.child_right.as_ref() {
                Some(n) => n.find_value(key_hash, key, key_len),
                None => None,
            };
        }

        match self.child_left.as_ref() {
            Some(n) => n.find_value(key_hash, key, key_len),
            None => None,
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
    let len = std::cmp::min(key_len, key.len());
    for i in 0..len {
        key_sum = key_sum % u32::MAX;
        let term = ((key[i] as u32).wrapping_mul((i as u32).wrapping_add(1))) % u32::MAX;
        key_sum = key_sum.wrapping_add(term);
    }
    key_sum
}
pub fn btree_malloc<T: Any>(_: usize) -> T {
    // SAFETY: This mirrors the C `btree_malloc` which allocates and zero-initializes
    // a block of memory. For arbitrary `T: Any`, the closest safe-Rust analogue is a
    // zero-initialized value; this is unsafe in general but matches C semantics, and
    // this function is not invoked from within the safe Rust translation itself.
    unsafe { std::mem::zeroed() }
}
// Not needed in Rust, but we can implement it
pub fn btree_free<T: Any>(_: &T) {
    // No-op: Rust's ownership/Drop trait handles deallocation automatically.
    // The reference is dropped at the end of this function's scope.
}
