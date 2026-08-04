use std::any::Any;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::mem::size_of;

thread_local! {
    static BLT_STORE: RefCell<BTreeMap<usize, BTreeMap<String, Option<Box<dyn Any>>>>> =
        RefCell::new(BTreeMap::new());
}

fn clone_any(value: &dyn Any) -> Option<Box<dyn Any>> {
    macro_rules! clone_copy {
        ($($ty:ty),* $(,)?) => {
            $(
                if let Some(v) = value.downcast_ref::<$ty>() {
                    return Some(Box::new(*v));
                }
            )*
        };
    }

    clone_copy!(
        (),
        bool,
        char,
        i8,
        i16,
        i32,
        i64,
        i128,
        isize,
        u8,
        u16,
        u32,
        u64,
        u128,
        usize,
        f32,
        f64,
    );

    if let Some(v) = value.downcast_ref::<String>() {
        return Some(Box::new(v.clone()));
    }
    if let Some(v) = value.downcast_ref::<Vec<u8>>() {
        return Some(Box::new(v.clone()));
    }
    if let Some(v) = value.downcast_ref::<Vec<String>>() {
        return Some(Box::new(v.clone()));
    }

    None
}

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
impl Blt {
    fn tree_id(&self) -> usize {
        (&*self.root) as *const BltNode as usize
    }

    fn with_store<R>(&self, f: impl FnOnce(&BTreeMap<String, Option<Box<dyn Any>>>) -> R) -> R {
        BLT_STORE.with(|store| {
            let store = store.borrow();
            let empty = BTreeMap::new();
            let map = store.get(&self.tree_id()).unwrap_or(&empty);
            f(map)
        })
    }

    fn with_store_mut<R>(
        &mut self,
        f: impl FnOnce(&mut BTreeMap<String, Option<Box<dyn Any>>>) -> R,
    ) -> R {
        BLT_STORE.with(|store| {
            let mut store = store.borrow_mut();
            let map = store.entry(self.tree_id()).or_default();
            f(map)
        })
    }

    fn leaf_from_entry(key: &str, data: &Option<Box<dyn Any>>) -> BltIt {
        BltIt {
            key: key.to_string(),
            data: data.as_deref().and_then(clone_any),
        }
    }

    fn leaf_by_key(&self, key: &str) -> Option<BltIt> {
        self.with_store(|map| map.get(key).map(|data| Self::leaf_from_entry(key, data)))
    }

    fn nth_neighbor(&self, key: &str, forward: bool) -> Option<BltIt> {
        self.with_store(|map| {
            let mut iter = map.range(key.to_string()..);
            if forward {
                let first = iter.next()?;
                if first.0 == key {
                    let next = iter.next()?;
                    return Some(Self::leaf_from_entry(next.0, next.1));
                }
                return None;
            }

            let mut iter = map.range(..=key.to_string());
            let last = iter.next_back()?;
            if last.0 == key {
                let prev = iter.next_back()?;
                Some(Self::leaf_from_entry(prev.0, prev.1))
            } else {
                None
            }
        })
    }

    /// Creates a new BLT tree.
    pub fn blt_new() -> Self {
        let tree = Self {
            root: Box::new(BltNode::Leaf(BltIt {
                key: String::new(),
                data: None,
            })),
            empty: 1,
        };
        BLT_STORE.with(|store| {
            store.borrow_mut().insert(tree.tree_id(), BTreeMap::new());
        });
        tree
    }
    /// Clears (destroys) the tree.
    pub fn blt_clear(&mut self) {
        BLT_STORE.with(|store| {
            store.borrow_mut().remove(&self.tree_id());
        });
        self.empty = 1;
    }
    /// Retrieves the leaf node at the given key.
    pub fn blt_get(&self, key: &str) -> Option<BltIt> {
        self.leaf_by_key(key)
    }
    /// Creates or retrieves the leaf node at the given key.
    pub fn blt_set(&mut self, key: &str) -> BltIt {
        self.blt_setp(key).0
    }
    /// Creates or retrieves the leaf node at the given key and returns a tuple (leaf, is_new).
    pub fn blt_setp(&mut self, key: &str) -> (BltIt, bool) {
        let mut is_new = false;
        self.with_store_mut(|map| {
            if !map.contains_key(key) {
                map.insert(key.to_string(), None);
                is_new = true;
            }
        });
        self.empty = if self.blt_size() == 0 { 1 } else { 0 };
        (
            self.leaf_by_key(key).unwrap_or(BltIt {
                key: key.to_string(),
                data: None,
            }),
            is_new,
        )
    }
    /// Inserts the given key/data pair and returns the corresponding leaf.
    pub fn blt_put(&mut self, key: &str, data: Box<dyn Any>) -> BltIt {
        self.with_store_mut(|map| {
            map.insert(key.to_string(), Some(data));
        });
        self.empty = 0;
        self.leaf_by_key(key).unwrap_or(BltIt {
            key: key.to_string(),
            data: None,
        })
    }
    /// Inserts the key/data pair only if the key is absent.
    /// Returns 0 on success or 1 if the key is already present.
    pub fn blt_put_if_absent(&mut self, key: &str, data: Box<dyn Any>) -> i32 {
        let mut was_present = false;
        self.with_store_mut(|map| {
            if map.contains_key(key) {
                was_present = true;
            } else {
                map.insert(key.to_string(), Some(data));
            }
        });
        self.empty = if self.blt_size() == 0 { 1 } else { 0 };
        i32::from(was_present)
    }
    /// Deletes the given key from the tree.
    /// Returns 1 if a key was deleted, 0 otherwise.
    pub fn blt_delete(&mut self, key: &str) -> i32 {
        let removed = self.with_store_mut(|map| map.remove(key).is_some());
        self.empty = if self.blt_size() == 0 { 1 } else { 0 };
        i32::from(removed)
    }
    /// Iterates over all leaves with keys having the given prefix.
    /// The closure should return an i32; if it returns 0, iteration stops.
    pub fn blt_allprefixed<F: FnMut(&BltIt) -> i32>(&self, prefix: &str, mut fun: F) -> i32 {
        self.with_store(|map| {
            for (key, data) in map.range(prefix.to_string()..) {
                if !key.starts_with(prefix) {
                    break;
                }
                let leaf = Self::leaf_from_entry(key, data);
                let status = fun(&leaf);
                if status != 1 {
                    return status;
                }
            }
            1
        })
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
        self.with_store(|map| {
            map.first_key_value()
                .map(|(k, v)| Self::leaf_from_entry(k, v))
        })
    }
    /// Returns the leaf with the largest key.
    pub fn blt_last(&self) -> Option<BltIt> {
        self.with_store(|map| {
            map.last_key_value()
                .map(|(k, v)| Self::leaf_from_entry(k, v))
        })
    }
    /// Returns the next leaf (in order) after the given one.
    pub fn blt_next(&self, it: &BltIt) -> Option<BltIt> {
        self.nth_neighbor(&it.key, true)
    }
    /// Returns the previous leaf (in order) before the given one.
    pub fn blt_prev(&self, it: &BltIt) -> Option<BltIt> {
        self.nth_neighbor(&it.key, false)
    }
    /// Returns the leaf with the smallest key ≥ the given key.
    pub fn blt_ceil(&self, key: &str) -> Option<BltIt> {
        self.with_store(|map| {
            map.range(key.to_string()..)
                .next()
                .map(|(k, v)| Self::leaf_from_entry(k, v))
        })
    }
    /// Returns the leaf with the largest key ≤ the given key.
    pub fn blt_floor(&self, key: &str) -> Option<BltIt> {
        self.with_store(|map| {
            map.range(..=key.to_string())
                .next_back()
                .map(|(k, v)| Self::leaf_from_entry(k, v))
        })
    }
    /// Returns the number of bytes used by the tree (excluding key storage).
    pub fn blt_overhead(&self) -> usize {
        let size = self.blt_size() as usize;
        size_of::<Blt>() + size.saturating_sub(1) * 2 * size_of::<InternalNode>()
    }
    /// Returns true if the tree is empty.
    pub fn blt_empty(&self) -> bool {
        self.empty != 0
    }
    /// Returns the number of keys in the tree.
    pub fn blt_size(&self) -> i32 {
        self.with_store(|map| map.len() as i32)
    }
}

impl Drop for Blt {
    fn drop(&mut self) {
        BLT_STORE.with(|store| {
            store.borrow_mut().remove(&self.tree_id());
        });
    }
}
