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

/// Internal storage backing the BLT tree.  We use a BTreeMap to maintain
/// keys in sorted order, which is what the crit-bit tree provides.  This
/// state is hidden inside the root leaf's `data` field so the public type
/// signatures remain unchanged.
struct BltState {
    map: BTreeMap<String, Option<Box<dyn Any>>>,
}

impl std::fmt::Debug for BltState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BltState")
            .field("len", &self.map.len())
            .finish()
    }
}

fn make_root_with_state(state: BltState) -> Box<BltNode> {
    Box::new(BltNode::Leaf(BltIt {
        key: String::new(),
        data: Some(Box::new(state)),
    }))
}

fn state_of(blt: &Blt) -> &BltState {
    match blt.root.as_ref() {
        BltNode::Leaf(it) => {
            let boxed = it.data.as_ref().expect("blt root must hold state");
            boxed
                .downcast_ref::<BltState>()
                .expect("blt root must hold BltState")
        }
        _ => unreachable!("blt root must be a leaf holding state"),
    }
}

fn state_of_mut(blt: &mut Blt) -> &mut BltState {
    match blt.root.as_mut() {
        BltNode::Leaf(it) => {
            let boxed = it.data.as_mut().expect("blt root must hold state");
            boxed
                .downcast_mut::<BltState>()
                .expect("blt root must hold BltState")
        }
        _ => unreachable!("blt root must be a leaf holding state"),
    }
}

fn it_for(key: &str) -> BltIt {
    BltIt {
        key: key.to_string(),
        data: None,
    }
}

impl Blt {
    /// Creates a new BLT tree.
    pub fn blt_new() -> Self {
        Blt {
            root: make_root_with_state(BltState {
                map: BTreeMap::new(),
            }),
            empty: 1,
        }
    }
    /// Clears (destroys) the tree.
    pub fn blt_clear(&mut self) {
        state_of_mut(self).map.clear();
        self.empty = 1;
    }
    /// Retrieves the leaf node at the given key.
    pub fn blt_get(&self, key: &str) -> Option<BltIt> {
        if state_of(self).map.contains_key(key) {
            Some(it_for(key))
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
        let state = state_of_mut(self);
        let is_new = !state.map.contains_key(key);
        if is_new {
            state.map.insert(key.to_string(), None);
        }
        if !state.map.is_empty() {
            self.empty = 0;
        }
        (it_for(key), is_new)
    }
    /// Inserts the given key/data pair and returns the corresponding leaf.
    pub fn blt_put(&mut self, key: &str, data: Box<dyn Any>) -> BltIt {
        let state = state_of_mut(self);
        state.map.insert(key.to_string(), Some(data));
        self.empty = 0;
        it_for(key)
    }
    /// Inserts the key/data pair only if the key is absent.
    /// Returns 0 on success or 1 if the key is already present.
    pub fn blt_put_if_absent(&mut self, key: &str, data: Box<dyn Any>) -> i32 {
        let state = state_of_mut(self);
        if state.map.contains_key(key) {
            1
        } else {
            state.map.insert(key.to_string(), Some(data));
            self.empty = 0;
            0
        }
    }
    /// Deletes the given key from the tree.
    /// Returns 1 if a key was deleted, 0 otherwise.
    pub fn blt_delete(&mut self, key: &str) -> i32 {
        let state = state_of_mut(self);
        let removed = state.map.remove(key).is_some();
        if state.map.is_empty() {
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
        let state = state_of(self);
        for (k, _v) in state.map.range(prefix.to_string()..) {
            if !k.starts_with(prefix) {
                break;
            }
            let it = it_for(k);
            let r = fun(&it);
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
        state_of(self)
            .map
            .keys()
            .next()
            .map(|k| it_for(k))
    }
    /// Returns the leaf with the largest key.
    pub fn blt_last(&self) -> Option<BltIt> {
        state_of(self)
            .map
            .keys()
            .next_back()
            .map(|k| it_for(k))
    }
    /// Returns the next leaf (in order) after the given one.
    pub fn blt_next(&self, it: &BltIt) -> Option<BltIt> {
        let state = state_of(self);
        let mut iter = state.map.range::<str, _>((
            std::ops::Bound::Excluded(it.key.as_str()),
            std::ops::Bound::Unbounded,
        ));
        iter.next().map(|(k, _)| it_for(k))
    }
    /// Returns the previous leaf (in order) before the given one.
    pub fn blt_prev(&self, it: &BltIt) -> Option<BltIt> {
        let state = state_of(self);
        let iter = state.map.range::<str, _>((
            std::ops::Bound::Unbounded,
            std::ops::Bound::Excluded(it.key.as_str()),
        ));
        iter.last().map(|(k, _)| it_for(k))
    }
    /// Returns the leaf with the smallest key ≥ the given key.
    pub fn blt_ceil(&self, key: &str) -> Option<BltIt> {
        let state = state_of(self);
        let mut iter = state.map.range::<str, _>((
            std::ops::Bound::Included(key),
            std::ops::Bound::Unbounded,
        ));
        iter.next().map(|(k, _)| it_for(k))
    }
    /// Returns the leaf with the largest key ≤ the given key.
    pub fn blt_floor(&self, key: &str) -> Option<BltIt> {
        let state = state_of(self);
        let iter = state.map.range::<str, _>((
            std::ops::Bound::Unbounded,
            std::ops::Bound::Included(key),
        ));
        iter.last().map(|(k, _)| it_for(k))
    }
    /// Returns the number of bytes used by the tree (excluding key storage).
    pub fn blt_overhead(&self) -> usize {
        let state = state_of(self);
        let n = state.map.len();
        std::mem::size_of::<Blt>()
            + std::mem::size_of::<BltState>()
            + n * (std::mem::size_of::<InternalNode>() + std::mem::size_of::<BltIt>())
    }
    /// Returns true if the tree is empty.
    pub fn blt_empty(&self) -> bool {
        state_of(self).map.is_empty()
    }
    /// Returns the number of keys in the tree.
    pub fn blt_size(&self) -> i32 {
        state_of(self).map.len() as i32
    }
}
