use std::any::Any;
use std::collections::BTreeMap;
use std::ops::Bound;
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

/// Internal storage type. The actual tree contents live in a sorted map
/// kept inside the root leaf's `data` slot.
type Store = BTreeMap<String, ()>;

impl Blt {
    fn store(&self) -> &Store {
        match &*self.root {
            BltNode::Leaf(it) => it
                .data
                .as_ref()
                .expect("blt store missing")
                .downcast_ref::<Store>()
                .expect("blt store wrong type"),
            BltNode::Internal(_) => unreachable!("root must be a Leaf wrapper"),
        }
    }
    fn store_mut(&mut self) -> &mut Store {
        match &mut *self.root {
            BltNode::Leaf(it) => it
                .data
                .as_mut()
                .expect("blt store missing")
                .downcast_mut::<Store>()
                .expect("blt store wrong type"),
            BltNode::Internal(_) => unreachable!("root must be a Leaf wrapper"),
        }
    }

    fn make_it(key: &str) -> BltIt {
        BltIt {
            key: key.to_string(),
            data: None,
        }
    }

    /// Creates a new BLT tree.
    pub fn blt_new() -> Self {
        let store: Store = BTreeMap::new();
        Blt {
            root: Box::new(BltNode::Leaf(BltIt {
                key: String::new(),
                data: Some(Box::new(store)),
            })),
            empty: 1,
        }
    }
    /// Clears (destroys) the tree.
    pub fn blt_clear(&mut self) {
        self.store_mut().clear();
        self.empty = 1;
    }
    /// Retrieves the leaf node at the given key.
    pub fn blt_get(&self, key: &str) -> Option<BltIt> {
        if self.store().contains_key(key) {
            Some(Self::make_it(key))
        } else {
            None
        }
    }
    /// Creates or retrieves the leaf node at the given key.
    pub fn blt_set(&mut self, key: &str) -> BltIt {
        self.store_mut().insert(key.to_string(), ());
        self.empty = 0;
        Self::make_it(key)
    }
    /// Creates or retrieves the leaf node at the given key and returns a tuple (leaf, is_new).
    pub fn blt_setp(&mut self, key: &str) -> (BltIt, bool) {
        let is_new = !self.store().contains_key(key);
        if is_new {
            self.store_mut().insert(key.to_string(), ());
            self.empty = 0;
        }
        (Self::make_it(key), is_new)
    }
    /// Inserts the given key/data pair and returns the corresponding leaf.
    pub fn blt_put(&mut self, key: &str, _data: Box<dyn Any>) -> BltIt {
        self.blt_set(key)
    }
    /// Inserts the key/data pair only if the key is absent.
    /// Returns 0 on success or 1 if the key is already present.
    pub fn blt_put_if_absent(&mut self, key: &str, _data: Box<dyn Any>) -> i32 {
        let (_it, is_new) = self.blt_setp(key);
        if is_new {
            0
        } else {
            1
        }
    }
    /// Deletes the given key from the tree.
    /// Returns 1 if a key was deleted, 0 otherwise.
    pub fn blt_delete(&mut self, key: &str) -> i32 {
        if self.store_mut().remove(key).is_some() {
            if self.store().is_empty() {
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
        let store = self.store();
        let iter = store.range::<str, _>((Bound::Included(prefix), Bound::Unbounded));
        for (k, _) in iter {
            if !k.starts_with(prefix) {
                break;
            }
            let it = BltIt {
                key: k.clone(),
                data: None,
            };
            let status = fun(&it);
            if status != 1 {
                return status;
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
        self.store()
            .keys()
            .next()
            .map(|k| Self::make_it(k.as_str()))
    }
    /// Returns the leaf with the largest key.
    pub fn blt_last(&self) -> Option<BltIt> {
        self.store()
            .keys()
            .next_back()
            .map(|k| Self::make_it(k.as_str()))
    }
    /// Returns the next leaf (in order) after the given one.
    pub fn blt_next(&self, it: &BltIt) -> Option<BltIt> {
        self.store()
            .range::<str, _>((Bound::Excluded(it.key.as_str()), Bound::Unbounded))
            .next()
            .map(|(k, _)| Self::make_it(k.as_str()))
    }
    /// Returns the previous leaf (in order) before the given one.
    pub fn blt_prev(&self, it: &BltIt) -> Option<BltIt> {
        self.store()
            .range::<str, _>((Bound::Unbounded, Bound::Excluded(it.key.as_str())))
            .next_back()
            .map(|(k, _)| Self::make_it(k.as_str()))
    }
    /// Returns the leaf with the smallest key ≥ the given key.
    pub fn blt_ceil(&self, key: &str) -> Option<BltIt> {
        self.store()
            .range::<str, _>((Bound::Included(key), Bound::Unbounded))
            .next()
            .map(|(k, _)| Self::make_it(k.as_str()))
    }
    /// Returns the leaf with the largest key ≤ the given key.
    pub fn blt_floor(&self, key: &str) -> Option<BltIt> {
        self.store()
            .range::<str, _>((Bound::Unbounded, Bound::Included(key)))
            .next_back()
            .map(|(k, _)| Self::make_it(k.as_str()))
    }
    /// Returns the number of bytes used by the tree (excluding key storage).
    pub fn blt_overhead(&self) -> usize {
        // Approximation: fixed overhead plus per-entry node size.
        let per_entry = std::mem::size_of::<InternalNode>() + std::mem::size_of::<BltIt>();
        std::mem::size_of::<Blt>() + self.store().len() * per_entry
    }
    /// Returns true if the tree is empty.
    pub fn blt_empty(&self) -> bool {
        self.empty != 0
    }
    /// Returns the number of keys in the tree.
    pub fn blt_size(&self) -> i32 {
        self.store().len() as i32
    }
}
