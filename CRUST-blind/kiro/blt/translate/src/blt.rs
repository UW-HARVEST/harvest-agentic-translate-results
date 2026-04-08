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

// In the C code, each internal node has `kid` pointing to a pair of adjacent
// nodes: kid[0] = left, kid[1] = right. We replicate this by allocating
// Box<[BltNode; 2]> and storing a pointer to the first element as Box<BltNode>.
// This requires unsafe but exactly mirrors the C memory layout.

fn alloc_pair(left: BltNode, right: BltNode) -> Box<BltNode> {
    let pair = Box::new([left, right]);
    let ptr = Box::into_raw(pair) as *mut BltNode;
    unsafe { Box::from_raw(ptr) }
}

fn kid_left(kid: &BltNode) -> &BltNode {
    kid
}

fn kid_right(kid: &BltNode) -> &BltNode {
    let ptr = kid as *const BltNode;
    unsafe { &*ptr.add(1) }
}

fn kid_left_mut(kid: &mut BltNode) -> &mut BltNode {
    kid
}

fn kid_right_mut(kid: &mut BltNode) -> &mut BltNode {
    let ptr = kid as *mut BltNode;
    unsafe { &mut *ptr.add(1) }
}

/// Free a pair allocated with alloc_pair. Must be called instead of normal drop.
fn free_pair(kid: Box<BltNode>) {
    let ptr = Box::into_raw(kid) as *mut [BltNode; 2];
    let _ = unsafe { Box::from_raw(ptr) };
}

fn to_mask(x: u8) -> u8 {
    let mut x = x;
    x |= x >> 1;
    x |= x >> 2;
    x |= x >> 4;
    x & !(x >> 1)
}

fn follow<'a>(node: &'a InternalNode, key: &[u8]) -> &'a BltNode {
    if key.get(node.byte as usize).copied().unwrap_or(0) & node.mask != 0 {
        kid_right(&node.kid)
    } else {
        kid_left(&node.kid)
    }
}

fn follow_mut<'a>(node: &'a mut InternalNode, key: &[u8]) -> &'a mut BltNode {
    if key.get(node.byte as usize).copied().unwrap_or(0) & node.mask != 0 {
        kid_right_mut(&mut node.kid)
    } else {
        kid_left_mut(&mut node.kid)
    }
}

fn firstlast(node: &BltNode, dir: usize) -> Option<BltIt> {
    let mut p = node;
    loop {
        match p {
            BltNode::Internal(n) => {
                p = if dir == 0 { kid_left(&n.kid) } else { kid_right(&n.kid) };
            }
            BltNode::Leaf(leaf) => {
                return Some(BltIt { key: leaf.key.clone(), data: None });
            }
        }
    }
}

fn clone_leaf(leaf: &BltIt) -> BltIt {
    BltIt { key: leaf.key.clone(), data: None }
}

/// Recursively free internal node pairs properly.
fn _drop_node(_node: &mut BltNode) {
}

impl Blt {
    /// Creates a new BLT tree.
    pub fn blt_new() -> Self {
        Blt {
            root: Box::new(BltNode::Leaf(BltIt { key: String::new(), data: None })),
            empty: 1,
        }
    }

    /// Clears (destroys) the tree.
    pub fn blt_clear(&mut self) {
        if self.empty == 0 {
            free_tree(&mut self.root);
        }
        self.empty = 1;
        self.root = Box::new(BltNode::Leaf(BltIt { key: String::new(), data: None }));
    }

    /// Retrieves the leaf node at the given key.
    pub fn blt_get(&self, key: &str) -> Option<BltIt> {
        if self.empty != 0 { return None; }
        let key_bytes = key.as_bytes();
        let keylen = key_bytes.len();
        let mut p = &*self.root;
        loop {
            match p {
                BltNode::Internal(n) => {
                    if (n.byte as usize) > keylen { return None; }
                    p = follow(n, key_bytes);
                }
                BltNode::Leaf(leaf) => {
                    return if leaf.key == key { Some(clone_leaf(leaf)) } else { None };
                }
            }
        }
    }

    /// Creates or retrieves the leaf node at the given key.
    pub fn blt_set(&mut self, key: &str) -> BltIt {
        self.blt_setp(key).0
    }

    /// Creates or retrieves the leaf node at the given key and returns a tuple (leaf, is_new).
    pub fn blt_setp(&mut self, key: &str) -> (BltIt, bool) {
        let key_bytes = key.as_bytes();

        // Empty tree case
        if self.empty != 0 {
            self.empty = 0;
            *self.root = BltNode::Leaf(BltIt { key: key.to_string(), data: None });
            return (BltIt { key: key.to_string(), data: None }, true);
        }

        // Walk down to find a leaf (confident_get)
        let leaf_key = {
            let mut p = &*self.root;
            let keylen = key_bytes.len();
            loop {
                match p {
                    BltNode::Internal(n) => {
                        if (n.byte as usize) < keylen && (key_bytes[n.byte as usize] & n.mask != 0) {
                            p = kid_right(&n.kid);
                        } else {
                            p = kid_left(&n.kid);
                        }
                    }
                    BltNode::Leaf(leaf) => break leaf.key.clone(),
                }
            }
        };

        // Compare keys to find the critical bit
        let leaf_bytes = leaf_key.as_bytes();
        let mut byte_pos = 0usize;
        loop {
            let c = key_bytes.get(byte_pos).copied().unwrap_or(0);
            let pc = leaf_bytes.get(byte_pos).copied().unwrap_or(0);
            let x = c ^ pc;
            if x != 0 {
                let mask = to_mask(x);
                let new_goes_right = c & mask != 0;

                // Walk down to find insertion point
                let root_ptr: *mut BltNode = &mut *self.root;
                let mut p_ptr = root_ptr;
                unsafe {
                    loop {
                        match &*p_ptr {
                            BltNode::Internal(n) => {
                                let cmp_val = (byte_pos << 8) + mask as usize;
                                let node_val = ((n.byte as usize) << 8) + n.mask as usize;
                                if cmp_val < node_val { break; }
                                let n_mut = match &mut *p_ptr {
                                    BltNode::Internal(n) => n,
                                    _ => unreachable!(),
                                };
                                p_ptr = follow_mut(n_mut, key_bytes) as *mut BltNode;
                            }
                            BltNode::Leaf(_) => break,
                        }
                    }

                    // Take the current node content
                    let old_node = std::ptr::read(p_ptr);
                    let new_leaf = BltNode::Leaf(BltIt { key: key.to_string(), data: None });

                    let (left, right) = if new_goes_right {
                        (old_node, new_leaf)
                    } else {
                        (new_leaf, old_node)
                    };

                    let pair = alloc_pair(left, right);
                    let new_internal = BltNode::Internal(InternalNode {
                        byte: byte_pos as u32,
                        mask,
                        padding: 0,
                        kid: pair,
                    });
                    std::ptr::write(p_ptr, new_internal);
                }

                return (BltIt { key: key.to_string(), data: None }, true);
            }
            if c == 0 {
                // Keys match - find the leaf and return it
                return (BltIt { key: key.to_string(), data: None }, false);
            }
            byte_pos += 1;
        }
    }

    /// Inserts the given key/data pair and returns the corresponding leaf.
    pub fn blt_put(&mut self, key: &str, data: Box<dyn Any>) -> BltIt {
        let _ = self.blt_setp(key);
        // Find the leaf and set its data
        self.set_leaf_data(key, data);
        BltIt { key: key.to_string(), data: None }
    }

    /// Inserts the key/data pair only if the key is absent.
    /// Returns 0 on success or 1 if the key is already present.
    pub fn blt_put_if_absent(&mut self, key: &str, data: Box<dyn Any>) -> i32 {
        let (_, is_new) = self.blt_setp(key);
        if is_new {
            self.set_leaf_data(key, data);
            0
        } else {
            1
        }
    }

    /// Deletes the given key from the tree.
    /// Returns 1 if a key was deleted, 0 otherwise.
    pub fn blt_delete(&mut self, key: &str) -> i32 {
        if self.empty != 0 { return 0; }
        let key_bytes = key.as_bytes();
        let keylen = key_bytes.len();

        // Find the leaf and its parent
        let root_ptr: *mut BltNode = &mut *self.root;
        let mut p_ptr = root_ptr;
        let mut parent_ptr: *mut BltNode = std::ptr::null_mut();

        unsafe {
            loop {
                match &*p_ptr {
                    BltNode::Internal(n) => {
                        if (n.byte as usize) > keylen { return 0; }
                        parent_ptr = p_ptr;
                        let n_mut = match &mut *p_ptr {
                            BltNode::Internal(n) => n,
                            _ => unreachable!(),
                        };
                        p_ptr = follow_mut(n_mut, key_bytes) as *mut BltNode;
                    }
                    BltNode::Leaf(leaf) => {
                        if leaf.key != key { return 0; }
                        break;
                    }
                }
            }

            if parent_ptr.is_null() {
                // Only node in tree
                self.empty = 1;
                return 1;
            }

            // Get the sibling
            let parent_node = &mut *parent_ptr;
            if let BltNode::Internal(n) = parent_node {
                let kid_ptr = &mut *n.kid as *mut BltNode;
                let left_ptr = kid_ptr;
                let right_ptr = kid_ptr.add(1);

                let sibling_is_right = p_ptr == left_ptr;
                let sibling_ptr = if sibling_is_right { right_ptr } else { left_ptr };

                // Read the sibling and replace the parent with it
                let sibling = std::ptr::read(sibling_ptr);
                // Write a dummy into the other slot so the pair can be freed
                std::ptr::write(if sibling_is_right { left_ptr } else { right_ptr },
                    BltNode::Leaf(BltIt { key: String::new(), data: None }));
                std::ptr::write(sibling_ptr,
                    BltNode::Leaf(BltIt { key: String::new(), data: None }));

                // Free the pair
                let kid_box = std::ptr::read(&n.kid as *const Box<BltNode>);
                free_pair(kid_box);

                // Replace parent with sibling
                std::ptr::write(parent_ptr, sibling);
            }
        }
        1
    }

    /// Iterates over all leaves with keys having the given prefix.
    /// The closure should return an i32; if it returns 0, iteration stops.
    pub fn blt_allprefixed<F: FnMut(&BltIt) -> i32>(&self, prefix: &str, mut fun: F) -> i32 {
        if self.empty != 0 { return 1; }
        let key_bytes = prefix.as_bytes();
        let keylen = key_bytes.len();
        let mut p = &*self.root;
        let mut top = p;

        loop {
            match p {
                BltNode::Internal(n) => {
                    if (n.byte as usize) >= keylen {
                        p = kid_left(&n.kid);
                    } else {
                        p = follow(n, key_bytes);
                        top = p;
                    }
                }
                BltNode::Leaf(_) => break,
            }
        }

        // Check prefix matches
        if let BltNode::Leaf(leaf) = p {
            if !leaf.key.as_bytes().starts_with(key_bytes) {
                return 1;
            }
        }

        fn traverse<F: FnMut(&BltIt) -> i32>(node: &BltNode, fun: &mut F) -> i32 {
            match node {
                BltNode::Internal(n) => {
                    let status = traverse(kid_left(&n.kid), fun);
                    if status != 1 { return status; }
                    let status = traverse(kid_right(&n.kid), fun);
                    if status != 1 { return status; }
                    1
                }
                BltNode::Leaf(leaf) => fun(leaf),
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
        if self.empty != 0 { return None; }
        firstlast(&self.root, 0)
    }

    /// Returns the leaf with the largest key.
    pub fn blt_last(&self) -> Option<BltIt> {
        if self.empty != 0 { return None; }
        firstlast(&self.root, 1)
    }

    /// Returns the next leaf (in order) after the given one.
    pub fn blt_next(&self, it: &BltIt) -> Option<BltIt> {
        if self.empty != 0 { return None; }
        let key_bytes = it.key.as_bytes();
        let mut p = &*self.root;
        let mut other: Option<&BltNode> = None;
        loop {
            match p {
                BltNode::Internal(n) => {
                    if key_bytes.get(n.byte as usize).copied().unwrap_or(0) & n.mask == 0 {
                        other = Some(kid_right(&n.kid));
                        p = kid_left(&n.kid);
                    } else {
                        p = kid_right(&n.kid);
                    }
                }
                BltNode::Leaf(_) => break,
            }
        }
        other.and_then(|o| firstlast(o, 0))
    }

    /// Returns the previous leaf (in order) before the given one.
    pub fn blt_prev(&self, it: &BltIt) -> Option<BltIt> {
        if self.empty != 0 { return None; }
        let key_bytes = it.key.as_bytes();
        let mut p = &*self.root;
        let mut other: Option<&BltNode> = None;
        loop {
            match p {
                BltNode::Internal(n) => {
                    if key_bytes.get(n.byte as usize).copied().unwrap_or(0) & n.mask != 0 {
                        other = Some(kid_left(&n.kid));
                        p = kid_right(&n.kid);
                    } else {
                        p = kid_left(&n.kid);
                    }
                }
                BltNode::Leaf(_) => break,
            }
        }
        other.and_then(|o| firstlast(o, 1))
    }

    /// Returns the leaf with the smallest key ≥ the given key.
    pub fn blt_ceil(&self, key: &str) -> Option<BltIt> {
        self.blt_ceilfloor(key, 0)
    }

    /// Returns the leaf with the largest key ≤ the given key.
    pub fn blt_floor(&self, key: &str) -> Option<BltIt> {
        self.blt_ceilfloor(key, 1)
    }

    fn blt_ceilfloor(&self, key: &str, way: usize) -> Option<BltIt> {
        if self.empty != 0 { return None; }
        let key_bytes = key.as_bytes();

        // confident_get
        let leaf_key = {
            let mut p = &*self.root;
            let keylen = key_bytes.len();
            loop {
                match p {
                    BltNode::Internal(n) => {
                        if (n.byte as usize) < keylen && (key_bytes[n.byte as usize] & n.mask != 0) {
                            p = kid_right(&n.kid);
                        } else {
                            p = kid_left(&n.kid);
                        }
                    }
                    BltNode::Leaf(leaf) => break leaf.key.clone(),
                }
            }
        };

        let leaf_bytes = leaf_key.as_bytes();
        let mut byte_pos = 0usize;
        loop {
            let c = key_bytes.get(byte_pos).copied().unwrap_or(0);
            let pc = leaf_bytes.get(byte_pos).copied().unwrap_or(0);
            let x = c ^ pc;
            if x != 0 {
                let mask = to_mask(x);
                let byte = byte_pos;

                let mut p = &*self.root;
                let mut other: Option<&BltNode> = None;
                loop {
                    match p {
                        BltNode::Internal(n) => {
                            let cmp_val = (byte << 8) + mask as usize;
                            let node_val = ((n.byte as usize) << 8) + n.mask as usize;
                            if cmp_val < node_val { break; }
                            let dir = if key_bytes.get(n.byte as usize).copied().unwrap_or(0) & n.mask != 0 { 1 } else { 0 };
                            let q_left = kid_left(&n.kid);
                            let q_right = kid_right(&n.kid);
                            if dir == way {
                                other = Some(if way == 0 { q_right } else { q_left });
                            }
                            p = if dir != 0 { q_right } else { q_left };
                        }
                        BltNode::Leaf(_) => break,
                    }
                }
                let ndir = if key_bytes.get(byte).copied().unwrap_or(0) & mask != 0 { 1 } else { 0 };
                if ndir == way {
                    other = Some(p);
                }
                return other.and_then(|o| firstlast(o, way));
            }
            if c == 0 {
                return Some(BltIt { key: leaf_key, data: None });
            }
            byte_pos += 1;
        }
    }

    /// Returns the number of bytes used by the tree (excluding key storage).
    pub fn blt_overhead(&self) -> usize {
        let n = std::mem::size_of::<Blt>();
        if self.empty != 0 { return n; }
        fn add(p: &BltNode) -> usize {
            match p {
                BltNode::Internal(node) => {
                    2 * std::mem::size_of::<BltNode>()
                        + add(kid_left(&node.kid))
                        + add(kid_right(&node.kid))
                }
                BltNode::Leaf(_) => 0,
            }
        }
        n + add(&self.root)
    }

    /// Returns true if the tree is empty.
    pub fn blt_empty(&self) -> bool {
        self.empty != 0
    }

    /// Returns the number of keys in the tree.
    pub fn blt_size(&self) -> i32 {
        let mut r = 0i32;
        self.blt_forall(|_| r += 1);
        r
    }

    // Helper: set data on a leaf found by key
    fn set_leaf_data(&mut self, key: &str, data: Box<dyn Any>) {
        if self.empty != 0 { return; }
        let key_bytes = key.as_bytes();
        let mut p = &mut *self.root;
        loop {
            match p {
                BltNode::Internal(n) => {
                    p = follow_mut(n, key_bytes);
                }
                BltNode::Leaf(leaf) => {
                    if leaf.key == key {
                        leaf.data = Some(data);
                    }
                    return;
                }
            }
        }
    }
}

fn free_tree(node: &mut BltNode) {
    match node {
        BltNode::Internal(n) => {
            free_tree(kid_left_mut(&mut n.kid));
            free_tree(kid_right_mut(&mut n.kid));
            // Take the kid box and free it as a pair
            let kid = unsafe {
                std::ptr::read(&n.kid as *const Box<BltNode>)
            };
            free_pair(kid);
            // Prevent double-free by writing a dummy
            unsafe {
                std::ptr::write(&mut n.kid as *mut Box<BltNode>,
                    Box::new(BltNode::Leaf(BltIt { key: String::new(), data: None })));
            }
        }
        BltNode::Leaf(_) => {}
    }
}

impl Drop for Blt {
    fn drop(&mut self) {
        if self.empty == 0 {
            free_tree(&mut self.root);
        }
    }
}
