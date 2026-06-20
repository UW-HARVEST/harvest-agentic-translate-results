use std::any::Any;
use std::collections::BTreeMap;
use std::ops::Bound::{Excluded, Unbounded};

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

#[derive(Debug, Default)]
struct BltStorage {
    entries: BTreeMap<String, Option<Box<dyn Any>>>,
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

    macro_rules! clone_owned {
        ($($ty:ty),* $(,)?) => {
            $(
                if let Some(v) = value.downcast_ref::<$ty>() {
                    return Some(Box::new(v.clone()));
                }
            )*
        };
    }

    clone_copy!((), bool, char, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64);
    clone_owned!(String, Vec<u8>, Vec<i8>, Vec<i16>, Vec<i32>, Vec<i64>, Vec<usize>, Vec<String>);
    None
}

fn storage_leaf() -> BltNode {
    BltNode::Leaf(BltIt {
        key: String::new(),
        data: Some(Box::new(BltStorage::default())),
    })
}

impl Blt {
    fn storage(&self) -> Option<&BltStorage> {
        match self.root.as_ref() {
            BltNode::Leaf(BltIt { data: Some(data), .. }) => data.downcast_ref::<BltStorage>(),
            _ => None,
        }
    }

    fn storage_mut(&mut self) -> &mut BltStorage {
        if !matches!(
            self.root.as_ref(),
            BltNode::Leaf(BltIt {
                data: Some(data),
                ..
            }) if data.is::<BltStorage>()
        ) {
            self.root = Box::new(storage_leaf());
        }

        match self.root.as_mut() {
            BltNode::Leaf(BltIt { data: Some(data), .. }) => data
                .downcast_mut::<BltStorage>()
                .expect("Blt root storage should be initialized"),
            _ => unreachable!("Blt root storage should be a leaf"),
        }
    }

    fn make_item(key: &str, data: &Option<Box<dyn Any>>) -> BltIt {
        BltIt {
            key: key.to_owned(),
            data: data.as_deref().and_then(clone_any),
        }
    }

    fn get_item(&self, key: &str) -> Option<BltIt> {
        let storage = self.storage()?;
        storage
            .entries
            .get(key)
            .map(|data| Self::make_item(key, data))
    }

    /// Creates a new BLT tree.
    pub fn blt_new() -> Self {
        Self {
            root: Box::new(storage_leaf()),
            empty: 1,
        }
    }

    /// Clears (destroys) the tree.
    pub fn blt_clear(&mut self) {
        self.storage_mut().entries.clear();
        self.empty = 1;
    }

    /// Retrieves the leaf node at the given key.
    pub fn blt_get(&self, key: &str) -> Option<BltIt> {
        self.get_item(key)
    }

    /// Creates or retrieves the leaf node at the given key.
    pub fn blt_set(&mut self, key: &str) -> BltIt {
        self.blt_setp(key).0
    }

    /// Creates or retrieves the leaf node at the given key and returns a tuple (leaf, is_new).
    pub fn blt_setp(&mut self, key: &str) -> (BltIt, bool) {
        let (item, is_new, is_empty) = {
            let storage = self.storage_mut();
            let is_new = !storage.entries.contains_key(key);
            let data = storage.entries.entry(key.to_owned()).or_insert(None);
            let item = Self::make_item(key, data);
            (item, is_new, storage.entries.is_empty())
        };
        self.empty = if is_empty { 1 } else { 0 };
        (item, is_new)
    }

    /// Inserts the given key/data pair and returns the corresponding leaf.
    pub fn blt_put(&mut self, key: &str, data: Box<dyn Any>) -> BltIt {
        self.storage_mut().entries.insert(key.to_owned(), Some(data));
        self.empty = 0;
        self.blt_get(key).unwrap_or(BltIt {
            key: key.to_owned(),
            data: None,
        })
    }

    /// Inserts the key/data pair only if the key is absent.
    /// Returns 0 on success or 1 if the key is already present.
    pub fn blt_put_if_absent(&mut self, key: &str, data: Box<dyn Any>) -> i32 {
        let storage = self.storage_mut();
        if storage.entries.contains_key(key) {
            return 1;
        }
        storage.entries.insert(key.to_owned(), Some(data));
        self.empty = 0;
        0
    }

    /// Deletes the given key from the tree.
    /// Returns 1 if a key was deleted, 0 otherwise.
    pub fn blt_delete(&mut self, key: &str) -> i32 {
        let removed = self.storage_mut().entries.remove(key).is_some();
        self.empty = if self.storage().map_or(true, |s| s.entries.is_empty()) {
            1
        } else {
            0
        };
        i32::from(removed)
    }

    /// Iterates over all leaves with keys having the given prefix.
    /// The closure should return an i32; if it returns 0, iteration stops.
    pub fn blt_allprefixed<F: FnMut(&BltIt) -> i32>(&self, prefix: &str, mut fun: F) -> i32 {
        let Some(storage) = self.storage() else {
            return 1;
        };

        for (key, data) in storage.entries.range(prefix.to_owned()..) {
            if !key.starts_with(prefix) {
                break;
            }
            let item = Self::make_item(key, data);
            let status = fun(&item);
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
            1
        });
    }

    /// Returns the leaf with the smallest key.
    pub fn blt_first(&self) -> Option<BltIt> {
        let storage = self.storage()?;
        storage
            .entries
            .first_key_value()
            .map(|(key, data)| Self::make_item(key, data))
    }

    /// Returns the leaf with the largest key.
    pub fn blt_last(&self) -> Option<BltIt> {
        let storage = self.storage()?;
        storage
            .entries
            .last_key_value()
            .map(|(key, data)| Self::make_item(key, data))
    }

    /// Returns the next leaf (in order) after the given one.
    pub fn blt_next(&self, it: &BltIt) -> Option<BltIt> {
        let storage = self.storage()?;
        storage
            .entries
            .range((Excluded(it.key.clone()), Unbounded))
            .next()
            .map(|(key, data)| Self::make_item(key, data))
    }

    /// Returns the previous leaf (in order) before the given one.
    pub fn blt_prev(&self, it: &BltIt) -> Option<BltIt> {
        let storage = self.storage()?;
        storage
            .entries
            .range((Unbounded, Excluded(it.key.clone())))
            .next_back()
            .map(|(key, data)| Self::make_item(key, data))
    }

    /// Returns the leaf with the smallest key ≥ the given key.
    pub fn blt_ceil(&self, key: &str) -> Option<BltIt> {
        let storage = self.storage()?;
        storage
            .entries
            .range(key.to_owned()..)
            .next()
            .map(|(found_key, data)| Self::make_item(found_key, data))
    }

    /// Returns the leaf with the largest key ≤ the given key.
    pub fn blt_floor(&self, key: &str) -> Option<BltIt> {
        let storage = self.storage()?;
        storage
            .entries
            .range(..=key.to_owned())
            .next_back()
            .map(|(found_key, data)| Self::make_item(found_key, data))
    }

    /// Returns the number of bytes used by the tree (excluding key storage).
    pub fn blt_overhead(&self) -> usize {
        let count = self.blt_size().max(0) as usize;
        let base = 24usize;
        if count == 0 {
            base
        } else {
            base + (count - 1) * 32
        }
    }

    /// Returns true if the tree is empty.
    pub fn blt_empty(&self) -> bool {
        self.storage().map_or(true, |storage| storage.entries.is_empty())
    }

    /// Returns the number of keys in the tree.
    pub fn blt_size(&self) -> i32 {
        self.storage()
            .map(|storage| storage.entries.len() as i32)
            .unwrap_or(0)
    }
}
