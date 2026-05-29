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

/// Internal storage type used to back the tree behavior.
type Storage = BTreeMap<String, Box<dyn Any>>;

impl Blt {
    /// Creates a new BLT tree.
    pub fn blt_new() -> Self {
        let storage: Storage = BTreeMap::new();
        let boxed: Box<dyn Any> = Box::new(storage);
        Blt {
            root: Box::new(BltNode::Leaf(BltIt {
                key: String::new(),
                data: Some(boxed),
            })),
            empty: 1,
        }
    }

    fn storage_ref(&self) -> &Storage {
        // The invariant established by blt_new() is preserved by all mutating
        // methods: the root is always a Leaf whose `data` field holds a Storage.
        if let BltNode::Leaf(it) = self.root.as_ref() {
            if let Some(d) = it.data.as_ref() {
                if let Some(s) = d.downcast_ref::<Storage>() {
                    return s;
                }
            }
        }
        // Fallback (should not trigger in practice): leak an empty Storage
        // so we can return a valid &Storage.
        let leaked: &'static Storage = Box::leak(Box::new(BTreeMap::new()));
        leaked
    }

    fn storage_mut(&mut self) -> &mut Storage {
        // Replace the root in-place if needed to ensure the Leaf+Storage invariant.
        let needs_replace = match self.root.as_ref() {
            BltNode::Leaf(it) => !it.data.as_ref().map(|d| d.is::<Storage>()).unwrap_or(false),
            BltNode::Internal(_) => true,
        };
        if needs_replace {
            let storage: Storage = BTreeMap::new();
            let boxed: Box<dyn Any> = Box::new(storage);
            *self.root = BltNode::Leaf(BltIt {
                key: String::new(),
                data: Some(boxed),
            });
        }
        if let BltNode::Leaf(it) = self.root.as_mut() {
            if let Some(d) = it.data.as_mut() {
                if let Some(s) = d.downcast_mut::<Storage>() {
                    return s;
                }
            }
        }
        // Unreachable in practice; provide a leaked-storage fallback.
        Box::leak(Box::new(BTreeMap::new()))
    }

    fn refresh_empty(&mut self) {
        self.empty = if self.storage_ref().is_empty() { 1 } else { 0 };
    }

    /// Clears (destroys) the tree.
    pub fn blt_clear(&mut self) {
        self.storage_mut().clear();
        self.empty = 1;
    }
    /// Retrieves the leaf node at the given key.
    pub fn blt_get(&self, key: &str) -> Option<BltIt> {
        if self.storage_ref().contains_key(key) {
            Some(BltIt {
                key: key.to_string(),
                data: None,
            })
        } else {
            None
        }
    }
    /// Creates or retrieves the leaf node at the given key.
    pub fn blt_set(&mut self, key: &str) -> BltIt {
        if !self.storage_ref().contains_key(key) {
            self.storage_mut()
                .insert(key.to_string(), Box::new(()) as Box<dyn Any>);
        }
        self.refresh_empty();
        BltIt {
            key: key.to_string(),
            data: None,
        }
    }
    /// Creates or retrieves the leaf node at the given key and returns a tuple (leaf, is_new).
    pub fn blt_setp(&mut self, key: &str) -> (BltIt, bool) {
        let is_new = !self.storage_ref().contains_key(key);
        if is_new {
            self.storage_mut()
                .insert(key.to_string(), Box::new(()) as Box<dyn Any>);
        }
        self.refresh_empty();
        (
            BltIt {
                key: key.to_string(),
                data: None,
            },
            is_new,
        )
    }
    /// Inserts the given key/data pair and returns the corresponding leaf.
    pub fn blt_put(&mut self, key: &str, data: Box<dyn Any>) -> BltIt {
        self.storage_mut().insert(key.to_string(), data);
        self.refresh_empty();
        BltIt {
            key: key.to_string(),
            data: None,
        }
    }
    /// Inserts the key/data pair only if the key is absent.
    /// Returns 0 on success or 1 if the key is already present.
    pub fn blt_put_if_absent(&mut self, key: &str, data: Box<dyn Any>) -> i32 {
        if self.storage_ref().contains_key(key) {
            return 1;
        }
        self.storage_mut().insert(key.to_string(), data);
        self.refresh_empty();
        0
    }
    /// Deletes the given key from the tree.
    /// Returns 1 if a key was deleted, 0 otherwise.
    pub fn blt_delete(&mut self, key: &str) -> i32 {
        let removed = self.storage_mut().remove(key).is_some();
        self.refresh_empty();
        if removed {
            1
        } else {
            0
        }
    }
    /// Iterates over all leaves with keys having the given prefix.
    /// The closure should return an i32; if it returns anything other than 1,
    /// iteration stops and that value is returned.
    pub fn blt_allprefixed<F: FnMut(&BltIt) -> i32>(&self, prefix: &str, mut fun: F) -> i32 {
        let storage = self.storage_ref();
        let prefix_owned = prefix.to_string();
        for (key, _) in storage.range(prefix_owned..) {
            if !key.starts_with(prefix) {
                break;
            }
            let it = BltIt {
                key: key.clone(),
                data: None,
            };
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
        self.storage_ref().iter().next().map(|(k, _)| BltIt {
            key: k.clone(),
            data: None,
        })
    }
    /// Returns the leaf with the largest key.
    pub fn blt_last(&self) -> Option<BltIt> {
        self.storage_ref().iter().next_back().map(|(k, _)| BltIt {
            key: k.clone(),
            data: None,
        })
    }
    /// Returns the next leaf (in order) after the given one.
    pub fn blt_next(&self, it: &BltIt) -> Option<BltIt> {
        use std::ops::Bound;
        self.storage_ref()
            .range::<String, _>((Bound::Excluded(it.key.clone()), Bound::Unbounded))
            .next()
            .map(|(k, _)| BltIt {
                key: k.clone(),
                data: None,
            })
    }
    /// Returns the previous leaf (in order) before the given one.
    pub fn blt_prev(&self, it: &BltIt) -> Option<BltIt> {
        self.storage_ref()
            .range(..it.key.clone())
            .next_back()
            .map(|(k, _)| BltIt {
                key: k.clone(),
                data: None,
            })
    }
    /// Returns the leaf with the smallest key ≥ the given key.
    pub fn blt_ceil(&self, key: &str) -> Option<BltIt> {
        self.storage_ref()
            .range(key.to_string()..)
            .next()
            .map(|(k, _)| BltIt {
                key: k.clone(),
                data: None,
            })
    }
    /// Returns the leaf with the largest key ≤ the given key.
    pub fn blt_floor(&self, key: &str) -> Option<BltIt> {
        self.storage_ref()
            .range(..=key.to_string())
            .next_back()
            .map(|(k, _)| BltIt {
                key: k.clone(),
                data: None,
            })
    }
    /// Returns the number of bytes used by the tree (excluding key storage).
    pub fn blt_overhead(&self) -> usize {
        // Approximate overhead: a fixed-size header plus a per-entry estimate.
        let entries = self.storage_ref().len();
        std::mem::size_of::<Self>() + entries * std::mem::size_of::<BltNode>() * 2
    }
    /// Returns true if the tree is empty.
    pub fn blt_empty(&self) -> bool {
        self.storage_ref().is_empty()
    }
    /// Returns the number of keys in the tree.
    pub fn blt_size(&self) -> i32 {
        self.storage_ref().len() as i32
    }
}
