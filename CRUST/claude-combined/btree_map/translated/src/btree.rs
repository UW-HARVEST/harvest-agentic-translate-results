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
        match &mut self.node {
            None => {
                self.node = Some(n_node);
            }
            Some(root) => {
                let root_mut = Arc::get_mut(root).expect("BTree node should have unique ownership");
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
            Some(n) => n.get_node_count(),
        }
    }
    pub fn find_entry(&self, key: &Vec<u8>, key_len: usize) -> Option<Value> {
        let node = self.node.as_ref()?;
        let key_len = min_size(BTREE_KEY_SIZE, key_len);
        let key_hash = calc_key_hash(key, key_len);
        node.find_value(key_hash, key.clone(), key_len)
    }
    pub fn free_tree(&mut self) {
        self.node = None;
    }
}
impl Node {
    pub fn new_node(key: Vec<u8>, key_len: usize, value: Vec<u8>, value_len: usize) -> Arc<Self> {
        let key_len = min_size(BTREE_KEY_SIZE, key_len);
        let mut p_key = [0u8; BTREE_KEY_SIZE];
        let copy_len = key_len.min(key.len());
        p_key[..copy_len].copy_from_slice(&key[..copy_len]);

        let pk_vec: Vec<u8> = p_key.to_vec();
        let key_hash = calc_key_hash(&pk_vec, key_len);

        let mut value_vec = vec![0u8; value_len];
        let v_copy_len = value_len.min(value.len());
        value_vec[..v_copy_len].copy_from_slice(&value[..v_copy_len]);

        Arc::new(Node {
            key_hash,
            p_key,
            key_len,
            value: Value {
                value: value_vec,
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
            let right_mut = Arc::get_mut(right).expect("Node should have unique ownership");
            right_mut.add_node(n_node);
            return;
        }

        if n_node.key_hash == self.key_hash {
            let cmp_len = n_node.key_len;
            if self.p_key[..cmp_len] == n_node.p_key[..cmp_len] {
                // Update existing value
                self.value = Value {
                    value: n_node.value.value.clone(),
                    len: n_node.value.len,
                };
                return;
            }
        }

        if self.child_left.is_none() {
            self.child_left = Some(n_node);
            return;
        }
        let left = self.child_left.as_mut().unwrap();
        let left_mut = Arc::get_mut(left).expect("Node should have unique ownership");
        left_mut.add_node(n_node);
    }
    pub fn free_node(&mut self) {
        self.child_left = None;
        self.child_right = None;
    }
    pub fn delete_node(
        root: &mut Arc<Node>,
        key_hash: u32,
        key: Vec<u8>,
        key_len: usize,
    ) -> Option<Arc<Node>> {
        let key_len = min_size(BTREE_KEY_SIZE, key_len);
        let root_hash = root.key_hash;

        if key_hash < root_hash {
            let root_mut = Arc::get_mut(root).expect("Node should have unique ownership");
            if let Some(mut left) = root_mut.child_left.take() {
                root_mut.child_left = Node::delete_node(&mut left, key_hash, key, key_len);
            }
        } else if key_hash > root_hash {
            let root_mut = Arc::get_mut(root).expect("Node should have unique ownership");
            if let Some(mut right) = root_mut.child_right.take() {
                root_mut.child_right = Node::delete_node(&mut right, key_hash, key, key_len);
            }
        } else {
            // Hash matches; check actual key bytes
            let matches = root.p_key[..key_len] == key[..key_len];
            if matches {
                let root_mut = Arc::get_mut(root).expect("Node should have unique ownership");
                if root_mut.child_left.is_none() {
                    return root_mut.child_right.take();
                }
                if root_mut.child_right.is_none() {
                    return root_mut.child_left.take();
                }

                // Two children: find inorder successor (leftmost of right subtree)
                let (s_hash, s_pkey, s_klen, s_value) = {
                    let mut current: &Arc<Node> = root_mut.child_right.as_ref().unwrap();
                    while let Some(next) = current.child_left.as_ref() {
                        current = next;
                    }
                    (
                        current.key_hash,
                        current.p_key,
                        current.key_len,
                        current.value.clone(),
                    )
                };

                root_mut.key_hash = s_hash;
                root_mut.p_key = s_pkey;
                root_mut.key_len = s_klen;
                root_mut.value = s_value;

                if let Some(mut right) = root_mut.child_right.take() {
                    let succ_key_vec: Vec<u8> = s_pkey[..s_klen].to_vec();
                    root_mut.child_right =
                        Node::delete_node(&mut right, s_hash, succ_key_vec, s_klen);
                }
            }
        }

        Some(Arc::clone(root))
    }
    pub fn get_node_count(&self) -> usize {
        let mut count = 1usize;
        if let Some(left) = &self.child_left {
            count += left.get_node_count();
        }
        if let Some(right) = &self.child_right {
            count += right.get_node_count();
        }
        count
    }
    pub fn list_node_entries(&self, list: &mut EntryList) {
        if let Some(left) = &self.child_left {
            left.list_node_entries(list);
        }

        if list.len >= list.cap {
            return;
        }

        let mut key_vec = vec![0u8; self.key_len];
        key_vec.copy_from_slice(&self.p_key[..self.key_len]);

        let entry = Entry {
            key: BTreeKey {
                key: key_vec,
                len: self.key_len,
            },
            value: Value {
                value: self.value.value.clone(),
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
            if self.p_key[..key_len] == key[..key_len] {
                return Some(Value {
                    value: self.value.value.clone(),
                    len: self.value.len,
                });
            }
        }

        if key_hash > self.key_hash {
            if let Some(right) = &self.child_right {
                return right.find_value(key_hash, key, key_len);
            }
            return None;
        }

        if let Some(left) = &self.child_left {
            return left.find_value(key_hash, key, key_len);
        }
        None
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
    let len = key_len.min(key.len());
    for i in 0..len {
        key_sum = key_sum % u32::MAX;
        let byte = key[i] as u32;
        let mult = (i as u32).wrapping_add(1);
        let prod = byte.wrapping_mul(mult) % u32::MAX;
        key_sum = key_sum.wrapping_add(prod);
    }
    key_sum
}
pub fn btree_malloc<T: Any>(_: usize) -> T {
    // Memory allocation is handled automatically by Rust's ownership system
    // (Vec, Box, Arc handle their own allocation). This function is not used
    // by the Rust implementation and serves only as a stub for the C API.
    unsafe { std::mem::zeroed() }
}
// Not needed in Rust, but we can implement it
pub fn btree_free<T: Any>(_: &T) {
    // Memory deallocation is handled automatically by Rust's Drop trait.
    // This is a no-op.
}
