use std::any::Any;
use std::sync::Arc;

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
            Some(root) => {
                if let Some(root_mut) = Arc::get_mut(root) {
                    root_mut.add_node(n_node);
                }
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
        let Some(root) = self.node.take() else {
            return;
        };

        let key_len = min_size(BTREE_KEY_SIZE, min_size(key_len, key.len()));
        let key_hash = calc_key_hash(key, key_len);
        self.node = delete_node_impl(Some(root), key_hash, key, key_len);
    }

    pub fn get_entry_count(&self) -> usize {
        match &self.node {
            Some(node) => node.get_node_count(),
            None => 0,
        }
    }

    pub fn find_entry(&self, key: &Vec<u8>, key_len: usize) -> Option<Value> {
        let node = self.node.as_ref()?;
        let key_len = min_size(BTREE_KEY_SIZE, min_size(key_len, key.len()));
        let key_hash = calc_key_hash(key, key_len);
        node.find_value(key_hash, key.clone(), key_len)
    }

    pub fn free_tree(&mut self) {
        if let Some(mut node) = self.node.take() {
            if let Some(node_mut) = Arc::get_mut(&mut node) {
                node_mut.free_node();
            }
        }
    }
}

impl Node {
    pub fn new_node(key: Vec<u8>, key_len: usize, value: Vec<u8>, value_len: usize) -> Arc<Self> {
        let stored_key_len = min_size(BTREE_KEY_SIZE, min_size(key_len, key.len()));
        let stored_value_len = min_size(value_len, value.len());
        let mut p_key = [0u8; BTREE_KEY_SIZE];
        p_key[..stored_key_len].copy_from_slice(&key[..stored_key_len]);

        let stored_value = value[..stored_value_len].to_vec();
        let key_hash = calc_key_hash(&p_key[..stored_key_len].to_vec(), stored_key_len);

        Arc::new(Node {
            key_hash,
            p_key,
            key_len: stored_key_len,
            value: Value {
                value: stored_value,
                len: stored_value_len,
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
                Some(child) => {
                    if let Some(child_mut) = Arc::get_mut(child) {
                        child_mut.add_node(n_node);
                    }
                }
            }
            return;
        }

        if n_node.key_hash == self.key_hash
            && keys_equal_prefix(&self.p_key, &n_node.p_key, n_node.key_len)
        {
            self.value = n_node.value.clone();
            return;
        }

        match self.child_left.as_mut() {
            None => {
                self.child_left = Some(n_node);
            }
            Some(child) => {
                if let Some(child_mut) = Arc::get_mut(child) {
                    child_mut.add_node(n_node);
                }
            }
        }
    }

    pub fn free_node(&mut self) {
        if let Some(mut child) = self.child_left.take() {
            if let Some(child_mut) = Arc::get_mut(&mut child) {
                child_mut.free_node();
            }
        }

        if let Some(mut child) = self.child_right.take() {
            if let Some(child_mut) = Arc::get_mut(&mut child) {
                child_mut.free_node();
            }
        }

        self.value.value.clear();
        self.value.len = 0;
        self.key_len = 0;
        self.key_hash = 0;
        self.p_key = [0u8; BTREE_KEY_SIZE];
    }

    pub fn delete_node(root: &mut Arc<Node>, key_hash: u32, key: Vec<u8>, key_len: usize) -> Option<Arc<Node>> {
        let updated = delete_node_impl(Some(Arc::clone(root)), key_hash, &key, key_len);
        if let Some(new_root) = &updated {
            *root = Arc::clone(new_root);
        }
        updated
    }

    pub fn get_node_count(&self) -> usize {
        1 + self.child_left.as_ref().map_or(0, |child| child.get_node_count())
            + self.child_right.as_ref().map_or(0, |child| child.get_node_count())
    }

    pub fn list_node_entries(&self, list: &mut EntryList) {
        if let Some(child) = &self.child_left {
            child.list_node_entries(list);
        }

        if list.len >= list.cap {
            return;
        }

        list.entries.push(Entry {
            key: BTreeKey {
                key: self.p_key[..self.key_len].to_vec(),
                len: self.key_len,
            },
            value: self.value.clone(),
        });
        list.len += 1;

        if let Some(child) = &self.child_right {
            child.list_node_entries(list);
        }
    }

    pub fn find_value(&self, key_hash: u32, key: Vec<u8>, key_len: usize) -> Option<Value> {
        let key_len = min_size(BTREE_KEY_SIZE, min_size(key_len, key.len()));
        if self.key_hash == key_hash && keys_equal_prefix(&self.p_key, &key, key_len) {
            return Some(self.value.clone());
        }

        if key_hash > self.key_hash {
            return self
                .child_right
                .as_ref()
                .and_then(|child| child.find_value(key_hash, key, key_len));
        }

        self.child_left
            .as_ref()
            .and_then(|child| child.find_value(key_hash, key, key_len))
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
    let mut key_sum = 0u32;
    let key_len = min_size(key_len, key.len());
    for (i, byte) in key.iter().copied().take(key_len).enumerate() {
        key_sum %= u32::MAX;
        key_sum = key_sum.wrapping_add(((byte as u32) * ((i + 1) as u32)) % u32::MAX);
    }
    key_sum
}

pub fn btree_malloc<T: Any>(_: usize) -> T {
    let type_id = std::any::TypeId::of::<T>();

    if type_id == std::any::TypeId::of::<BTree>() {
        let value: Box<dyn Any> = Box::new(BTree::new_btree());
        if let Ok(value) = value.downcast::<T>() {
            return *value;
        }
    }

    if type_id == std::any::TypeId::of::<EntryList>() {
        let value: Box<dyn Any> = Box::new(EntryList {
            entries: Vec::new(),
            len: 0,
            cap: 0,
        });
        if let Ok(value) = value.downcast::<T>() {
            return *value;
        }
    }

    if type_id == std::any::TypeId::of::<Value>() {
        let value: Box<dyn Any> = Box::new(Value {
            value: Vec::new(),
            len: 0,
        });
        if let Ok(value) = value.downcast::<T>() {
            return *value;
        }
    }

    if type_id == std::any::TypeId::of::<BTreeKey>() {
        let value: Box<dyn Any> = Box::new(BTreeKey {
            key: Vec::new(),
            len: 0,
        });
        if let Ok(value) = value.downcast::<T>() {
            return *value;
        }
    }

    if type_id == std::any::TypeId::of::<Node>() {
        let value: Box<dyn Any> = Box::new(Node {
            key_hash: 0,
            p_key: [0u8; BTREE_KEY_SIZE],
            key_len: 0,
            value: Value {
                value: Vec::new(),
                len: 0,
            },
            child_left: None,
            child_right: None,
        });
        if let Ok(value) = value.downcast::<T>() {
            return *value;
        }
    }

    loop {
        std::thread::park();
    }
}

pub fn btree_free<T: Any>(_: &T) {}

fn keys_equal_prefix(stored_key: &[u8], other_key: &[u8], len: usize) -> bool {
    let len = min_size(len, min_size(stored_key.len(), other_key.len()));
    stored_key[..len] == other_key[..len]
}

fn delete_node_impl(
    root: Option<Arc<Node>>,
    key_hash: u32,
    key: &[u8],
    key_len: usize,
) -> Option<Arc<Node>> {
    let mut root = root?;
    let key_len = min_size(BTREE_KEY_SIZE, min_size(key_len, key.len()));
    let root_mut = Arc::get_mut(&mut root)?;

    if key_hash < root_mut.key_hash {
        let left = root_mut.child_left.take();
        root_mut.child_left = delete_node_impl(left, key_hash, key, key_len);
        return Some(root);
    }

    if key_hash > root_mut.key_hash {
        let right = root_mut.child_right.take();
        root_mut.child_right = delete_node_impl(right, key_hash, key, key_len);
        return Some(root);
    }

    if !keys_equal_prefix(&root_mut.p_key, key, key_len) {
        return Some(root);
    }

    if root_mut.child_left.is_none() {
        return root_mut.child_right.take();
    }

    if root_mut.child_right.is_none() {
        return root_mut.child_left.take();
    }

    let successor = leftmost_node(root_mut.child_right.as_ref().expect("right child exists"));
    root_mut.key_hash = successor.key_hash;
    root_mut.p_key = successor.p_key;
    root_mut.key_len = successor.key_len;
    root_mut.value = successor.value.clone();

    let successor_key = successor.p_key[..successor.key_len].to_vec();
    let right = root_mut.child_right.take();
    root_mut.child_right = delete_node_impl(right, successor.key_hash, &successor_key, successor.key_len);
    Some(root)
}

fn leftmost_node(node: &Arc<Node>) -> NodeSnapshot {
    let mut current = node.as_ref();
    while let Some(left) = current.child_left.as_ref() {
        current = left.as_ref();
    }

    NodeSnapshot {
        key_hash: current.key_hash,
        p_key: current.p_key,
        key_len: current.key_len,
        value: current.value.clone(),
    }
}

struct NodeSnapshot {
    key_hash: u32,
    p_key: [u8; BTREE_KEY_SIZE],
    key_len: usize,
    value: Value,
}
