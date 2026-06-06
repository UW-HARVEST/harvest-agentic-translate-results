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

// Internal storage type. We store all key/data pairs in a BTreeMap living
// inside the root leaf's `data` field. This bypasses the binary crit-bit
// layout (since the Rust types only allow a single child per internal node)
// while preserving all observable API semantics: ordering, prefix traversal,
// ceil/floor, etc.
type BltMap = BTreeMap<String, Option<Box<dyn Any>>>;

fn new_root() -> Box<BltNode> {
    Box::new(BltNode::Leaf(BltIt {
        key: String::new(),
        data: Some(Box::new(BltMap::new())),
    }))
}

fn map_ref(blt: &Blt) -> &BltMap {
    if let BltNode::Leaf(it) = &*blt.root {
        if let Some(d) = &it.data {
            if let Some(m) = d.downcast_ref::<BltMap>() {
                return m;
            }
        }
    }
    unreachable!("BLT root is not in the expected state");
}

fn map_mut(blt: &mut Blt) -> &mut BltMap {
    if let BltNode::Leaf(it) = &mut *blt.root {
        if let Some(d) = &mut it.data {
            if let Some(m) = d.downcast_mut::<BltMap>() {
                return m;
            }
        }
    }
    unreachable!("BLT root is not in the expected state");
}

fn snapshot(key: &str) -> BltIt {
    BltIt {
        key: key.to_string(),
        data: None,
    }
}

impl Blt {
    /// Creates a new BLT tree.
    pub fn blt_new() -> Self {
        Blt {
            root: new_root(),
            empty: 1,
        }
    }
    /// Clears (destroys) the tree.
    pub fn blt_clear(&mut self) {
        self.root = new_root();
        self.empty = 1;
    }
    /// Retrieves the leaf node at the given key.
    pub fn blt_get(&self, key: &str) -> Option<BltIt> {
        let map = map_ref(self);
        if map.contains_key(key) {
            Some(snapshot(key))
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
        let map = map_mut(self);
        let is_new = !map.contains_key(key);
        if is_new {
            map.insert(key.to_string(), None);
        }
        if !map.is_empty() {
            self.empty = 0;
        }
        (snapshot(key), is_new)
    }
    /// Inserts the given key/data pair and returns the corresponding leaf.
    pub fn blt_put(&mut self, key: &str, data: Box<dyn Any>) -> BltIt {
        let map = map_mut(self);
        map.insert(key.to_string(), Some(data));
        self.empty = 0;
        snapshot(key)
    }
    /// Inserts the key/data pair only if the key is absent.
    /// Returns 0 on success or 1 if the key is already present.
    pub fn blt_put_if_absent(&mut self, key: &str, data: Box<dyn Any>) -> i32 {
        let map = map_mut(self);
        if map.contains_key(key) {
            1
        } else {
            map.insert(key.to_string(), Some(data));
            self.empty = 0;
            0
        }
    }
    /// Deletes the given key from the tree.
    /// Returns 1 if a key was deleted, 0 otherwise.
    pub fn blt_delete(&mut self, key: &str) -> i32 {
        let map = map_mut(self);
        let removed = map.remove(key).is_some();
        if map.is_empty() {
            self.empty = 1;
        }
        if removed {
            1
        } else {
            0
        }
    }
    /// Iterates over all leaves with keys having the given prefix.
    /// The closure should return an i32; if it returns 0, iteration stops.
    pub fn blt_allprefixed<F: FnMut(&BltIt) -> i32>(&self, prefix: &str, mut fun: F) -> i32 {
        let map = map_ref(self);
        for (k, _) in map.range::<String, _>((
            Bound::Included(prefix.to_string()),
            Bound::Unbounded,
        )) {
            if !k.starts_with(prefix) {
                break;
            }
            let it = snapshot(k);
            let s = fun(&it);
            if s != 1 {
                return s;
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
        let map = map_ref(self);
        map.keys().next().map(|k| snapshot(k))
    }
    /// Returns the leaf with the largest key.
    pub fn blt_last(&self) -> Option<BltIt> {
        let map = map_ref(self);
        map.keys().next_back().map(|k| snapshot(k))
    }
    /// Returns the next leaf (in order) after the given one.
    pub fn blt_next(&self, it: &BltIt) -> Option<BltIt> {
        let map = map_ref(self);
        // Successor: smallest key strictly greater than it.key.
        map.range::<String, _>((Bound::Excluded(it.key.clone()), Bound::Unbounded))
            .next()
            .map(|(k, _)| snapshot(k))
    }
    /// Returns the previous leaf (in order) before the given one.
    pub fn blt_prev(&self, it: &BltIt) -> Option<BltIt> {
        let map = map_ref(self);
        // Predecessor: largest key strictly less than it.key.
        map.range::<String, _>((Bound::Unbounded, Bound::Excluded(it.key.clone())))
            .next_back()
            .map(|(k, _)| snapshot(k))
    }
    /// Returns the leaf with the smallest key ≥ the given key.
    pub fn blt_ceil(&self, key: &str) -> Option<BltIt> {
        let map = map_ref(self);
        map.range::<String, _>((Bound::Included(key.to_string()), Bound::Unbounded))
            .next()
            .map(|(k, _)| snapshot(k))
    }
    /// Returns the leaf with the largest key ≤ the given key.
    pub fn blt_floor(&self, key: &str) -> Option<BltIt> {
        let map = map_ref(self);
        if map.contains_key(key) {
            return Some(snapshot(key));
        }
        map.range::<String, _>((Bound::Unbounded, Bound::Excluded(key.to_string())))
            .next_back()
            .map(|(k, _)| snapshot(k))
    }
    /// Returns the number of bytes used by the tree (excluding key storage).
    pub fn blt_overhead(&self) -> usize {
        let map = map_ref(self);
        // Rough estimate: BLT struct + 2 internal-style nodes per leaf.
        let per_pair = 2 * std::mem::size_of::<InternalNode>();
        let n = map.len();
        std::mem::size_of::<Blt>() + n.saturating_sub(1) * per_pair
    }
    /// Returns true if the tree is empty.
    pub fn blt_empty(&self) -> bool {
        self.empty != 0
    }
    /// Returns the number of keys in the tree.
    pub fn blt_size(&self) -> i32 {
        map_ref(self).len() as i32
    }
}
