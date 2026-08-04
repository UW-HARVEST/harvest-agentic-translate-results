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
    /// The two children. In the C version this corresponds to two adjacent
    /// memory blocks pointed to by `kid`; element 0 is the left child and
    /// element 1 is the right child.
    pub kid: Box<[BltNode; 2]>,
}
/// Represents a leaf node in the BLT tree.
#[derive(Debug)]
pub struct BltIt {
    /// The key associated with the leaf.
    pub key: String,
    /// Associated data.
    pub data: Option<Box<dyn Any>>,
}

// Returns the byte where each bit is 1 except for the bit corresponding to
// the leading bit of x.
fn to_mask(mut x: u8) -> u8 {
    x |= x >> 1;
    x |= x >> 2;
    x |= x >> 4;
    x & !(x >> 1)
}

// Returns the byte at index `idx` in `key`, or 0 if out of bounds (mirrors
// the implicit NUL terminator of C strings).
fn key_byte(key: &[u8], idx: usize) -> u8 {
    if idx < key.len() {
        key[idx]
    } else {
        0
    }
}

// Returns true iff crit position (byte_a, mask_a) is strictly earlier than
// (byte_b, mask_b). A position is earlier when its byte index is smaller, or
// when the byte indices are equal and the mask has a higher numeric value
// (single-bit masks: bigger value = more significant bit = earlier).
fn is_earlier(byte_a: u32, mask_a: u8, byte_b: u32, mask_b: u8) -> bool {
    if byte_a != byte_b {
        byte_a < byte_b
    } else {
        mask_a > mask_b
    }
}

// Walks down `node` always taking the child indicated by `dir` (0 = left,
// 1 = right) until a leaf is reached.
fn firstlast(node: &BltNode, dir: usize) -> &BltIt {
    let mut p = node;
    loop {
        match p {
            BltNode::Internal(n) => p = &n.kid[dir],
            BltNode::Leaf(it) => return it,
        }
    }
}

fn empty_leaf() -> BltNode {
    BltNode::Leaf(BltIt {
        key: String::new(),
        data: None,
    })
}

fn it_view(it: &BltIt) -> BltIt {
    BltIt {
        key: it.key.clone(),
        data: None,
    }
}

// Recursive helper used by blt_setp to find the insertion point and replace
// it with a new internal node containing the previous contents and the new
// leaf.
fn insert_helper(
    node: &mut BltNode,
    byte: u32,
    mask: u8,
    key: &[u8],
    new_key: String,
    goes_right: bool,
) {
    let descend_dir: Option<usize> = match node {
        BltNode::Internal(n) if !is_earlier(byte, mask, n.byte, n.mask) => {
            let b = n.byte as usize;
            let dir = if b < key.len() && (key[b] & n.mask) != 0 {
                1
            } else {
                0
            };
            Some(dir)
        }
        _ => None,
    };

    match descend_dir {
        Some(dir) => {
            if let BltNode::Internal(n) = node {
                insert_helper(&mut n.kid[dir], byte, mask, key, new_key, goes_right);
            } else {
                unreachable!();
            }
        }
        None => {
            let old = std::mem::replace(node, empty_leaf());
            let new_leaf = BltNode::Leaf(BltIt {
                key: new_key,
                data: None,
            });
            let kids = if goes_right {
                Box::new([old, new_leaf])
            } else {
                Box::new([new_leaf, old])
            };
            *node = BltNode::Internal(InternalNode {
                byte,
                mask,
                padding: 0,
                kid: kids,
            });
        }
    }
}

// Recursive helper used by blt_delete. Returns 1 if a key was removed, 0
// otherwise. When the matching leaf is the direct child of `node`, this
// function replaces `node` itself with its sibling.
fn delete_helper(node: &mut BltNode, key: &[u8], keylen: usize) -> i32 {
    match node {
        BltNode::Internal(n) => {
            if (n.byte as usize) > keylen {
                return 0;
            }
            let b = n.byte as usize;
            let dir = if b < keylen && (key[b] & n.mask) != 0 {
                1
            } else {
                0
            };

            let child_is_leaf = matches!(n.kid[dir], BltNode::Leaf(_));
            if child_is_leaf {
                let matches_key = match &n.kid[dir] {
                    BltNode::Leaf(leaf) => leaf.key.as_bytes() == key,
                    _ => false,
                };
                if !matches_key {
                    return 0;
                }
                let other = 1 - dir;
                let kids_box = std::mem::replace(
                    &mut n.kid,
                    Box::new([empty_leaf(), empty_leaf()]),
                );
                let [a, b] = *kids_box;
                let sibling = if other == 0 { a } else { b };
                *node = sibling;
                return 1;
            }

            delete_helper(&mut n.kid[dir], key, keylen)
        }
        BltNode::Leaf(_) => 0,
    }
}

impl Blt {
    /// Creates a new BLT tree.
    pub fn blt_new() -> Self {
        Blt {
            root: Box::new(empty_leaf()),
            empty: 1,
        }
    }
    /// Clears (destroys) the tree.
    pub fn blt_clear(&mut self) {
        self.root = Box::new(empty_leaf());
        self.empty = 1;
    }

    // Walks the tree as if `key` were present and returns the leaf reached.
    fn confident_walk<'a>(&'a self, key: &[u8]) -> Option<&'a BltIt> {
        if self.empty != 0 {
            return None;
        }
        let mut p: &BltNode = &self.root;
        loop {
            match p {
                BltNode::Internal(n) => {
                    let b = n.byte as usize;
                    let dir = if b < key.len() && (key[b] & n.mask) != 0 {
                        1
                    } else {
                        0
                    };
                    p = &n.kid[dir];
                }
                BltNode::Leaf(it) => return Some(it),
            }
        }
    }

    // Locates the leaf with the given key (mutable) for setting data.
    fn find_leaf_mut(&mut self, key: &str) -> Option<&mut BltIt> {
        if self.empty != 0 {
            return None;
        }
        let key_bytes = key.as_bytes();
        let keylen = key_bytes.len();
        let mut p: &mut BltNode = &mut self.root;
        loop {
            match p {
                BltNode::Internal(n) => {
                    if (n.byte as usize) > keylen {
                        return None;
                    }
                    let b = n.byte as usize;
                    let dir = if b < keylen && (key_bytes[b] & n.mask) != 0 {
                        1
                    } else {
                        0
                    };
                    p = &mut n.kid[dir];
                }
                BltNode::Leaf(it) => {
                    if it.key.as_bytes() == key_bytes {
                        return Some(it);
                    }
                    return None;
                }
            }
        }
    }

    /// Retrieves the leaf node at the given key.
    pub fn blt_get(&self, key: &str) -> Option<BltIt> {
        if self.empty != 0 {
            return None;
        }
        let key_bytes = key.as_bytes();
        let keylen = key_bytes.len();
        let mut p: &BltNode = &self.root;
        loop {
            match p {
                BltNode::Internal(n) => {
                    if (n.byte as usize) > keylen {
                        return None;
                    }
                    let b = n.byte as usize;
                    let dir = if b < keylen && (key_bytes[b] & n.mask) != 0 {
                        1
                    } else {
                        0
                    };
                    p = &n.kid[dir];
                }
                BltNode::Leaf(it) => {
                    if it.key.as_bytes() == key_bytes {
                        return Some(it_view(it));
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
        let key_bytes = key.as_bytes();

        // Empty tree case: place the leaf directly in the root.
        if self.empty != 0 {
            self.empty = 0;
            *self.root = BltNode::Leaf(BltIt {
                key: key.to_string(),
                data: None,
            });
            return (
                BltIt {
                    key: key.to_string(),
                    data: None,
                },
                true,
            );
        }

        // Find any leaf to compare against.
        let leaf_key = self.confident_walk(key_bytes).unwrap().key.clone();
        let pk = leaf_key.as_bytes();

        // Find first differing byte and compute the crit mask.
        let mut byte_idx: usize = 0;
        let mut x_mask: u8 = 0;
        let mut equal = false;
        let max_idx = std::cmp::max(key_bytes.len(), pk.len()) + 1;
        for i in 0..max_idx {
            let a = key_byte(key_bytes, i);
            let b = key_byte(pk, i);
            let xor = a ^ b;
            if xor != 0 {
                byte_idx = i;
                x_mask = to_mask(xor);
                break;
            }
            if a == 0 {
                equal = true;
                break;
            }
        }

        if equal {
            // Key already present.
            return (
                BltIt {
                    key: key.to_string(),
                    data: None,
                },
                false,
            );
        }

        let goes_right = (key_byte(key_bytes, byte_idx) & x_mask) != 0;
        let new_byte = byte_idx as u32;
        let new_mask = x_mask;

        insert_helper(
            &mut self.root,
            new_byte,
            new_mask,
            key_bytes,
            key.to_string(),
            goes_right,
        );

        (
            BltIt {
                key: key.to_string(),
                data: None,
            },
            true,
        )
    }
    /// Inserts the given key/data pair and returns the corresponding leaf.
    pub fn blt_put(&mut self, key: &str, data: Box<dyn Any>) -> BltIt {
        let _ = self.blt_setp(key);
        if let Some(leaf) = self.find_leaf_mut(key) {
            leaf.data = Some(data);
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
            if let Some(leaf) = self.find_leaf_mut(key) {
                leaf.data = Some(data);
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
        let key_bytes = key.as_bytes();
        let keylen = key_bytes.len();

        // Special case: root is itself a leaf.
        let root_leaf_match = match &*self.root {
            BltNode::Leaf(leaf) => leaf.key.as_bytes() == key_bytes,
            _ => false,
        };
        if matches!(*self.root, BltNode::Leaf(_)) {
            if root_leaf_match {
                self.empty = 1;
                *self.root = empty_leaf();
                return 1;
            }
            return 0;
        }

        delete_helper(&mut self.root, key_bytes, keylen)
    }
    /// Iterates over all leaves with keys having the given prefix.
    /// The closure should return an i32; if it returns 0, iteration stops.
    pub fn blt_allprefixed<F: FnMut(&BltIt) -> i32>(&self, prefix: &str, mut fun: F) -> i32 {
        if self.empty != 0 {
            return 1;
        }
        let key = prefix.as_bytes();
        let keylen = key.len();

        let mut p: &BltNode = &self.root;
        let mut top: &BltNode = p;

        loop {
            match p {
                BltNode::Internal(n) => {
                    if (n.byte as usize) >= keylen {
                        p = &n.kid[0];
                    } else {
                        let dir = if (key[n.byte as usize] & n.mask) != 0 {
                            1
                        } else {
                            0
                        };
                        p = &n.kid[dir];
                        top = p;
                    }
                }
                BltNode::Leaf(_) => break,
            }
        }

        if let BltNode::Leaf(leaf) = p {
            let lk = leaf.key.as_bytes();
            if lk.len() < keylen || &lk[..keylen] != key {
                return 1;
            }
        }

        fn traverse<F: FnMut(&BltIt) -> i32>(node: &BltNode, fun: &mut F) -> i32 {
            match node {
                BltNode::Internal(n) => {
                    let s = traverse(&n.kid[0], fun);
                    if s != 1 {
                        return s;
                    }
                    let s = traverse(&n.kid[1], fun);
                    if s != 1 {
                        return s;
                    }
                    1
                }
                BltNode::Leaf(it) => fun(it),
            }
        }

        traverse(top, &mut fun)
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
        Some(it_view(firstlast(&self.root, 0)))
    }
    /// Returns the leaf with the largest key.
    pub fn blt_last(&self) -> Option<BltIt> {
        if self.empty != 0 {
            return None;
        }
        Some(it_view(firstlast(&self.root, 1)))
    }
    /// Returns the next leaf (in order) after the given one.
    pub fn blt_next(&self, it: &BltIt) -> Option<BltIt> {
        if self.empty != 0 {
            return None;
        }
        let key = it.key.as_bytes();
        let mut p: &BltNode = &self.root;
        let mut other: Option<&BltNode> = None;
        loop {
            match p {
                BltNode::Internal(n) => {
                    let b = n.byte as usize;
                    let bit_set = b < key.len() && (key[b] & n.mask) != 0;
                    if !bit_set {
                        other = Some(&n.kid[1]);
                        p = &n.kid[0];
                    } else {
                        p = &n.kid[1];
                    }
                }
                BltNode::Leaf(_) => break,
            }
        }
        other.map(|n| it_view(firstlast(n, 0)))
    }
    /// Returns the previous leaf (in order) before the given one.
    pub fn blt_prev(&self, it: &BltIt) -> Option<BltIt> {
        if self.empty != 0 {
            return None;
        }
        let key = it.key.as_bytes();
        let mut p: &BltNode = &self.root;
        let mut other: Option<&BltNode> = None;
        loop {
            match p {
                BltNode::Internal(n) => {
                    let b = n.byte as usize;
                    let bit_set = b < key.len() && (key[b] & n.mask) != 0;
                    if bit_set {
                        other = Some(&n.kid[0]);
                        p = &n.kid[1];
                    } else {
                        p = &n.kid[0];
                    }
                }
                BltNode::Leaf(_) => break,
            }
        }
        other.map(|n| it_view(firstlast(n, 1)))
    }
    /// Returns the leaf with the smallest key ≥ the given key.
    pub fn blt_ceil(&self, key: &str) -> Option<BltIt> {
        self.ceilfloor(key, 0)
    }
    /// Returns the leaf with the largest key ≤ the given key.
    pub fn blt_floor(&self, key: &str) -> Option<BltIt> {
        self.ceilfloor(key, 1)
    }

    fn ceilfloor(&self, key: &str, way: usize) -> Option<BltIt> {
        if self.empty != 0 {
            return None;
        }
        let key_bytes = key.as_bytes();

        let leaf_key = self.confident_walk(key_bytes)?.key.clone();
        let pk = leaf_key.as_bytes();

        let mut byte_idx: usize = 0;
        let mut x_mask: u8 = 0;
        let mut equal = false;
        let max_idx = std::cmp::max(key_bytes.len(), pk.len()) + 1;
        for i in 0..max_idx {
            let a = key_byte(key_bytes, i);
            let b = key_byte(pk, i);
            let xor = a ^ b;
            if xor != 0 {
                byte_idx = i;
                x_mask = to_mask(xor);
                break;
            }
            if a == 0 {
                equal = true;
                break;
            }
        }

        if equal {
            return Some(BltIt {
                key: leaf_key,
                data: None,
            });
        }

        let new_byte = byte_idx as u32;
        let new_mask = x_mask;

        let mut p: &BltNode = &self.root;
        let mut other: Option<&BltNode> = None;

        loop {
            match p {
                BltNode::Internal(n) => {
                    if is_earlier(new_byte, new_mask, n.byte, n.mask) {
                        break;
                    }
                    let b = n.byte as usize;
                    let dir = if b < key_bytes.len() && (n.mask & key_bytes[b]) != 0 {
                        1
                    } else {
                        0
                    };
                    if dir == way {
                        other = Some(&n.kid[1 - way]);
                    }
                    p = &n.kid[dir];
                }
                BltNode::Leaf(_) => break,
            }
        }

        let ndir = if (x_mask & key_byte(key_bytes, byte_idx)) != 0 {
            1
        } else {
            0
        };
        if ndir == way {
            other = Some(p);
        }

        other.map(|n| it_view(firstlast(n, way)))
    }

    /// Returns the number of bytes used by the tree (excluding key storage).
    pub fn blt_overhead(&self) -> usize {
        let mut n = std::mem::size_of::<Blt>();
        if self.empty != 0 {
            return n;
        }
        fn add(node: &BltNode, n: &mut usize) {
            if let BltNode::Internal(int) = node {
                *n += 2 * std::mem::size_of::<InternalNode>();
                add(&int.kid[0], n);
                add(&int.kid[1], n);
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
        let mut count = 0;
        self.blt_forall(|_| count += 1);
        count
    }
}
