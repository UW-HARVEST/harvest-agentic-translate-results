use std::collections::HashMap;
use std::sync::Arc;
use std::any::{Any, TypeId};
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
        if let Some(root) = self.node.as_mut() {
            let root = ensure_unique_node(root);
            root.add_node(n_node);
        } else {
            self.node = Some(n_node);
        }
    }
    pub fn list_entries(&self) -> EntryList {
        let mut list = EntryList {
            entries: Vec::new(),
            len: 0,
            cap: self.get_entry_count(),
        };

        if let Some(node) = self.node.as_ref() {
            node.list_node_entries(&mut list);
        }

        list
    }
    pub fn remove_entry(&mut self, key: &Vec<u8>, key_len: usize) {
        let key_len = min_size(BTREE_KEY_SIZE, min_size(key_len, key.len()));
        let key_hash = calc_key_hash(key, key_len);

        if let Some(root) = self.node.take() {
            self.node = delete_node_owned(root, key_hash, &key[..key_len], key_len);
        }
    }
    pub fn get_entry_count(&self) -> usize {
        self.node.as_ref().map_or(0, |node| node.get_node_count())
    }
    pub fn find_entry(&self, key: &Vec<u8>, key_len: usize) -> Option<Value> {
        let key_len = min_size(BTREE_KEY_SIZE, min_size(key_len, key.len()));
        let key_hash = calc_key_hash(key, key_len);
        self.node
            .as_ref()
            .and_then(|node| node.find_value(key_hash, key[..key_len].to_vec(), key_len))
    }
    pub fn free_tree(&mut self) {
        self.node = None;
    }
}
impl Node {
    pub fn new_node(key: Vec<u8>, key_len: usize, value: Vec<u8>, value_len: usize) -> Arc<Self> {
        let key_len = min_size(BTREE_KEY_SIZE, min_size(key_len, key.len()));
        let value_len = min_size(value_len, value.len());
        let mut p_key = [0u8; BTREE_KEY_SIZE];
        p_key[..key_len].copy_from_slice(&key[..key_len]);

        Arc::new(Node {
            key_hash: calc_key_hash(&p_key[..key_len].to_vec(), key_len),
            p_key,
            key_len,
            value: Value {
                value: value[..value_len].to_vec(),
                len: value_len,
            },
            child_left: None,
            child_right: None,
        })
    }
    pub fn add_node(&mut self, n_node: Arc<Node>) {
        if n_node.key_hash > self.key_hash {
            if let Some(child) = self.child_right.as_mut() {
                ensure_unique_node(child).add_node(n_node);
            } else {
                self.child_right = Some(n_node);
            }
            return;
        }

        if n_node.key_hash == self.key_hash
            && self.p_key[..n_node.key_len] == n_node.p_key[..n_node.key_len]
        {
            self.value = Value {
                value: n_node.value.value.clone(),
                len: n_node.value.len,
            };
            return;
        }

        if let Some(child) = self.child_left.as_mut() {
            ensure_unique_node(child).add_node(n_node);
        } else {
            self.child_left = Some(n_node);
        }
    }
    pub fn free_node(&mut self) {
        if let Some(child) = self.child_left.as_mut() {
            ensure_unique_node(child).free_node();
        }
        if let Some(child) = self.child_right.as_mut() {
            ensure_unique_node(child).free_node();
        }
        self.child_left = None;
        self.child_right = None;
        self.value.value.clear();
        self.value.len = 0;
    }
    pub fn delete_node(root: &mut Arc<Node>, key_hash: u32, key: Vec<u8>, key_len: usize) -> Option<Arc<Node>> {
        let key_len = min_size(BTREE_KEY_SIZE, min_size(key_len, key.len()));
        delete_node_owned(Arc::clone(root), key_hash, &key[..key_len], key_len)
    }
    pub fn get_node_count(&self) -> usize {
        1 + self.child_left.as_ref().map_or(0, |node| node.get_node_count())
            + self.child_right.as_ref().map_or(0, |node| node.get_node_count())
    }
    pub fn list_node_entries(&self, list: &mut EntryList) {
        if let Some(child) = self.child_left.as_ref() {
            child.list_node_entries(list);
        }

        if list.len < list.cap {
            list.entries.push(Entry {
                key: BTreeKey {
                    key: self.p_key[..self.key_len].to_vec(),
                    len: self.key_len,
                },
                value: self.value.clone(),
            });
            list.len += 1;
        }

        if let Some(child) = self.child_right.as_ref() {
            child.list_node_entries(list);
        }
    }
    pub fn find_value(&self, key_hash: u32, key: Vec<u8>, key_len: usize) -> Option<Value> {
        let key_len = min_size(BTREE_KEY_SIZE, min_size(key_len, key.len()));
        if self.key_hash == key_hash && self.p_key[..key_len] == key[..key_len] {
            return Some(self.value.clone());
        }

        if key_hash > self.key_hash {
            return self
                .child_right
                .as_ref()
                .and_then(|node| node.find_value(key_hash, key, key_len));
        }

        self.child_left
            .as_ref()
            .and_then(|node| node.find_value(key_hash, key, key_len))
    }
}
pub fn min_size(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}
pub fn calc_key_hash(key: &Vec<u8>, key_len: usize) -> u32 {
    let key_len = min_size(key_len, key.len());
    let mut key_sum = 0u32;
    for (idx, byte) in key.iter().take(key_len).enumerate() {
        key_sum %= u32::MAX;
        key_sum = key_sum.wrapping_add((u32::from(*byte) * (idx as u32 + 1)) % u32::MAX);
    }
    key_sum
}
pub fn btree_malloc<T: Any>(_: usize) -> T {
    let boxed: Box<dyn Any> = if TypeId::of::<T>() == TypeId::of::<BTree>() {
        Box::new(BTree::new_btree())
    } else if TypeId::of::<T>() == TypeId::of::<Node>() {
        Box::new(Node {
            key_hash: 0,
            p_key: [0; BTREE_KEY_SIZE],
            key_len: 0,
            value: Value {
                value: Vec::new(),
                len: 0,
            },
            child_left: None,
            child_right: None,
        })
    } else if TypeId::of::<T>() == TypeId::of::<Value>() {
        Box::new(Value {
            value: Vec::new(),
            len: 0,
        })
    } else if TypeId::of::<T>() == TypeId::of::<BTreeKey>() {
        Box::new(BTreeKey {
            key: Vec::new(),
            len: 0,
        })
    } else if TypeId::of::<T>() == TypeId::of::<Entry>() {
        Box::new(Entry {
            key: BTreeKey {
                key: Vec::new(),
                len: 0,
            },
            value: Value {
                value: Vec::new(),
                len: 0,
            },
        })
    } else if TypeId::of::<T>() == TypeId::of::<EntryList>() {
        Box::new(EntryList {
            entries: Vec::new(),
            len: 0,
            cap: 0,
        })
    } else if TypeId::of::<T>() == TypeId::of::<Vec<Entry>>() {
        Box::new(Vec::<Entry>::new())
    } else {
        std::process::abort();
    };

    if let Ok(value) = boxed.downcast::<T>() {
        *value
    } else {
        std::process::abort();
    }
}
// Not needed in Rust, but we can implement it
pub fn btree_free<T: Any>(_: &T) {
}

fn ensure_unique_node(node: &mut Arc<Node>) -> &mut Node {
    if Arc::get_mut(node).is_none() {
        *node = clone_node(node);
    }
    if let Some(inner) = Arc::get_mut(node) {
        inner
    } else {
        std::process::abort();
    }
}

fn clone_node(node: &Arc<Node>) -> Arc<Node> {
    Arc::new(Node {
        key_hash: node.key_hash,
        p_key: node.p_key,
        key_len: node.key_len,
        value: node.value.clone(),
        child_left: node.child_left.as_ref().map(clone_node),
        child_right: node.child_right.as_ref().map(clone_node),
    })
}

fn delete_node_owned(
    mut root: Arc<Node>,
    key_hash: u32,
    key: &[u8],
    key_len: usize,
) -> Option<Arc<Node>> {
    let key_len = min_size(BTREE_KEY_SIZE, min_size(key_len, key.len()));

    if key_hash < root.key_hash {
        let root_mut = ensure_unique_node(&mut root);
        if let Some(child) = root_mut.child_left.take() {
            root_mut.child_left = delete_node_owned(child, key_hash, key, key_len);
        }
        return Some(root);
    }

    if key_hash > root.key_hash {
        let root_mut = ensure_unique_node(&mut root);
        if let Some(child) = root_mut.child_right.take() {
            root_mut.child_right = delete_node_owned(child, key_hash, key, key_len);
        }
        return Some(root);
    }

    if root.p_key[..key_len] != key[..key_len] {
        return Some(root);
    }

    if root.child_left.is_none() {
        let root_mut = ensure_unique_node(&mut root);
        return root_mut.child_right.take();
    }

    if root.child_right.is_none() {
        let root_mut = ensure_unique_node(&mut root);
        return root_mut.child_left.take();
    }

    let Some(right_child) = root.child_right.as_ref() else {
        return Some(root);
    };
    let successor = find_min(right_child);
    let successor_key = successor.p_key[..successor.key_len].to_vec();
    let successor_value = successor.value.clone();
    let successor_hash = successor.key_hash;
    let successor_len = successor.key_len;

    let root_mut = ensure_unique_node(&mut root);
    root_mut.key_hash = successor_hash;
    root_mut.p_key[..successor_len].copy_from_slice(&successor_key);
    root_mut.key_len = successor_len;
    root_mut.value = successor_value;

    if let Some(child) = root_mut.child_right.take() {
        root_mut.child_right =
            delete_node_owned(child, successor_hash, &successor_key, successor_len);
    }

    Some(root)
}

fn find_min(mut node: &Arc<Node>) -> &Arc<Node> {
    while let Some(left) = node.child_left.as_ref() {
        node = left;
    }
    node
}
