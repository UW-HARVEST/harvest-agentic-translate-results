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
        Arc::get_mut(root)
            .expect("BTree root has unique ownership")
            .add_node(n_node);
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
        if self.node.is_none() {
            return;
        }
        let key_len_clamped = min_size(BTREE_KEY_SIZE, key_len);
        let key_for_hash: Vec<u8> = key.iter().take(key_len_clamped).copied().collect();
        let key_hash = calc_key_hash(&key_for_hash, key_len_clamped);

        let mut root = self.node.take().unwrap();
        self.node = Node::delete_node(&mut root, key_hash, key.clone(), key_len);
    }
    pub fn get_entry_count(&self) -> usize {
        match &self.node {
            Some(root) => root.get_node_count(),
            None => 0,
        }
    }
    pub fn find_entry(&self, key: &Vec<u8>, key_len: usize) -> Option<Value> {
        let root = self.node.as_ref()?;
        let key_len_clamped = min_size(BTREE_KEY_SIZE, key_len);
        let key_for_hash: Vec<u8> = key.iter().take(key_len_clamped).copied().collect();
        let key_hash = calc_key_hash(&key_for_hash, key_len_clamped);
        root.find_value(key_hash, key.clone(), key_len)
    }
    pub fn free_tree(&mut self) {
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

        let key_for_hash: Vec<u8> = p_key[..actual_key_len].to_vec();
        let key_hash = calc_key_hash(&key_for_hash, actual_key_len);

        let mut value_vec: Vec<u8> = Vec::with_capacity(value_len);
        for i in 0..value_len {
            value_vec.push(value[i]);
        }

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
            let child = self.child_right.as_mut().unwrap();
            Arc::get_mut(child)
                .expect("right child has unique ownership")
                .add_node(n_node);
            return;
        }

        if n_node.key_hash == self.key_hash {
            let cmp_len = n_node.key_len;
            if self.p_key[..cmp_len] == n_node.p_key[..cmp_len] {
                self.value.value = n_node.value.value.clone();
                self.value.len = n_node.value.len;
                return;
            }
        }

        if self.child_left.is_none() {
            self.child_left = Some(n_node);
            return;
        }
        let child = self.child_left.as_mut().unwrap();
        Arc::get_mut(child)
            .expect("left child has unique ownership")
            .add_node(n_node);
    }
    pub fn free_node(&mut self) {
        // Children are dropped automatically via Arc when set to None.
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

        let root_mut = Arc::get_mut(root).expect("delete_node root has unique ownership");

        if key_hash < root_mut.key_hash {
            if let Some(mut left) = root_mut.child_left.take() {
                root_mut.child_left = Node::delete_node(&mut left, key_hash, key, key_len);
            }
        } else if key_hash > root_mut.key_hash {
            if let Some(mut right) = root_mut.child_right.take() {
                root_mut.child_right = Node::delete_node(&mut right, key_hash, key, key_len);
            }
        } else {
            // matching hash; verify key
            let cmp_len = std::cmp::min(key_len, root_mut.key_len);
            let key_matches = root_mut.p_key[..cmp_len] == key[..cmp_len];
            if key_matches {
                // Node with no left child or no right child
                if root_mut.child_left.is_none() {
                    return root_mut.child_right.take();
                }
                if root_mut.child_right.is_none() {
                    return root_mut.child_left.take();
                }

                // Two children: find inorder successor (leftmost of right subtree)
                let (succ_hash, succ_p_key, succ_key_len, succ_val, succ_val_len) = {
                    let mut current: &Arc<Node> = root_mut.child_right.as_ref().unwrap();
                    while let Some(left) = current.child_left.as_ref() {
                        current = left;
                    }
                    (
                        current.key_hash,
                        current.p_key,
                        current.key_len,
                        current.value.value.clone(),
                        current.value.len,
                    )
                };

                root_mut.key_hash = succ_hash;
                root_mut.p_key = succ_p_key;
                root_mut.key_len = succ_key_len;
                root_mut.value.value = succ_val;
                root_mut.value.len = succ_val_len;

                let succ_key_vec = succ_p_key[..succ_key_len].to_vec();
                if let Some(mut right) = root_mut.child_right.take() {
                    root_mut.child_right =
                        Node::delete_node(&mut right, succ_hash, succ_key_vec, succ_key_len);
                }
            }
        }

        Some(root.clone())
    }
    pub fn get_node_count(&self) -> usize {
        let left = self.child_left.as_ref().map_or(0, |n| n.get_node_count());
        let right = self
            .child_right
            .as_ref()
            .map_or(0, |n| n.get_node_count());
        1 + left + right
    }
    pub fn list_node_entries(&self, list: &mut EntryList) {
        if let Some(left) = self.child_left.as_ref() {
            left.list_node_entries(list);
        }

        if list.len >= list.cap {
            return;
        }

        let entry = Entry {
            key: BTreeKey {
                key: self.p_key[..self.key_len].to_vec(),
                len: self.key_len,
            },
            value: Value {
                value: self.value.value.clone(),
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
        if self.key_hash == key_hash {
            if self.p_key[..key_len] == key[..key_len] {
                return Some(self.value.clone());
            }
        }

        if key_hash > self.key_hash {
            match self.child_right.as_ref() {
                Some(right) => right.find_value(key_hash, key, key_len),
                None => None,
            }
        } else {
            match self.child_left.as_ref() {
                Some(left) => left.find_value(key_hash, key, key_len),
                None => None,
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
    // Mirrors the C implementation:
    //   for i in 0..key_len:
    //       key_sum = key_sum % UINT32_MAX;
    //       key_sum += (byte_key[i] * (i + 1)) % UINT32_MAX;
    // For typical inputs (small bytes/indices), the modulo operations are no-ops
    // and the addition naturally wraps around as uint32_t. We use wrapping_add
    // to match that overflow behavior.
    let mut key_sum: u32 = 0;
    for i in 0..key_len {
        let byte_val = key[i] as u32;
        let multiplier = (i as u32).wrapping_add(1);
        let product = byte_val.wrapping_mul(multiplier);
        key_sum = key_sum.wrapping_add(product);
    }
    key_sum
}
pub fn btree_malloc<T: Any>(_: usize) -> T {
    // Rust manages memory automatically; this helper exists only to mirror the
    // C API and is not called from the Rust implementation. We cannot construct
    // an arbitrary `T: Any` from a size, so this is a placeholder for a
    // deliberately-unreachable code path.
    loop {}
}
// Not needed in Rust, but we can implement it
pub fn btree_free<T: Any>(_: &T) {
    // Rust drops values automatically when they go out of scope, so freeing is
    // a no-op here.
}
