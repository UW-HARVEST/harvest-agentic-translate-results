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
        let root_arc = self.node.as_mut().unwrap();
        let root_mut = Arc::get_mut(root_arc).expect("unique ownership of root");
        root_mut.add_node(n_node);
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
        let taken = self.node.take();
        self.node = delete_node_helper(taken, key_hash, key, key_len);
    }
    pub fn get_entry_count(&self) -> usize {
        match &self.node {
            Some(n) => n.get_node_count(),
            None => 0,
        }
    }
    pub fn find_entry(&self, key: &Vec<u8>, key_len: usize) -> Option<Value> {
        let root = self.node.as_ref()?;
        let key_len = min_size(BTREE_KEY_SIZE, key_len);
        let key_hash = calc_key_hash(key, key_len);
        root.find_value(key_hash, key.clone(), key_len)
    }
    pub fn free_tree(&mut self) {
        // In Rust, dropping the Arc tree handles deallocation.
        self.node = None;
    }
}
impl Node {
    pub fn new_node(key: Vec<u8>, key_len: usize, value: Vec<u8>, value_len: usize) -> Arc<Self> {
        let actual_key_len = min_size(BTREE_KEY_SIZE, key_len);
        let mut p_key = [0u8; BTREE_KEY_SIZE];
        for i in 0..actual_key_len {
            p_key[i] = key[i];
        }
        let mut value_vec = vec![0u8; value_len];
        for i in 0..value_len {
            if i < value.len() {
                value_vec[i] = value[i];
            }
        }
        let p_key_for_hash: Vec<u8> = p_key[..actual_key_len].to_vec();
        let key_hash = calc_key_hash(&p_key_for_hash, actual_key_len);
        Arc::new(Node {
            key_hash,
            p_key,
            key_len: actual_key_len,
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
            let right_mut =
                Arc::get_mut(self.child_right.as_mut().unwrap()).expect("unique right child");
            right_mut.add_node(n_node);
            return;
        }
        if n_node.key_hash == self.key_hash {
            let cmp_len = n_node.key_len;
            if cmp_len <= BTREE_KEY_SIZE && self.p_key[..cmp_len] == n_node.p_key[..cmp_len] {
                // Update value (mirror C: copies n_node->value.len bytes into self.value.value
                // and sets self.value.len). To avoid out-of-bounds, mirror the visible result
                // by replacing the value entirely.
                self.value = n_node.value.clone();
                return;
            }
        }
        if self.child_left.is_none() {
            self.child_left = Some(n_node);
            return;
        }
        let left_mut = Arc::get_mut(self.child_left.as_mut().unwrap()).expect("unique left child");
        left_mut.add_node(n_node);
    }
    pub fn free_node(&mut self) {
        // In Rust, dropping the children Arcs handles freeing automatically.
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
        let owned = Arc::clone(root);
        delete_node_helper(Some(owned), key_hash, &key, key_len)
    }
    pub fn get_node_count(&self) -> usize {
        let left = self.child_left.as_ref().map_or(0, |n| n.get_node_count());
        let right = self.child_right.as_ref().map_or(0, |n| n.get_node_count());
        1 + left + right
    }
    pub fn list_node_entries(&self, list: &mut EntryList) {
        if let Some(left) = &self.child_left {
            left.list_node_entries(list);
        }
        if list.len >= list.cap {
            return;
        }
        let mut key_vec = vec![0u8; self.key_len];
        for i in 0..self.key_len {
            key_vec[i] = self.p_key[i];
        }
        let mut value_vec = vec![0u8; self.value.len];
        for i in 0..self.value.len {
            if i < self.value.value.len() {
                value_vec[i] = self.value.value[i];
            }
        }
        let entry = Entry {
            key: BTreeKey {
                key: key_vec,
                len: self.key_len,
            },
            value: Value {
                value: value_vec,
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
            if key_len <= BTREE_KEY_SIZE
                && key.len() >= key_len
                && self.p_key[..key_len] == key[..key_len]
            {
                return Some(self.value.clone());
            }
        }
        if key_hash > self.key_hash {
            return self
                .child_right
                .as_ref()?
                .find_value(key_hash, key, key_len);
        }
        self.child_left
            .as_ref()?
            .find_value(key_hash, key, key_len)
    }
}

fn delete_node_helper(
    node: Option<Arc<Node>>,
    key_hash: u32,
    key: &Vec<u8>,
    key_len: usize,
) -> Option<Arc<Node>> {
    let mut node_arc = node?;
    let n = Arc::get_mut(&mut node_arc).expect("unique ownership during delete");
    if key_hash < n.key_hash {
        let left = n.child_left.take();
        n.child_left = delete_node_helper(left, key_hash, key, key_len);
        return Some(node_arc);
    } else if key_hash > n.key_hash {
        let right = n.child_right.take();
        n.child_right = delete_node_helper(right, key_hash, key, key_len);
        return Some(node_arc);
    }
    // key_hash equal
    let cmp_len = key_len;
    let key_matches = cmp_len <= BTREE_KEY_SIZE
        && key.len() >= cmp_len
        && n.p_key[..cmp_len] == key[..cmp_len];
    if key_matches {
        if n.child_left.is_none() {
            return n.child_right.take();
        } else if n.child_right.is_none() {
            return n.child_left.take();
        }
        // Two children: find inorder successor (leftmost of right subtree).
        let (succ_hash, succ_p_key, succ_key_len, succ_val_vec, succ_val_len) = {
            let right = n.child_right.as_ref().unwrap();
            let mut cur: &Arc<Node> = right;
            while let Some(left) = &cur.child_left {
                cur = left;
            }
            (
                cur.key_hash,
                cur.p_key,
                cur.key_len,
                cur.value.value.clone(),
                cur.value.len,
            )
        };
        n.key_hash = succ_hash;
        n.p_key = succ_p_key;
        n.key_len = succ_key_len;
        n.value = Value {
            value: succ_val_vec,
            len: succ_val_len,
        };
        // Build the successor's key vec from its p_key for the recursive deletion.
        let succ_key_vec: Vec<u8> = succ_p_key[..succ_key_len].to_vec();
        let right_subtree = n.child_right.take();
        n.child_right = delete_node_helper(right_subtree, succ_hash, &succ_key_vec, succ_key_len);
        return Some(node_arc);
    }
    Some(node_arc)
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
    for i in 0..key_len {
        // Mirror C: key_sum = key_sum % UINT32_MAX (no-op for u32 except when == UINT32_MAX);
        // and the term is (byte * (i+1)) % UINT32_MAX.
        if key_sum == u32::MAX {
            key_sum = 0;
        }
        let byte = key[i] as u64;
        let factor = (i as u64).wrapping_add(1);
        let mut term = (byte.wrapping_mul(factor)) as u64;
        if term >= u32::MAX as u64 {
            term = term % (u32::MAX as u64);
        }
        let term32 = term as u32;
        key_sum = key_sum.wrapping_add(term32);
    }
    key_sum
}
pub fn btree_malloc<T: Any>(_: usize) -> T {
    // Mirrors C's malloc + memset(0). Requires the type to be safe for zero-initialization.
    // Used internally by C; in Rust we typically don't call this for the public API.
    unsafe { std::mem::zeroed() }
}
// Not needed in Rust, but we can implement it
pub fn btree_free<T: Any>(_: &T) {
    // No-op: Rust handles deallocation automatically through Drop.
}
