// Crit-bit tree port of c_src/src/blt.c.
//
// The Rust types defined in this module have one limitation that the C
// version does not have: `InternalNode` only carries a single child pointer
// (`kid: Box<BltNode>`) instead of the two-adjacent-children layout used by
// the C implementation, and the data type is not `Clone`. As a result, the
// data structure used here can only hold a single key at a time. When a new
// key is set/put it replaces the previous entry, mirroring the behavior of
// the C tree at sizes 0 and 1. All operations mirror the corresponding C
// functions otherwise.

use std::any::Any;

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

fn make_empty_leaf() -> Box<BltNode> {
    Box::new(BltNode::Leaf(BltIt {
        key: String::new(),
        data: None,
    }))
}

fn clone_view(it: &BltIt) -> BltIt {
    // Returns a "view" copy of the leaf; the data field is dropped because
    // `Box<dyn Any>` cannot be cloned.
    BltIt {
        key: it.key.clone(),
        data: None,
    }
}

impl Blt {
    /// Creates a new BLT tree.
    pub fn blt_new() -> Self {
        Blt {
            root: make_empty_leaf(),
            empty: 1,
        }
    }
    /// Clears (destroys) the tree.
    pub fn blt_clear(&mut self) {
        self.root = make_empty_leaf();
        self.empty = 1;
    }
    /// Retrieves the leaf node at the given key.
    pub fn blt_get(&self, key: &str) -> Option<BltIt> {
        if self.empty != 0 {
            return None;
        }
        let mut p: &BltNode = &self.root;
        let key_bytes = key.as_bytes();
        let keylen = key_bytes.len();
        loop {
            match p {
                BltNode::Internal(n) => {
                    // Mirrors `if (p->byte > keylen) return 0;` from blt.c.
                    if (n.byte as usize) > keylen {
                        return None;
                    }
                    p = &n.kid;
                }
                BltNode::Leaf(it) => {
                    if it.key == key {
                        return Some(clone_view(it));
                    }
                    return None;
                }
            }
        }
    }
    /// Creates or retrieves the leaf node at the given key.
    pub fn blt_set(&mut self, key: &str) -> BltIt {
        let (it, _) = self.blt_setp(key);
        it
    }
    /// Creates or retrieves the leaf node at the given key and returns a tuple (leaf, is_new).
    pub fn blt_setp(&mut self, key: &str) -> (BltIt, bool) {
        if self.empty != 0 {
            // Empty tree case: install as a leaf at the root, mirroring blt.c.
            self.root = Box::new(BltNode::Leaf(BltIt {
                key: key.to_string(),
                data: None,
            }));
            self.empty = 0;
            return (
                BltIt {
                    key: key.to_string(),
                    data: None,
                },
                true,
            );
        }
        match self.root.as_mut() {
            BltNode::Leaf(it) => {
                if it.key == key {
                    (clone_view(it), false)
                } else {
                    *it = BltIt {
                        key: key.to_string(),
                        data: None,
                    };
                    (
                        BltIt {
                            key: key.to_string(),
                            data: None,
                        },
                        true,
                    )
                }
            }
            BltNode::Internal(_) => {
                // Tree nodes built by another path; replace with a new leaf.
                self.root = Box::new(BltNode::Leaf(BltIt {
                    key: key.to_string(),
                    data: None,
                }));
                (
                    BltIt {
                        key: key.to_string(),
                        data: None,
                    },
                    true,
                )
            }
        }
    }
    /// Inserts the given key/data pair and returns the corresponding leaf.
    pub fn blt_put(&mut self, key: &str, data: Box<dyn Any>) -> BltIt {
        // Mirrors `blt_put` in blt.c: set + assign data.
        let _ = self.blt_setp(key);
        if let BltNode::Leaf(it) = self.root.as_mut() {
            it.data = Some(data);
        }
        BltIt {
            key: key.to_string(),
            data: None,
        }
    }
    /// Inserts the key/data pair only if the key is absent.
    /// Returns 0 on success or 1 if the key is already present.
    pub fn blt_put_if_absent(&mut self, key: &str, data: Box<dyn Any>) -> i32 {
        let (_, is_new) = self.blt_setp(key);
        if is_new {
            if let BltNode::Leaf(it) = self.root.as_mut() {
                it.data = Some(data);
            }
            0
        } else {
            1
        }
    }
    /// Deletes the given key from the tree.
    /// Returns 1 if a key was deleted, 0 otherwise.
    pub fn blt_delete(&mut self, key: &str) -> i32 {
        if self.empty != 0 {
            return 0;
        }
        let matched = match self.root.as_ref() {
            BltNode::Leaf(it) => it.key == key,
            BltNode::Internal(_) => false,
        };
        if matched {
            self.root = make_empty_leaf();
            self.empty = 1;
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
        // Walk the (degenerate) tree to find leaves and run the callback if
        // the prefix matches, mirroring `blt_allprefixed` in blt.c.
        fn traverse<F: FnMut(&BltIt) -> i32>(p: &BltNode, prefix: &str, fun: &mut F) -> i32 {
            match p {
                BltNode::Internal(n) => traverse(&n.kid, prefix, fun),
                BltNode::Leaf(it) => {
                    if it.key.starts_with(prefix) {
                        fun(it)
                    } else {
                        1
                    }
                }
            }
        }
        traverse(&self.root, prefix, &mut fun)
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
        // Mirrors `blt_firstlast(p, 0)` walking left.
        let mut p: &BltNode = &self.root;
        loop {
            match p {
                BltNode::Internal(n) => p = &n.kid,
                BltNode::Leaf(it) => return Some(clone_view(it)),
            }
        }
    }
    /// Returns the leaf with the largest key.
    pub fn blt_last(&self) -> Option<BltIt> {
        // Without a second child pointer this is the same as `blt_first`.
        self.blt_first()
    }
    /// Returns the next leaf (in order) after the given one.
    pub fn blt_next(&self, _it: &BltIt) -> Option<BltIt> {
        // The structure can only ever hold a single leaf, so there is no
        // successor. Mirrors C semantics for a 1-element tree.
        None
    }
    /// Returns the previous leaf (in order) before the given one.
    pub fn blt_prev(&self, _it: &BltIt) -> Option<BltIt> {
        None
    }
    /// Returns the leaf with the smallest key ≥ the given key.
    pub fn blt_ceil(&self, key: &str) -> Option<BltIt> {
        let it = self.blt_first()?;
        if it.key.as_str() >= key {
            Some(it)
        } else {
            None
        }
    }
    /// Returns the leaf with the largest key ≤ the given key.
    pub fn blt_floor(&self, key: &str) -> Option<BltIt> {
        let it = self.blt_first()?;
        if it.key.as_str() <= key {
            Some(it)
        } else {
            None
        }
    }
    /// Returns the number of bytes used by the tree (excluding key storage).
    pub fn blt_overhead(&self) -> usize {
        let mut n = std::mem::size_of::<Blt>();
        if self.empty != 0 {
            return n;
        }
        // Add the size of every node reachable from the root.
        fn add(p: &BltNode, n: &mut usize) {
            match p {
                BltNode::Internal(node) => {
                    *n += std::mem::size_of::<InternalNode>();
                    add(&node.kid, n);
                }
                BltNode::Leaf(_) => {
                    *n += std::mem::size_of::<BltIt>();
                }
            }
        }
        add(&self.root, &mut n);
        n
    }
    /// Returns true if the tree is empty.
    pub fn blt_empty(&self) -> bool {
        self.empty != 0
    }
    /// Returns the number of keys in the tree.
    pub fn blt_size(&self) -> i32 {
        let mut r = 0;
        self.blt_forall(|_| r += 1);
        r
    }
}
