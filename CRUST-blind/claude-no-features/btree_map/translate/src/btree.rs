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

#[allow(dead_code)]
fn keys_equal(p_key: &[u8; BTREE_KEY_SIZE], key: &[u8], key_len: usize) -> bool {
    let n = key_len.min(key.len()).min(BTREE_KEY_SIZE);
    p_key[..n] == key[..n]
}

impl BTree {
    pub fn new_btree() -> Self {
        BTree { node: None }
    }
    pub fn add_entry(&mut self, key: Vec<u8>, key_len: usize, value: Vec<u8>, value_len: usize) {
        let n_node = Node::new_node(key, key_len, value, value_len);
        match self.node.as_mut() {
            None => {
                self.node = Some(n_node);
            }
            Some(root) => {
                let root_mut = Arc::get_mut(root).expect("Arc has unique ownership");
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
        if let Some(root) = &self.node {
            root.list_node_entries(&mut list);
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
            Some(root) => root.get_node_count(),
        }
    }
    pub fn find_entry(&self, key: &Vec<u8>, key_len: usize) -> Option<Value> {
        let root = self.node.as_ref()?;
        let key_len = min_size(BTREE_KEY_SIZE, key_len);
        let key_hash = calc_key_hash(key, key_len);
        root.find_value(key_hash, key.clone(), key_len)
    }
    pub fn free_tree(&mut self) {
        self.node = None;
    }
}
impl Node {
    pub fn new_node(key: Vec<u8>, key_len: usize, value: Vec<u8>, value_len: usize) -> Arc<Self> {
        let actual_key_len = min_size(BTREE_KEY_SIZE, key_len);
        let copy_len = actual_key_len.min(key.len());
        let mut p_key = [0u8; BTREE_KEY_SIZE];
        for i in 0..copy_len {
            p_key[i] = key[i];
        }

        // Compute hash on the stored p_key bytes (length actual_key_len)
        let key_for_hash: Vec<u8> = p_key[..actual_key_len].to_vec();
        let key_hash = calc_key_hash(&key_for_hash, actual_key_len);

        // Allocate value buffer of value_len bytes, copy from input
        let mut value_buf = vec![0u8; value_len];
        let v_copy_len = value_len.min(value.len());
        for i in 0..v_copy_len {
            value_buf[i] = value[i];
        }

        Arc::new(Node {
            key_hash,
            p_key,
            key_len: actual_key_len,
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
            match self.child_right.as_mut() {
                None => {
                    self.child_right = Some(n_node);
                }
                Some(right) => {
                    let right_mut = Arc::get_mut(right).expect("Arc has unique ownership");
                    right_mut.add_node(n_node);
                }
            }
            return;
        }

        if n_node.key_hash == self.key_hash {
            let n = n_node.key_len.min(BTREE_KEY_SIZE);
            if self.p_key[..n] == n_node.p_key[..n] {
                // Update existing entry's value
                self.value.value = n_node.value.value.clone();
                self.value.len = n_node.value.len;
                return;
            }
        }

        match self.child_left.as_mut() {
            None => {
                self.child_left = Some(n_node);
            }
            Some(left) => {
                let left_mut = Arc::get_mut(left).expect("Arc has unique ownership");
                left_mut.add_node(n_node);
            }
        }
    }
    pub fn free_node(&mut self) {
        // Rust handles deallocation automatically when Arc/Vec are dropped.
        // Explicitly drop children and value buffer to mirror C semantics.
        self.child_left = None;
        self.child_right = None;
        self.value.value.clear();
        self.value.len = 0;
    }
    pub fn delete_node(
        root: &mut Arc<Node>,
        key_hash: u32,
        key: Vec<u8>,
        key_len: usize,
    ) -> Option<Arc<Node>> {
        let key_len = min_size(BTREE_KEY_SIZE, key_len);
        let root_mut = Arc::get_mut(root).expect("Arc has unique ownership");

        if key_hash < root_mut.key_hash {
            if let Some(mut left) = root_mut.child_left.take() {
                root_mut.child_left = Node::delete_node(&mut left, key_hash, key, key_len);
            }
        } else if key_hash > root_mut.key_hash {
            if let Some(mut right) = root_mut.child_right.take() {
                root_mut.child_right = Node::delete_node(&mut right, key_hash, key, key_len);
            }
        } else {
            // hashes equal - check the actual key bytes
            let cmp_n = key_len.min(key.len()).min(BTREE_KEY_SIZE);
            if root_mut.p_key[..cmp_n] == key[..cmp_n] {
                // Found the node to delete
                if root_mut.child_left.is_none() {
                    return root_mut.child_right.take();
                } else if root_mut.child_right.is_none() {
                    return root_mut.child_left.take();
                }

                // Two children: find in-order successor (leftmost in right subtree)
                let (succ_hash, succ_p_key, succ_key_len, succ_val_buf, succ_val_len) = {
                    let mut temp: &Arc<Node> = root_mut
                        .child_right
                        .as_ref()
                        .expect("right child exists");
                    while let Some(left) = &temp.child_left {
                        temp = left;
                    }
                    (
                        temp.key_hash,
                        temp.p_key,
                        temp.key_len,
                        temp.value.value.clone(),
                        temp.value.len,
                    )
                };

                // Copy successor's content into root
                root_mut.key_hash = succ_hash;
                root_mut.p_key = succ_p_key;
                root_mut.key_len = succ_key_len;
                root_mut.value.value = succ_val_buf;
                root_mut.value.len = succ_val_len;

                // Remove successor from right subtree
                if let Some(mut right) = root_mut.child_right.take() {
                    let succ_key_vec: Vec<u8> = succ_p_key[..succ_key_len].to_vec();
                    root_mut.child_right =
                        Node::delete_node(&mut right, succ_hash, succ_key_vec, succ_key_len);
                }
            }
        }

        Some(root.clone())
    }
    pub fn get_node_count(&self) -> usize {
        let l = self
            .child_left
            .as_ref()
            .map(|n| n.get_node_count())
            .unwrap_or(0);
        let r = self
            .child_right
            .as_ref()
            .map(|n| n.get_node_count())
            .unwrap_or(0);
        1 + l + r
    }
    pub fn list_node_entries(&self, list: &mut EntryList) {
        if let Some(left) = &self.child_left {
            left.list_node_entries(list);
        }

        if list.len >= list.cap {
            return;
        }

        let key_bytes: Vec<u8> = self.p_key[..self.key_len].to_vec();
        let val_bytes: Vec<u8> = self.value.value.clone();

        let entry = Entry {
            key: BTreeKey {
                key: key_bytes,
                len: self.key_len,
            },
            value: Value {
                value: val_bytes,
                len: self.value.len,
            },
        };
        list.entries.push(entry);
        list.len += 1;

        if let Some(right) = &self.child_right {
            right.list_node_entries(list);
        }
    }
    pub fn find_value(&self, key_hash: u32, key: Vec<u8>, key_len: usize) -> Option<Value> {
        let key_len = min_size(BTREE_KEY_SIZE, key_len);
        if self.key_hash == key_hash {
            let cmp_n = key_len.min(key.len()).min(BTREE_KEY_SIZE);
            if self.p_key[..cmp_n] == key[..cmp_n] {
                return Some(self.value.clone());
            }
        }

        if key_hash > self.key_hash {
            return self
                .child_right
                .as_ref()?
                .find_value(key_hash, key, key_len);
        }

        self.child_left.as_ref()?.find_value(key_hash, key, key_len)
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
    let n = key_len.min(key.len());
    for i in 0..n {
        if key_sum == u32::MAX {
            key_sum = 0;
        }
        let term64 = (key[i] as u64) * ((i as u64) + 1);
        let term = (term64 % (u32::MAX as u64)) as u32;
        key_sum = key_sum.wrapping_add(term);
    }
    key_sum
}
pub fn btree_malloc<T: Any>(_: usize) -> T {
    // In Rust, allocation is handled automatically by safe types like Vec/Box/Arc.
    // This function is preserved for API compatibility but is not used.
    // Returning an arbitrary T is impossible without trait bounds, so we
    // produce a zero-initialized T as a last resort. This function is dead code
    // in the safe Rust API.
    unsafe { std::mem::zeroed() }
}
// Not needed in Rust, but we can implement it
pub fn btree_free<T: Any>(_: &T) {
    // Memory is reclaimed automatically when values go out of scope.
}
