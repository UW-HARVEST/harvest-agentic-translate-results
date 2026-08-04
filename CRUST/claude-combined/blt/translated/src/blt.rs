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

// Internal storage: we store the actual sorted map inside the root leaf's
// `data` field. This lets us satisfy the public struct definitions while
// providing efficient BTreeMap-backed operations under the hood.
type Storage = BTreeMap<String, Box<dyn Any>>;

fn storage_ref(node: &BltNode) -> &Storage {
    match node {
        BltNode::Leaf(it) => it
            .data
            .as_ref()
            .expect("blt: missing internal storage")
            .downcast_ref::<Storage>()
            .expect("blt: storage type mismatch"),
        _ => panic!("blt: root must be a leaf in this representation"),
    }
}

fn storage_mut(node: &mut BltNode) -> &mut Storage {
    match node {
        BltNode::Leaf(it) => it
            .data
            .as_mut()
            .expect("blt: missing internal storage")
            .downcast_mut::<Storage>()
            .expect("blt: storage type mismatch"),
        _ => panic!("blt: root must be a leaf in this representation"),
    }
}

fn make_view(key: &str) -> BltIt {
    BltIt {
        key: key.to_string(),
        data: None,
    }
}

impl Blt {
    /// Creates a new BLT tree.
    pub fn blt_new() -> Self {
        let storage: Storage = BTreeMap::new();
        Blt {
            root: Box::new(BltNode::Leaf(BltIt {
                key: String::new(),
                data: Some(Box::new(storage)),
            })),
            empty: 1,
        }
    }
    /// Clears (destroys) the tree.
    pub fn blt_clear(&mut self) {
        let m = storage_mut(&mut self.root);
        m.clear();
        self.empty = 1;
    }
    /// Retrieves the leaf node at the given key.
    pub fn blt_get(&self, key: &str) -> Option<BltIt> {
        let m = storage_ref(&self.root);
        if m.contains_key(key) {
            Some(make_view(key))
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
        let is_new = {
            let m = storage_mut(&mut self.root);
            if m.contains_key(key) {
                false
            } else {
                m.insert(key.to_string(), Box::new(()) as Box<dyn Any>);
                true
            }
        };
        if is_new {
            self.empty = 0;
        }
        (make_view(key), is_new)
    }
    /// Inserts the given key/data pair and returns the corresponding leaf.
    pub fn blt_put(&mut self, key: &str, data: Box<dyn Any>) -> BltIt {
        {
            let m = storage_mut(&mut self.root);
            m.insert(key.to_string(), data);
        }
        self.empty = 0;
        make_view(key)
    }
    /// Inserts the key/data pair only if the key is absent.
    /// Returns 0 on success or 1 if the key is already present.
    pub fn blt_put_if_absent(&mut self, key: &str, data: Box<dyn Any>) -> i32 {
        let already_present = {
            let m = storage_mut(&mut self.root);
            if m.contains_key(key) {
                true
            } else {
                m.insert(key.to_string(), data);
                false
            }
        };
        if !already_present {
            self.empty = 0;
            0
        } else {
            1
        }
    }
    /// Deletes the given key from the tree.
    /// Returns 1 if a key was deleted, 0 otherwise.
    pub fn blt_delete(&mut self, key: &str) -> i32 {
        let (removed, is_empty) = {
            let m = storage_mut(&mut self.root);
            let r = m.remove(key).is_some();
            (r, m.is_empty())
        };
        if removed && is_empty {
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
        let m = storage_ref(&self.root);
        let start = prefix.to_string();
        for (k, _) in m.range::<String, _>((Bound::Included(start), Bound::Unbounded)) {
            if !k.starts_with(prefix) {
                break;
            }
            let view = make_view(k);
            let r = fun(&view);
            if r != 1 {
                return r;
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
        let m = storage_ref(&self.root);
        m.keys().next().map(|k| make_view(k))
    }
    /// Returns the leaf with the largest key.
    pub fn blt_last(&self) -> Option<BltIt> {
        let m = storage_ref(&self.root);
        m.keys().next_back().map(|k| make_view(k))
    }
    /// Returns the next leaf (in order) after the given one.
    pub fn blt_next(&self, it: &BltIt) -> Option<BltIt> {
        let m = storage_ref(&self.root);
        m.range::<str, _>((Bound::Excluded(it.key.as_str()), Bound::Unbounded))
            .next()
            .map(|(k, _)| make_view(k))
    }
    /// Returns the previous leaf (in order) before the given one.
    pub fn blt_prev(&self, it: &BltIt) -> Option<BltIt> {
        let m = storage_ref(&self.root);
        m.range::<str, _>((Bound::Unbounded, Bound::Excluded(it.key.as_str())))
            .next_back()
            .map(|(k, _)| make_view(k))
    }
    /// Returns the leaf with the smallest key ≥ the given key.
    pub fn blt_ceil(&self, key: &str) -> Option<BltIt> {
        let m = storage_ref(&self.root);
        m.range::<str, _>((Bound::Included(key), Bound::Unbounded))
            .next()
            .map(|(k, _)| make_view(k))
    }
    /// Returns the leaf with the largest key ≤ the given key.
    pub fn blt_floor(&self, key: &str) -> Option<BltIt> {
        let m = storage_ref(&self.root);
        m.range::<str, _>((Bound::Unbounded, Bound::Included(key)))
            .next_back()
            .map(|(k, _)| make_view(k))
    }
    /// Returns the number of bytes used by the tree (excluding key storage).
    pub fn blt_overhead(&self) -> usize {
        let m = storage_ref(&self.root);
        // Approximate the C overhead: BLT struct + 2 * sizeof(node) per
        // internal node. For a tree with N leaves, there are N-1 internal
        // nodes. Each internal node represents a sibling pair in the C code.
        let n = m.len();
        let internal_nodes = if n > 1 { n - 1 } else { 0 };
        std::mem::size_of::<Blt>() + internal_nodes * 2 * std::mem::size_of::<InternalNode>()
    }
    /// Returns true if the tree is empty.
    pub fn blt_empty(&self) -> bool {
        self.empty != 0
    }
    /// Returns the number of keys in the tree.
    pub fn blt_size(&self) -> i32 {
        let m = storage_ref(&self.root);
        m.len() as i32
    }
}
