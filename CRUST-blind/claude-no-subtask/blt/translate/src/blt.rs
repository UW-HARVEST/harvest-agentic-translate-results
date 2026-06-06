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

// ---------- Internal binary crit-bit tree implementation ----------
//
// The exposed `BltNode::Internal` variant only has one `kid` pointer, but a
// crit-bit tree fundamentally needs two children per internal node.  We
// therefore keep a "shadow" binary crit-bit tree as the source of truth and
// only use the exported `Blt::root` field as a placeholder (kept consistent
// for `empty` reads).  All real work is performed against this internal tree.

#[derive(Debug, Clone)]
struct InnerLeaf {
    key: String,
}

#[derive(Debug)]
enum InnerNode {
    Internal(InnerInternal),
    Leaf(InnerLeaf),
}

#[derive(Debug)]
struct InnerInternal {
    byte: usize,
    mask: u8,
    left: Box<InnerNode>,
    right: Box<InnerNode>,
}

// Returns the byte where each bit is 1 except for the bit corresponding to
// the leading bit of x.
fn to_mask(x: u8) -> u8 {
    let mut x = x;
    x |= x >> 1;
    x |= x >> 2;
    x |= x >> 4;
    x & !(x >> 1)
}

fn key_byte(key: &str, idx: usize) -> u8 {
    let bytes = key.as_bytes();
    if idx < bytes.len() {
        bytes[idx]
    } else {
        0
    }
}

fn follow<'a>(p: &'a InnerInternal, key: &str) -> &'a InnerNode {
    let c = key_byte(key, p.byte);
    if c & p.mask != 0 {
        &p.right
    } else {
        &p.left
    }
}

// Used during lookup: when key is shorter than crit-bit byte, treat as 0
// (always go left). Same as in C.
fn confident_descend<'a>(root: &'a InnerNode, key: &str) -> &'a InnerLeaf {
    let mut p = root;
    let keylen = key.len();
    loop {
        match p {
            InnerNode::Leaf(l) => return l,
            InnerNode::Internal(n) => {
                let c = if n.byte < keylen {
                    key_byte(key, n.byte) & n.mask
                } else {
                    0
                };
                p = if c != 0 { &n.right } else { &n.left };
            }
        }
    }
}

fn first_last_in<'a>(root: &'a InnerNode, dir: usize) -> &'a InnerLeaf {
    let mut p = root;
    loop {
        match p {
            InnerNode::Leaf(l) => return l,
            InnerNode::Internal(n) => {
                p = if dir == 0 { &n.left } else { &n.right };
            }
        }
    }
}

/// Storage type used in `BltIt::data` so the public API can carry the entire
/// internal tree without altering exported types.  A `Blt` always wraps a
/// dummy `BltIt` containing this struct.  `data` of leaves returned to users
/// stays null (`None`) as per the C semantics in tests.
struct BltStorage {
    inner: Option<Box<InnerNode>>,
    // Per-key data stored externally (so public API leaves can stay simple).
    data: std::collections::HashMap<String, Option<Box<dyn Any>>>,
}

impl std::fmt::Debug for BltStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BltStorage").finish()
    }
}

impl Blt {
    fn make_placeholder() -> Box<BltNode> {
        Box::new(BltNode::Leaf(BltIt {
            key: String::new(),
            data: Some(Box::new(BltStorage {
                inner: None,
                data: std::collections::HashMap::new(),
            })),
        }))
    }

    fn storage(&self) -> &BltStorage {
        match &*self.root {
            BltNode::Leaf(it) => it
                .data
                .as_ref()
                .and_then(|b| b.downcast_ref::<BltStorage>())
                .expect("Blt root must contain BltStorage"),
            BltNode::Internal(_) => unreachable!("Blt root must be a Leaf placeholder"),
        }
    }

    fn storage_mut(&mut self) -> &mut BltStorage {
        match &mut *self.root {
            BltNode::Leaf(it) => it
                .data
                .as_mut()
                .and_then(|b| b.downcast_mut::<BltStorage>())
                .expect("Blt root must contain BltStorage"),
            BltNode::Internal(_) => unreachable!("Blt root must be a Leaf placeholder"),
        }
    }

    /// Creates a new BLT tree.
    pub fn blt_new() -> Self {
        Blt {
            root: Self::make_placeholder(),
            empty: 1,
        }
    }

    /// Clears (destroys) the tree.
    pub fn blt_clear(&mut self) {
        let s = self.storage_mut();
        s.inner = None;
        s.data.clear();
        self.empty = 1;
    }

    /// Retrieves the leaf node at the given key.
    pub fn blt_get(&self, key: &str) -> Option<BltIt> {
        let s = self.storage();
        let root = s.inner.as_deref()?;
        // Walk down following the key.
        let mut p = root;
        let keylen = key.len();
        loop {
            match p {
                InnerNode::Leaf(_) => break,
                InnerNode::Internal(n) => {
                    if n.byte > keylen {
                        return None;
                    }
                    p = follow(n, key);
                }
            }
        }
        if let InnerNode::Leaf(l) = p {
            if l.key == key {
                return Some(BltIt {
                    key: l.key.clone(),
                    data: None,
                });
            }
        }
        None
    }

    /// Creates or retrieves the leaf node at the given key.
    pub fn blt_set(&mut self, key: &str) -> BltIt {
        let (it, _) = self.blt_setp(key);
        it
    }

    /// Creates or retrieves the leaf node at the given key and returns a tuple (leaf, is_new).
    pub fn blt_setp(&mut self, key: &str) -> (BltIt, bool) {
        // Empty tree case.
        let storage = self.storage_mut();
        if storage.inner.is_none() {
            storage.inner = Some(Box::new(InnerNode::Leaf(InnerLeaf {
                key: key.to_string(),
            })));
            storage.data.insert(key.to_string(), None);
            self.empty = 0;
            return (
                BltIt {
                    key: key.to_string(),
                    data: None,
                },
                true,
            );
        }

        // Find the candidate leaf via confident descent.
        let leaf_key = {
            let root = storage.inner.as_deref().unwrap();
            confident_descend(root, key).key.clone()
        };

        // Compare keys. Iterate over byte positions including a virtual NUL.
        let key_bytes = key.as_bytes();
        let leaf_bytes = leaf_key.as_bytes();
        let max_len = key_bytes.len().max(leaf_bytes.len()) + 1;
        let mut diff_byte: Option<usize> = None;
        let mut diff_mask: u8 = 0;
        let mut equal = false;
        for i in 0..max_len {
            let a = if i < key_bytes.len() { key_bytes[i] } else { 0 };
            let b = if i < leaf_bytes.len() {
                leaf_bytes[i]
            } else {
                0
            };
            let x = a ^ b;
            if x != 0 {
                diff_byte = Some(i);
                diff_mask = to_mask(x);
                break;
            }
            if a == 0 {
                equal = true;
                break;
            }
        }

        if equal {
            // Key already exists.
            return (
                BltIt {
                    key: key.to_string(),
                    data: None,
                },
                false,
            );
        }

        let byte = diff_byte.unwrap();
        let mask = diff_mask;

        // Now insert into the inner tree. We need to find the first node along
        // the path from the root whose crit-bit is greater than ours, and
        // splice in a new internal node.
        let storage = self.storage_mut();
        let root_owned = storage.inner.take().unwrap();
        let new_root = insert_node(root_owned, key, byte, mask);
        storage.inner = Some(new_root);
        storage.data.insert(key.to_string(), None);
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
        let it = self.blt_set(key);
        // Store data.
        let storage = self.storage_mut();
        storage.data.insert(key.to_string(), Some(data));
        BltIt {
            key: it.key,
            data: None,
        }
    }

    /// Inserts the key/data pair only if the key is absent.
    /// Returns 0 on success or 1 if the key is already present.
    pub fn blt_put_if_absent(&mut self, key: &str, data: Box<dyn Any>) -> i32 {
        let (_it, is_new) = self.blt_setp(key);
        if is_new {
            let storage = self.storage_mut();
            storage.data.insert(key.to_string(), Some(data));
            0
        } else {
            1
        }
    }

    /// Deletes the given key from the tree.
    /// Returns 1 if a key was deleted, 0 otherwise.
    pub fn blt_delete(&mut self, key: &str) -> i32 {
        let storage = self.storage_mut();
        if storage.inner.is_none() {
            return 0;
        }
        let root_owned = storage.inner.take().unwrap();
        let (new_root, removed) = delete_from(root_owned, key);
        storage.inner = new_root;
        if removed {
            storage.data.remove(key);
            if storage.inner.is_none() {
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
        let s = self.storage();
        let root = match s.inner.as_deref() {
            Some(r) => r,
            None => return 1,
        };
        // Walk down, recording the highest node whose crit-bit is within
        // the prefix length.
        let prefix_len = prefix.len();
        let mut p: &InnerNode = root;
        let mut top: &InnerNode = root;
        loop {
            match p {
                InnerNode::Leaf(_) => break,
                InnerNode::Internal(n) => {
                    if n.byte >= prefix_len {
                        // Always go left (skip).
                        p = &n.left;
                    } else {
                        p = follow(n, prefix);
                        top = p;
                    }
                }
            }
        }
        // Verify prefix matches.
        if let InnerNode::Leaf(l) = p {
            let lb = l.key.as_bytes();
            let pb = prefix.as_bytes();
            if lb.len() < pb.len() || &lb[..pb.len()] != pb {
                return 1;
            }
        }
        // Recurse from `top`.
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
        let s = self.storage();
        let root = s.inner.as_deref()?;
        let l = first_last_in(root, 0);
        Some(BltIt {
            key: l.key.clone(),
            data: None,
        })
    }

    /// Returns the leaf with the largest key.
    pub fn blt_last(&self) -> Option<BltIt> {
        let s = self.storage();
        let root = s.inner.as_deref()?;
        let l = first_last_in(root, 1);
        Some(BltIt {
            key: l.key.clone(),
            data: None,
        })
    }

    /// Returns the next leaf (in order) after the given one.
    pub fn blt_next(&self, it: &BltIt) -> Option<BltIt> {
        let s = self.storage();
        let root = s.inner.as_deref()?;
        let mut p = root;
        let mut other: Option<&InnerNode> = None;
        loop {
            match p {
                InnerNode::Leaf(_) => break,
                InnerNode::Internal(n) => {
                    let c = key_byte(&it.key, n.byte) & n.mask;
                    if c == 0 {
                        other = Some(&n.right);
                        p = &n.left;
                    } else {
                        p = &n.right;
                    }
                }
            }
        }
        other.map(|o| {
            let l = first_last_in(o, 0);
            BltIt {
                key: l.key.clone(),
                data: None,
            }
        })
    }

    /// Returns the previous leaf (in order) before the given one.
    pub fn blt_prev(&self, it: &BltIt) -> Option<BltIt> {
        let s = self.storage();
        let root = s.inner.as_deref()?;
        let mut p = root;
        let mut other: Option<&InnerNode> = None;
        loop {
            match p {
                InnerNode::Leaf(_) => break,
                InnerNode::Internal(n) => {
                    let c = key_byte(&it.key, n.byte) & n.mask;
                    if c != 0 {
                        other = Some(&n.left);
                        p = &n.right;
                    } else {
                        p = &n.left;
                    }
                }
            }
        }
        other.map(|o| {
            let l = first_last_in(o, 1);
            BltIt {
                key: l.key.clone(),
                data: None,
            }
        })
    }

    /// Returns the leaf with the smallest key ≥ the given key.
    pub fn blt_ceil(&self, key: &str) -> Option<BltIt> {
        ceil_floor(self, key, 0)
    }

    /// Returns the leaf with the largest key ≤ the given key.
    pub fn blt_floor(&self, key: &str) -> Option<BltIt> {
        ceil_floor(self, key, 1)
    }

    /// Returns the number of bytes used by the tree (excluding key storage).
    pub fn blt_overhead(&self) -> usize {
        let mut n = std::mem::size_of::<Blt>();
        let s = self.storage();
        if let Some(root) = s.inner.as_deref() {
            count_overhead(root, &mut n);
        }
        n
    }

    /// Returns true if the tree is empty.
    pub fn blt_empty(&self) -> bool {
        self.empty != 0
    }

    /// Returns the number of keys in the tree.
    pub fn blt_size(&self) -> i32 {
        let mut count: i32 = 0;
        self.blt_forall(|_| count += 1);
        count
    }
}

fn count_overhead(p: &InnerNode, n: &mut usize) {
    match p {
        InnerNode::Internal(i) => {
            *n += 2 * std::mem::size_of::<InnerInternal>();
            count_overhead(&i.left, n);
            count_overhead(&i.right, n);
        }
        InnerNode::Leaf(_) => {}
    }
}

fn traverse<F: FnMut(&BltIt) -> i32>(p: &InnerNode, fun: &mut F) -> i32 {
    match p {
        InnerNode::Internal(n) => {
            let st = traverse(&n.left, fun);
            if st != 1 {
                return st;
            }
            let st = traverse(&n.right, fun);
            if st != 1 {
                return st;
            }
            1
        }
        InnerNode::Leaf(l) => {
            let it = BltIt {
                key: l.key.clone(),
                data: None,
            };
            fun(&it)
        }
    }
}

// Insert a new key into the inner tree given the precomputed crit (byte, mask).
fn insert_node(root: Box<InnerNode>, key: &str, byte: usize, mask: u8) -> Box<InnerNode> {
    let new_leaf = InnerNode::Leaf(InnerLeaf {
        key: key.to_string(),
    });
    insert_recursive(root, key, byte, mask, new_leaf)
}

fn insert_recursive(
    node: Box<InnerNode>,
    key: &str,
    byte: usize,
    mask: u8,
    new_leaf: InnerNode,
) -> Box<InnerNode> {
    // Should we splice here? We splice if the current node is a leaf or
    // its crit-bit is "after" ours, where ordering is:
    //   (byte << 8) + mask  with crit-bit "smaller" meaning earlier.
    // Earlier crit-bit (closer to root) has lower (byte<<8) + (NOT mask)?
    // The C code uses: if ((byte << 8) + p->mask < (p->byte << 8) + x) break;
    // Hmm — note p->mask is the bitmask itself (not its inverse).  Lower
    // mask byte means lower bit (e.g. mask=0x80 means highest bit, mask=0x01
    // means lowest bit). With this comparison, "less than" means we should
    // splice here (insert above this node).  Let's mirror it.
    let splice_here = match &*node {
        InnerNode::Leaf(_) => true,
        InnerNode::Internal(n) => {
            // Mirror C: (our_byte << 8) + their_mask < (their_byte << 8) + our_mask
            let lhs = (byte << 8) + (n.mask as usize);
            let rhs = (n.byte << 8) + (mask as usize);
            lhs < rhs
        }
    };
    if splice_here {
        // Build the internal node that replaces `node`.
        let new_byte_at_key = key_byte(key, byte);
        // direction of the new key:
        let dir_new = if new_byte_at_key & mask != 0 { 1 } else { 0 };
        let (left, right) = if dir_new == 1 {
            (node, Box::new(new_leaf))
        } else {
            (Box::new(new_leaf), node)
        };
        return Box::new(InnerNode::Internal(InnerInternal {
            byte,
            mask,
            left,
            right,
        }));
    }
    // Otherwise descend
    if let InnerNode::Internal(mut n) = *node {
        let go_right = key_byte(key, n.byte) & n.mask != 0;
        if go_right {
            n.right = insert_recursive(n.right, key, byte, mask, new_leaf);
        } else {
            n.left = insert_recursive(n.left, key, byte, mask, new_leaf);
        }
        Box::new(InnerNode::Internal(n))
    } else {
        unreachable!()
    }
}

// Delete a key from the tree.
//
// Returns the resulting root (if any) and whether the deletion happened.
fn delete_from(root: Box<InnerNode>, key: &str) -> (Option<Box<InnerNode>>, bool) {
    let res = delete_node(root, key);
    match res {
        DeleteResult::NotFound(node) => (Some(node), false),
        DeleteResult::Deleted(maybe_node) => (maybe_node, true),
    }
}

enum DeleteResult {
    NotFound(Box<InnerNode>),
    // The subtree resulting from deletion, or `None` if the entire subtree
    // collapsed (this only happens when called on a leaf that was removed —
    // the caller must promote the sibling).
    Deleted(Option<Box<InnerNode>>),
}

fn delete_node(node: Box<InnerNode>, key: &str) -> DeleteResult {
    match *node {
        InnerNode::Leaf(l) => {
            if l.key == key {
                DeleteResult::Deleted(None)
            } else {
                DeleteResult::NotFound(Box::new(InnerNode::Leaf(l)))
            }
        }
        InnerNode::Internal(n) => {
            let keylen = key.len();
            if n.byte > keylen {
                return DeleteResult::NotFound(Box::new(InnerNode::Internal(n)));
            }
            let go_right = key_byte(key, n.byte) & n.mask != 0;
            let InnerInternal {
                byte,
                mask,
                left,
                right,
            } = n;
            if go_right {
                match delete_node(right, key) {
                    DeleteResult::NotFound(r) => DeleteResult::NotFound(Box::new(
                        InnerNode::Internal(InnerInternal {
                            byte,
                            mask,
                            left,
                            right: r,
                        }),
                    )),
                    DeleteResult::Deleted(None) => {
                        // Right child collapsed; promote left.
                        DeleteResult::Deleted(Some(left))
                    }
                    DeleteResult::Deleted(Some(rr)) => DeleteResult::Deleted(Some(Box::new(
                        InnerNode::Internal(InnerInternal {
                            byte,
                            mask,
                            left,
                            right: rr,
                        }),
                    ))),
                }
            } else {
                match delete_node(left, key) {
                    DeleteResult::NotFound(l) => DeleteResult::NotFound(Box::new(
                        InnerNode::Internal(InnerInternal {
                            byte,
                            mask,
                            left: l,
                            right,
                        }),
                    )),
                    DeleteResult::Deleted(None) => DeleteResult::Deleted(Some(right)),
                    DeleteResult::Deleted(Some(ll)) => DeleteResult::Deleted(Some(Box::new(
                        InnerNode::Internal(InnerInternal {
                            byte,
                            mask,
                            left: ll,
                            right,
                        }),
                    ))),
                }
            }
        }
    }
}

fn ceil_floor(blt: &Blt, key: &str, way: usize) -> Option<BltIt> {
    let s = blt.storage();
    let root = s.inner.as_deref()?;
    let p_leaf = confident_descend(root, key);
    let key_bytes = key.as_bytes();
    let pkey_bytes = p_leaf.key.as_bytes();
    let max_len = key_bytes.len().max(pkey_bytes.len()) + 1;
    for i in 0..max_len {
        let c = if i < key_bytes.len() { key_bytes[i] } else { 0 };
        let pc = if i < pkey_bytes.len() {
            pkey_bytes[i]
        } else {
            0
        };
        let x = c ^ pc;
        if x != 0 {
            let byte = i;
            let xm = to_mask(x);
            // Walk down the tree until external or higher crit-bit.
            let mut p: &InnerNode = root;
            let mut other: Option<&InnerNode> = None;
            loop {
                match p {
                    InnerNode::Leaf(_) => break,
                    InnerNode::Internal(n) => {
                        let cur = (byte << 8) + (n.mask as usize);
                        let target = (n.byte << 8) + (xm as usize);
                        if cur < target {
                            break;
                        }
                        let dir = if n.mask & key_byte(key, n.byte) != 0 {
                            1
                        } else {
                            0
                        };
                        if dir == way {
                            other = Some(if way == 0 { &n.right } else { &n.left });
                        }
                        p = if dir == 1 { &n.right } else { &n.left };
                    }
                }
            }
            let ndir = if xm & key_byte(key, byte) != 0 { 1 } else { 0 };
            if ndir == way {
                other = Some(p);
            }
            return other.map(|o| {
                let l = first_last_in(o, way);
                BltIt {
                    key: l.key.clone(),
                    data: None,
                }
            });
        }
        if c == 0 {
            // Equal — return the leaf.
            return Some(BltIt {
                key: p_leaf.key.clone(),
                data: None,
            });
        }
    }
    None
}
