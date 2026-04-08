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
        match self.node {
            None => self.node = Some(n_node),
            Some(ref mut root) => {
                Arc::get_mut(root).unwrap().add_node(n_node);
            }
        }
    }
    pub fn list_entries(&self) -> EntryList {
        let cap = self.get_entry_count();
        let mut list = EntryList {
            entries: Vec::new(),
            len: 0,
            cap,
        };
        if let Some(ref node) = self.node {
            node.list_node_entries(&mut list);
        }
        list
    }
    pub fn remove_entry(&mut self, key: &Vec<u8>, key_len: usize) {
        let key_len = min_size(BTREE_KEY_SIZE, key_len);
        let truncated_key: Vec<u8> = key[..key_len].to_vec();
        let key_hash = calc_key_hash(&truncated_key, key_len);
        if let Some(root) = self.node.take() {
            self.node = Node::delete_node(&mut Arc::clone(&root), key_hash, truncated_key, key_len);
        }
    }
    pub fn get_entry_count(&self) -> usize {
        match self.node {
            None => 0,
            Some(ref node) => node.get_node_count(),
        }
    }
    pub fn find_entry(&self, key: &Vec<u8>, key_len: usize) -> Option<Value> {
        let key_len = min_size(BTREE_KEY_SIZE, key_len);
        let truncated_key: Vec<u8> = key[..key_len].to_vec();
        let key_hash = calc_key_hash(&truncated_key, key_len);
        match self.node {
            None => None,
            Some(ref node) => node.find_value(key_hash, truncated_key, key_len),
        }
    }
    pub fn free_tree(&mut self) {
        self.node = None;
    }
}
impl Node {
    pub fn new_node(key: Vec<u8>, key_len: usize, value: Vec<u8>, value_len: usize) -> Arc<Self> {
        let actual_key_len = min_size(BTREE_KEY_SIZE, key_len);
        let mut p_key = [0u8; BTREE_KEY_SIZE];
        p_key[..actual_key_len].copy_from_slice(&key[..actual_key_len]);
        let key_hash = calc_key_hash(&p_key[..actual_key_len].to_vec(), actual_key_len);
        let val = value[..value_len].to_vec();
        Arc::new(Node {
            key_hash,
            p_key,
            key_len: actual_key_len,
            value: Value { value: val, len: value_len },
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
            Arc::get_mut(self.child_right.as_mut().unwrap()).unwrap().add_node(n_node);
            return;
        }
        if n_node.key_hash == self.key_hash {
            if self.p_key[..n_node.key_len] == n_node.p_key[..n_node.key_len] {
                self.value.len = n_node.value.len;
                self.value.value = n_node.value.value.clone();
                return;
            }
        }
        if self.child_left.is_none() {
            self.child_left = Some(n_node);
            return;
        }
        Arc::get_mut(self.child_left.as_mut().unwrap()).unwrap().add_node(n_node);
    }
    pub fn free_node(&mut self) {
        self.child_left = None;
        self.child_right = None;
    }
    pub fn delete_node(root: &mut Arc<Node>, key_hash: u32, key: Vec<u8>, key_len: usize) -> Option<Arc<Node>> {
        // We need to rebuild the tree since Arc doesn't allow easy in-place mutation in all cases
        let root_ref = Arc::as_ref(root);
        let key_len = min_size(BTREE_KEY_SIZE, key_len);

        if key_hash < root_ref.key_hash {
            let mut new_node = Node {
                key_hash: root_ref.key_hash,
                p_key: root_ref.p_key,
                key_len: root_ref.key_len,
                value: root_ref.value.clone(),
                child_left: root_ref.child_left.clone(),
                child_right: root_ref.child_right.clone(),
            };
            if let Some(ref mut left) = new_node.child_left {
                new_node.child_left = Node::delete_node(left, key_hash, key, key_len);
            }
            Some(Arc::new(new_node))
        } else if key_hash > root_ref.key_hash {
            let mut new_node = Node {
                key_hash: root_ref.key_hash,
                p_key: root_ref.p_key,
                key_len: root_ref.key_len,
                value: root_ref.value.clone(),
                child_left: root_ref.child_left.clone(),
                child_right: root_ref.child_right.clone(),
            };
            if let Some(ref mut right) = new_node.child_right {
                new_node.child_right = Node::delete_node(right, key_hash, key, key_len);
            }
            Some(Arc::new(new_node))
        } else {
            // key_hash == root_ref.key_hash
            if root_ref.p_key[..key_len] == key[..key_len] {
                // Found the node to delete
                if root_ref.child_left.is_none() {
                    return root_ref.child_right.clone();
                } else if root_ref.child_right.is_none() {
                    return root_ref.child_left.clone();
                }
                // Two children: find inorder successor (leftmost in right subtree)
                let successor = find_min(root_ref.child_right.as_ref().unwrap());
                let mut new_node = Node {
                    key_hash: successor.key_hash,
                    p_key: successor.p_key,
                    key_len: successor.key_len,
                    value: successor.value.clone(),
                    child_left: root_ref.child_left.clone(),
                    child_right: root_ref.child_right.clone(),
                };
                let succ_key = successor.p_key[..successor.key_len].to_vec();
                let succ_key_len = successor.key_len;
                let succ_hash = successor.key_hash;
                if let Some(ref mut right) = new_node.child_right {
                    new_node.child_right = Node::delete_node(right, succ_hash, succ_key, succ_key_len);
                }
                Some(Arc::new(new_node))
            } else {
                // Hash collision but different key — same as key_hash < (go left in C code)
                // In the C code, when hash matches but key doesn't, it just returns root unchanged
                Some(root.clone())
            }
        }
    }
    pub fn get_node_count(&self) -> usize {
        let left = match self.child_left {
            Some(ref n) => n.get_node_count(),
            None => 0,
        };
        let right = match self.child_right {
            Some(ref n) => n.get_node_count(),
            None => 0,
        };
        1 + left + right
    }
    pub fn list_node_entries(&self, list: &mut EntryList) {
        if let Some(ref left) = self.child_left {
            left.list_node_entries(list);
        }
        if list.len >= list.cap {
            return;
        }
        let mut key_copy = vec![0u8; self.key_len];
        key_copy.copy_from_slice(&self.p_key[..self.key_len]);
        let entry = Entry {
            key: BTreeKey { key: key_copy, len: self.key_len },
            value: Value { value: self.value.value.clone(), len: self.value.len },
        };
        list.entries.push(entry);
        list.len += 1;
        if let Some(ref right) = self.child_right {
            right.list_node_entries(list);
        }
    }
    pub fn find_value(&self, key_hash: u32, key: Vec<u8>, key_len: usize) -> Option<Value> {
        let key_len = min_size(BTREE_KEY_SIZE, key_len);
        if self.key_hash == key_hash {
            if self.p_key[..key_len] == key[..key_len] {
                return Some(self.value.clone());
            }
        }
        if key_hash > self.key_hash {
            return match self.child_right {
                None => None,
                Some(ref right) => right.find_value(key_hash, key, key_len),
            };
        }
        match self.child_left {
            None => None,
            Some(ref left) => left.find_value(key_hash, key, key_len),
        }
    }
}

fn find_min(node: &Arc<Node>) -> &Node {
    let mut current = node.as_ref();
    while let Some(ref left) = current.child_left {
        current = left.as_ref();
    }
    current
}

pub fn min_size(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}
pub fn calc_key_hash(key: &Vec<u8>, key_len: usize) -> u32 {
    let mut key_sum: u32 = 0;
    for i in 0..key_len {
        key_sum = key_sum % u32::MAX;
        key_sum = key_sum.wrapping_add((key[i] as u32).wrapping_mul((i as u32) + 1) % u32::MAX);
    }
    key_sum
}
pub fn btree_malloc<T: Any>(_: usize) -> T {
    // Not meaningful in Rust — memory is managed automatically
    panic!("btree_malloc is not used in Rust")
}
// Not needed in Rust, but we can implement it
pub fn btree_free<T: Any>(_: &T) {
    // No-op in Rust — memory is managed automatically
}
