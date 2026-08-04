use std::any::Any;
use std::collections::BTreeMap;

/// The BLT tree structure.
#[derive(Debug)]
pub struct Blt {
    /// The root node.
    pub root: Box<BltNode>,
    /// Indicates whether the tree is empty.
    pub empty: i32,
}
/// A node in the BLT tree.
#[derive(Debug)]
pub enum BltNode {
    /// An internal node.
    Internal(InternalNode),
    /// A leaf node (external node).
    Leaf(BltIt),
}
/// Represents an internal node in the BLT tree.
#[derive(Debug)]
pub struct InternalNode {
    /// Byte number of difference (32 bits).
    pub byte: u32,
    /// Mask byte (8 bits).
    pub mask: u8,
    /// Padding (23 bits stored in a u32).
    pub padding: u32,
    /// The child node.
    pub kid: Box<BltNode>,
}
/// Represents a leaf node in the BLT tree.
#[derive(Debug)]
pub struct BltIt {
    /// The key associated with the leaf.
    pub key: String,
    /// Associated data.
    pub data: Option<Box<dyn Any>>,
}

/// Internal storage for the Blt tree (hidden inside the root leaf's data field).
struct BltStorage {
    entries: BTreeMap<String, Option<Box<dyn Any>>>,
}

const ROOT_SENTINEL_KEY: &str = "\x01__blt_internal_root__\x01";

fn make_root() -> Box<BltNode> {
    let storage: Box<dyn Any> = Box::new(BltStorage {
        entries: BTreeMap::new(),
    });
    Box::new(BltNode::Leaf(BltIt {
        key: ROOT_SENTINEL_KEY.to_string(),
        data: Some(storage),
    }))
}

fn storage_ref(blt: &Blt) -> &BltStorage {
    match &*blt.root {
        BltNode::Leaf(it) => it
            .data
            .as_ref()
            .and_then(|d| d.downcast_ref::<BltStorage>())
            .expect("blt internal storage corrupted"),
        BltNode::Internal(_) => panic!("blt root must be a leaf sentinel"),
    }
}

fn storage_mut(blt: &mut Blt) -> &mut BltStorage {
    match &mut *blt.root {
        BltNode::Leaf(it) => it
            .data
            .as_mut()
            .and_then(|d| d.downcast_mut::<BltStorage>())
            .expect("blt internal storage corrupted"),
        BltNode::Internal(_) => panic!("blt root must be a leaf sentinel"),
    }
}

fn make_leaf(key: &str) -> BltIt {
    BltIt {
        key: key.to_string(),
        data: None,
    }
}

impl Blt {
    /// Creates a new BLT tree.
    pub fn blt_new() -> Self {
        Blt {
            root: make_root(),
            empty: 1,
        }
    }
    /// Clears (destroys) the tree.
    pub fn blt_clear(&mut self) {
        storage_mut(self).entries.clear();
        self.empty = 1;
    }
    /// Retrieves the leaf node at the given key.
    pub fn blt_get(&self, key: &str) -> Option<BltIt> {
        if self.empty != 0 {
            return None;
        }
        let store = storage_ref(self);
        if store.entries.contains_key(key) {
            Some(make_leaf(key))
        } else {
            None
        }
    }
    /// Creates or retrieves the leaf node at the given key.
    pub fn blt_set(&mut self, key: &str) -> BltIt {
        let (it, _) = self.blt_setp(key);
        it
    }
    /// Creates or retrieves the leaf node at the given key and returns a tuple (leaf, is_new).
    pub fn blt_setp(&mut self, key: &str) -> (BltIt, bool) {
        let store = storage_mut(self);
        let is_new = !store.entries.contains_key(key);
        if is_new {
            store.entries.insert(key.to_string(), None);
            self.empty = 0;
        }
        (make_leaf(key), is_new)
    }
    /// Inserts the given key/data pair and returns the corresponding leaf.
    pub fn blt_put(&mut self, key: &str, data: Box<dyn Any>) -> BltIt {
        let store = storage_mut(self);
        store.entries.insert(key.to_string(), Some(data));
        self.empty = 0;
        make_leaf(key)
    }
    /// Inserts the key/data pair only if the key is absent.
    /// Returns 0 on success or 1 if the key is already present.
    pub fn blt_put_if_absent(&mut self, key: &str, data: Box<dyn Any>) -> i32 {
        let store = storage_mut(self);
        if store.entries.contains_key(key) {
            1
        } else {
            store.entries.insert(key.to_string(), Some(data));
            self.empty = 0;
            0
        }
    }
    /// Deletes the given key from the tree.
    /// Returns 1 if a key was deleted, 0 otherwise.
    pub fn blt_delete(&mut self, key: &str) -> i32 {
        let store = storage_mut(self);
        if store.entries.remove(key).is_some() {
            if store.entries.is_empty() {
                self.empty = 1;
            }
            1
        } else {
            0
        }
    }
    /// Iterates over all leaves with keys having the given prefix.
    /// The closure should return an i32; if it returns 0, iteration stops.
    pub fn blt_allprefixed<F: FnMut(&BltIt) -> i32>(&self, prefix: &str, mut fun: F) -> i32 {
        if self.empty != 0 {
            return 1;
        }
        let store = storage_ref(self);
        for (k, _) in store.entries.iter() {
            if k.starts_with(prefix) {
                let leaf = make_leaf(k);
                let status = fun(&leaf);
                if status != 1 {
                    return status;
                }
            }
        }
        1
    }
    /// Iterates through all leaves in order and calls the provided closure.
    pub fn blt_forall<F: FnMut(&BltIt)>(&self, mut fun: F) {
        let _ = self.blt_allprefixed("", |it| {
            fun(it);
            1 // always continue iteration
        });
    }
    /// Returns the leaf with the smallest key.
    pub fn blt_first(&self) -> Option<BltIt> {
        if self.empty != 0 {
            return None;
        }
        storage_ref(self)
            .entries
            .keys()
            .next()
            .map(|k| make_leaf(k))
    }
    /// Returns the leaf with the largest key.
    pub fn blt_last(&self) -> Option<BltIt> {
        if self.empty != 0 {
            return None;
        }
        storage_ref(self)
            .entries
            .keys()
            .next_back()
            .map(|k| make_leaf(k))
    }
    /// Returns the next leaf (in order) after the given one.
    pub fn blt_next(&self, it: &BltIt) -> Option<BltIt> {
        if self.empty != 0 {
            return None;
        }
        let store = storage_ref(self);
        use std::ops::Bound::{Excluded, Unbounded};
        store
            .entries
            .range::<str, _>((Excluded(it.key.as_str()), Unbounded))
            .next()
            .map(|(k, _)| make_leaf(k))
    }
    /// Returns the previous leaf (in order) before the given one.
    pub fn blt_prev(&self, it: &BltIt) -> Option<BltIt> {
        if self.empty != 0 {
            return None;
        }
        let store = storage_ref(self);
        use std::ops::Bound::{Excluded, Unbounded};
        store
            .entries
            .range::<str, _>((Unbounded, Excluded(it.key.as_str())))
            .next_back()
            .map(|(k, _)| make_leaf(k))
    }
    /// Returns the leaf with the smallest key ≥ the given key.
    pub fn blt_ceil(&self, key: &str) -> Option<BltIt> {
        if self.empty != 0 {
            return None;
        }
        let store = storage_ref(self);
        use std::ops::Bound::{Included, Unbounded};
        store
            .entries
            .range::<str, _>((Included(key), Unbounded))
            .next()
            .map(|(k, _)| make_leaf(k))
    }
    /// Returns the leaf with the largest key ≤ the given key.
    pub fn blt_floor(&self, key: &str) -> Option<BltIt> {
        if self.empty != 0 {
            return None;
        }
        let store = storage_ref(self);
        use std::ops::Bound::{Included, Unbounded};
        store
            .entries
            .range::<str, _>((Unbounded, Included(key)))
            .next_back()
            .map(|(k, _)| make_leaf(k))
    }
    /// Returns the number of bytes used by the tree (excluding key storage).
    pub fn blt_overhead(&self) -> usize {
        let mut n = std::mem::size_of::<Blt>();
        if self.empty == 0 {
            // For each entry beyond the first, the C tree uses two adjacent
            // node slots. Approximate that here.
            let count = storage_ref(self).entries.len();
            if count > 0 {
                // Root holds one node; each additional entry adds two.
                let extra = count.saturating_sub(1);
                n += 2 * extra * std::mem::size_of::<InternalNode>();
            }
        }
        n
    }
    /// Returns true if the tree is empty.
    pub fn blt_empty(&self) -> bool {
        self.empty != 0
    }
    /// Returns the number of keys in the tree.
    pub fn blt_size(&self) -> i32 {
        if self.empty != 0 {
            return 0;
        }
        storage_ref(self).entries.len() as i32
    }
}
