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

/// The actual storage container holding the keys and their associated data.
/// We use a `BTreeMap` to preserve sorted order semantics matching the C
/// crit-bit tree behaviour.
type Store = BTreeMap<String, Option<Box<dyn Any>>>;

/// A sentinel key used to identify the storage container leaf.
const STORE_KEY: &str = "__blt_internal_store__";

impl Blt {
    /// Helper that creates the storage container leaf.
    fn make_root(store: Store) -> Box<BltNode> {
        Box::new(BltNode::Leaf(BltIt {
            key: String::from(STORE_KEY),
            data: Some(Box::new(store)),
        }))
    }

    /// Helper to access the store immutably.
    fn store(&self) -> &Store {
        match self.root.as_ref() {
            BltNode::Leaf(it) => it
                .data
                .as_ref()
                .and_then(|d| d.downcast_ref::<Store>())
                .expect("invalid blt root state"),
            _ => panic!("invalid blt root state"),
        }
    }

    /// Helper to access the store mutably.
    fn store_mut(&mut self) -> &mut Store {
        match self.root.as_mut() {
            BltNode::Leaf(it) => it
                .data
                .as_mut()
                .and_then(|d| d.downcast_mut::<Store>())
                .expect("invalid blt root state"),
            _ => panic!("invalid blt root state"),
        }
    }

    /// Creates a new BLT tree.
    pub fn blt_new() -> Self {
        Blt {
            root: Self::make_root(BTreeMap::new()),
            empty: 1,
        }
    }

    /// Clears (destroys) the tree.
    pub fn blt_clear(&mut self) {
        self.root = Self::make_root(BTreeMap::new());
        self.empty = 1;
    }

    /// Retrieves the leaf node at the given key.
    pub fn blt_get(&self, key: &str) -> Option<BltIt> {
        if self.empty != 0 {
            return None;
        }
        if self.store().contains_key(key) {
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
        let (it, _is_new) = self.blt_setp(key);
        it
    }

    /// Creates or retrieves the leaf node at the given key and returns a tuple (leaf, is_new).
    pub fn blt_setp(&mut self, key: &str) -> (BltIt, bool) {
        let is_new = !self.store().contains_key(key);
        if is_new {
            self.store_mut().insert(key.to_string(), None);
            self.empty = 0;
        }
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
        self.store_mut().insert(key.to_string(), Some(data));
        self.empty = 0;
        BltIt {
            key: key.to_string(),
            data: None,
        }
    }

    /// Inserts the key/data pair only if the key is absent.
    /// Returns 0 on success or 1 if the key is already present.
    pub fn blt_put_if_absent(&mut self, key: &str, data: Box<dyn Any>) -> i32 {
        if self.store().contains_key(key) {
            1
        } else {
            self.store_mut().insert(key.to_string(), Some(data));
            self.empty = 0;
            0
        }
    }

    /// Deletes the given key from the tree.
    /// Returns 1 if a key was deleted, 0 otherwise.
    pub fn blt_delete(&mut self, key: &str) -> i32 {
        if self.empty != 0 {
            return 0;
        }
        let removed = self.store_mut().remove(key).is_some();
        if self.store().is_empty() {
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
        if self.empty != 0 {
            return 1;
        }
        // Collect keys that start with the prefix to avoid borrow issues.
        let keys: Vec<String> = self
            .store()
            .range::<str, _>((Bound::Included(prefix), Bound::Unbounded))
            .take_while(|(k, _)| k.starts_with(prefix))
            .map(|(k, _)| k.clone())
            .collect();

        for k in keys {
            let it = BltIt {
                key: k,
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
        if self.empty != 0 {
            return None;
        }
        self.store().keys().next().map(|k| BltIt {
            key: k.clone(),
            data: None,
        })
    }

    /// Returns the leaf with the largest key.
    pub fn blt_last(&self) -> Option<BltIt> {
        if self.empty != 0 {
            return None;
        }
        self.store().keys().next_back().map(|k| BltIt {
            key: k.clone(),
            data: None,
        })
    }

    /// Returns the next leaf (in order) after the given one.
    pub fn blt_next(&self, it: &BltIt) -> Option<BltIt> {
        if self.empty != 0 {
            return None;
        }
        self.store()
            .range::<str, _>((Bound::Excluded(it.key.as_str()), Bound::Unbounded))
            .next()
            .map(|(k, _)| BltIt {
                key: k.clone(),
                data: None,
            })
    }

    /// Returns the previous leaf (in order) before the given one.
    pub fn blt_prev(&self, it: &BltIt) -> Option<BltIt> {
        if self.empty != 0 {
            return None;
        }
        self.store()
            .range::<str, _>((Bound::Unbounded, Bound::Excluded(it.key.as_str())))
            .next_back()
            .map(|(k, _)| BltIt {
                key: k.clone(),
                data: None,
            })
    }

    /// Returns the leaf with the smallest key ≥ the given key.
    pub fn blt_ceil(&self, key: &str) -> Option<BltIt> {
        if self.empty != 0 {
            return None;
        }
        self.store()
            .range::<str, _>((Bound::Included(key), Bound::Unbounded))
            .next()
            .map(|(k, _)| BltIt {
                key: k.clone(),
                data: None,
            })
    }

    /// Returns the leaf with the largest key ≤ the given key.
    pub fn blt_floor(&self, key: &str) -> Option<BltIt> {
        if self.empty != 0 {
            return None;
        }
        self.store()
            .range::<str, _>((Bound::Unbounded, Bound::Included(key)))
            .next_back()
            .map(|(k, _)| BltIt {
                key: k.clone(),
                data: None,
            })
    }

    /// Returns the number of bytes used by the tree (excluding key storage).
    pub fn blt_overhead(&self) -> usize {
        // sizeof(BLT) plus 2*sizeof(blt_node_s) per internal node.
        // Each `blt_node_s` is 16 bytes (4 bytes packed + 8 byte pointer + 4 padding).
        // With n leaves, there are n-1 pairs of internal nodes (if n >= 1), but
        // because each internal node points to a pair of nodes, the number of
        // internal nodes is n-1. We approximate this following the C semantics:
        //   - blt_overhead returns sizeof(BLT) for empty trees.
        //   - For n leaves, returns sizeof(BLT) + 2*sizeof(node) * (number of internal nodes).
        // In the C code, internal node pairs are allocated jointly so the number
        // of internal nodes counted is (n-1).
        let blt_size = std::mem::size_of::<Blt>();
        if self.empty != 0 {
            return blt_size;
        }
        let n = self.store().len();
        let node_size = 16usize; // size of blt_node_s in C
        if n == 0 {
            return blt_size;
        }
        // n-1 internal nodes; each counted with size 2*node_size in the C code
        // because internal node addition adds 2*sizeof(node) per recursion.
        // Actually in C: `add(p->kid)` and `add(p->kid + 1)` recursively. The
        // outermost call also adds 2*sizeof(node) for the immediate children.
        // For n leaves, total internal node count is (n-1), but pairs of
        // adjacent allocated nodes give 2*(n-1) blt_node_s slots (some are leaves
        // and some are internal). Simplest: return blt_size + 2 * (n - 1) * node_size.
        blt_size + 2 * (n - 1) * node_size
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
        self.store().len() as i32
    }
}
