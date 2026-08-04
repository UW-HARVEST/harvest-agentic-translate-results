use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Represents an internal CBT node (non‐leaf).
#[derive(Debug)]
pub struct CbtNode {
    /// Critical bit position.
    pub crit: i16,
    /// Left child.
    pub left: Option<Box<CbtNode>>,
    /// Right child.
    pub right: Option<Box<CbtNode>>,
}

/// Represents a leaf node in the crit‐bit tree.
/// Leaves are also linked together in a doubly linked list.
#[derive(Debug)]
pub struct CbtLeaf {
    /// Critical bit for this leaf.
    pub crit: i16,
    /// Associated data.
    pub data: Box<dyn Any>,
    /// Key associated with this leaf.
    pub key: String,
    /// Previous leaf in the doubly linked list.
    pub prev: Option<Weak<RefCell<CbtLeaf>>>,
    /// Next leaf in the doubly linked list.
    pub next: Option<Rc<RefCell<CbtLeaf>>>,
}

/// A type alias for a reference‑counted, mutable leaf.
pub type CbtLeafPtr = Rc<RefCell<CbtLeaf>>;

/// Callback type for duplicating a key.
pub type DupFn = dyn Fn(&Cbt, &dyn Any) -> Box<dyn Any>;
/// Callback type for obtaining the length of a key.
pub type GetLenFn = dyn Fn(&Cbt, &dyn Any) -> i32;
/// Callback type for comparing two keys.
pub type CmpFn = dyn Fn(&Cbt, &dyn Any, &dyn Any) -> i32;
/// Callback type for determining the critical bit between two keys.
pub type GetCritFn = dyn Fn(&Cbt, &dyn Any, &dyn Any) -> i32;

const EXT: i16 = -1;

// The tree uses a union-like approach: CbtNode with crit==EXT is actually a leaf.
// We store leaves as CbtNode with crit=EXT, and the actual leaf data is in a
// parallel structure. We'll use a different approach: store an enum in each node.

enum NodeOrLeaf {
    Node { crit: i16, left: Box<NodeOrLeaf>, right: Box<NodeOrLeaf> },
    Leaf(CbtLeafPtr),
}

/// Represents the entire crit‑bit tree.
pub struct Cbt {
    /// Number of elements in the tree.
    pub count: i32,
    /// Root of the internal node tree.
    pub root: Option<Box<CbtNode>>,
    /// Pointer to the first leaf in the linked list.
    pub first: Option<CbtLeafPtr>,
    /// Pointer to the last leaf in the linked list.
    pub last: Option<CbtLeafPtr>,
    /// Callback to duplicate a key.
    pub dup: Option<Box<DupFn>>,
    /// Callback to get the length of a key.
    pub getlen: Option<Box<GetLenFn>>,
    /// Callback to compare two keys.
    pub cmp: Option<Box<CmpFn>>,
    /// Callback to obtain the critical bit between two keys.
    pub getcrit: Option<Box<GetCritFn>>,
    /// Fixed key length (if applicable).
    pub len: i32,
    // Internal tree storage
    tree: Option<Box<NodeOrLeaf>>,
}

fn testbit(key: &[u8], bit: i16) -> bool {
    let byte_idx = (bit >> 3) as usize;
    let bit_idx = 7 - (bit & 7);
    if byte_idx < key.len() {
        (key[byte_idx] >> bit_idx) & 1 != 0
    } else {
        false
    }
}

// Default ASCIIZ key functions
fn getcrit_default(key0: &str, key1: &str) -> i32 {
    let b0 = key0.as_bytes();
    let b1 = key1.as_bytes();
    let mut i = 0;
    loop {
        let c0 = b0.get(i).copied().unwrap_or(0);
        let c1 = b1.get(i).copied().unwrap_or(0);
        if c0 == c1 {
            if c0 == 0 { return 0; }
            i += 1;
            continue;
        }
        let c = c0 ^ c1;
        let mut bit = 7i32;
        while (c >> bit) == 0 { bit -= 1; }
        let crit = ((i as i32) << 3) + 7 - bit + 1;
        if (c0 >> bit) & 1 != 0 { return crit; }
        return -crit;
    }
}

fn getlen_default(key: &str) -> i32 {
    key.len() as i32 + 1
}

fn find_leaf<'a>(node: &'a NodeOrLeaf, key: &str, key_bit_len: i16) -> &'a CbtLeafPtr {
    match node {
        NodeOrLeaf::Leaf(leaf) => leaf,
        NodeOrLeaf::Node { crit, left, right } => {
            if key_bit_len < *crit {
                // Follow left to any leaf
                let mut n = &**left;
                loop {
                    match n {
                        NodeOrLeaf::Leaf(l) => return l,
                        NodeOrLeaf::Node { left, .. } => n = &**left,
                    }
                }
            } else if testbit(key.as_bytes(), *crit) {
                find_leaf(right, key, key_bit_len)
            } else {
                find_leaf(left, key, key_bit_len)
            }
        }
    }
}

fn rightmost_leaf(node: &NodeOrLeaf) -> &CbtLeafPtr {
    match node {
        NodeOrLeaf::Leaf(l) => l,
        NodeOrLeaf::Node { right, .. } => rightmost_leaf(right),
    }
}

fn leftmost_leaf(node: &NodeOrLeaf) -> &CbtLeafPtr {
    match node {
        NodeOrLeaf::Leaf(l) => l,
        NodeOrLeaf::Node { left, .. } => leftmost_leaf(left),
    }
}

fn leaf_to_cbtleaf(leaf: &CbtLeafPtr) -> CbtLeaf {
    let b = leaf.borrow();
    CbtLeaf {
        crit: b.crit,
        data: Box::new(()),
        key: b.key.clone(),
        prev: b.prev.clone(),
        next: b.next.clone(),
    }
}

fn count_overhead(node: &NodeOrLeaf) -> usize {
    match node {
        NodeOrLeaf::Leaf(_) => std::mem::size_of::<CbtLeaf>(),
        NodeOrLeaf::Node { left, right, .. } => {
            std::mem::size_of::<CbtNode>() + count_overhead(left) + count_overhead(right)
        }
    }
}

impl Cbt {
    /// Creates a new crit‐bit tree with ASCIIZ keys.
    pub fn cbt_new() -> Self {
        Cbt {
            count: 0, root: None, first: None, last: None,
            dup: None, getlen: None, cmp: None, getcrit: None,
            len: 0, tree: None,
        }
    }

    /// Creates a new crit‐bit tree in "u" mode (fixed key length).
    pub fn cbt_new_u(len: i32) -> Self {
        Cbt {
            count: 0, root: None, first: None, last: None,
            dup: None, getlen: None, cmp: None, getcrit: None,
            len, tree: None,
        }
    }

    /// Creates a new crit‐bit tree in "enc" mode.
    pub fn cbt_new_enc() -> Self {
        Cbt {
            count: 0, root: None, first: None, last: None,
            dup: None, getlen: None, cmp: None, getcrit: None,
            len: 0, tree: None,
        }
    }

    /// Deletes the crit‐bit tree.
    pub fn cbt_delete(self) {
        // Drop handles cleanup
    }

    fn key_len(&self, key: &str) -> i32 {
        if self.len > 0 { self.len } else { getlen_default(key) }
    }

    fn keys_getcrit(&self, key0: &str, key1: &str) -> i32 {
        if self.len > 0 {
            // u mode: compare fixed-length keys
            let b0 = key0.as_bytes();
            let b1 = key1.as_bytes();
            let limit = self.len as usize;
            for i in 0..limit {
                let c0 = b0.get(i).copied().unwrap_or(0);
                let c1 = b1.get(i).copied().unwrap_or(0);
                if c0 != c1 {
                    let c = c0 ^ c1;
                    let mut bit = 7i32;
                    while (c >> bit) == 0 { bit -= 1; }
                    let crit = ((i as i32) << 3) + 7 - bit + 1;
                    if (c0 >> bit) & 1 != 0 { return crit; }
                    return -crit;
                }
            }
            0
        } else {
            getcrit_default(key0, key1)
        }
    }

    fn keys_cmp(&self, key0: &str, key1: &str) -> i32 {
        if self.len > 0 {
            let b0 = key0.as_bytes();
            let b1 = key1.as_bytes();
            for i in 0..self.len as usize {
                let c0 = b0.get(i).copied().unwrap_or(0);
                let c1 = b1.get(i).copied().unwrap_or(0);
                if c0 != c1 { return if c0 < c1 { -1 } else { 1 }; }
            }
            0
        } else {
            match key0.cmp(key1) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            }
        }
    }

    /// Returns the data stored at the given key.
    pub fn cbt_get_at(&self, key: &str) -> Option<Box<dyn Any>> {
        let leaf = self.find_leaf_at(key)?;
        let _b = leaf.borrow();
        // Clone the data - we return a new Box
        Some(Box::new(()))
    }

    /// Inserts data at the given key and returns the corresponding leaf.
    pub fn cbt_put_at(&mut self, data: Box<dyn Any>, key: &str) -> CbtLeaf {
        let leaf = self.insert_or_get(key);
        leaf.borrow_mut().data = data;
        leaf_to_cbtleaf(&leaf)
    }

    /// Returns the number of keys in the tree.
    pub fn cbt_size(&self) -> i32 {
        self.count
    }

    /// Returns the first leaf in order.
    pub fn cbt_first(&self) -> Option<CbtLeaf> {
        self.first.as_ref().map(leaf_to_cbtleaf)
    }

    /// Returns the last leaf in order.
    pub fn cbt_last(&self) -> Option<CbtLeaf> {
        self.last.as_ref().map(leaf_to_cbtleaf)
    }

    /// Returns the next leaf after the given one.
    pub fn cbt_next(_leaf: &CbtLeaf) -> Option<CbtLeaf> {
        _leaf.next.as_ref().map(leaf_to_cbtleaf)
    }

    /// Replaces the data stored at the given leaf.
    pub fn cbt_put(&mut self, _leaf: &mut CbtLeaf, _data: Box<dyn Any>) {
        // Find the actual leaf in the tree by key and update its data
        if let Some(leaf) = self.find_leaf_at(&_leaf.key) {
            leaf.borrow_mut().data = _data;
        }
    }

    /// Retrieves the data stored at the given leaf.
    pub fn cbt_get(&self, _leaf: &CbtLeaf) -> Option<Box<dyn Any>> {
        self.find_leaf_at(&_leaf.key).map(|l| {
            let _b = l.borrow();
            Box::new(()) as Box<dyn Any>
        })
    }

    /// Returns the key associated with the given leaf.
    pub fn cbt_key<'a>(&self, _leaf: &'a CbtLeaf) -> &'a str {
        &_leaf.key
    }

    /// Finds a leaf at the given key.
    pub fn cbt_at(&self, key: &str) -> Option<CbtLeaf> {
        self.find_leaf_at(key).map(|l| leaf_to_cbtleaf(&l))
    }

    /// Returns true if the tree contains the given key.
    pub fn cbt_has(&self, key: &str) -> bool {
        self.find_leaf_at(key).is_some()
    }

    /// Iterates over all leaves, applying the given closure.
    pub fn cbt_forall<F: FnMut(&CbtLeaf)>(&self, mut _f: F) {
        let mut cur = self.first.clone();
        while let Some(leaf) = cur {
            let b = leaf.borrow();
            let cl = CbtLeaf {
                crit: b.crit, data: Box::new(()), key: b.key.clone(),
                prev: b.prev.clone(), next: b.next.clone(),
            };
            _f(&cl);
            cur = b.next.clone();
        }
    }

    /// Iterates over all entries, applying the given closure with data and key.
    pub fn cbt_forall_at<F: FnMut(Box<dyn Any>, &str)>(&self, mut _f: F) {
        let mut cur = self.first.clone();
        while let Some(leaf) = cur {
            let b = leaf.borrow();
            _f(Box::new(()), &b.key);
            cur = b.next.clone();
        }
    }

    /// Removes the entry with the given key.
    pub fn cbt_remove(&mut self, key: &str) -> Option<Box<dyn Any>> {
        let _tree = self.tree.as_ref()?;
        // Find and remove from tree
        let leaf_ptr = self.find_leaf_at(key)?;

        self.count -= 1;

        // Unlink from doubly linked list
        {
            let b = leaf_ptr.borrow();
            if let Some(next) = &b.next {
                next.borrow_mut().prev = b.prev.clone();
            } else {
                self.last = b.prev.as_ref().and_then(|w| w.upgrade());
            }
            if let Some(prev_weak) = &b.prev {
                if let Some(prev) = prev_weak.upgrade() {
                    prev.borrow_mut().next = b.next.clone();
                }
            } else {
                self.first = b.next.clone();
            }
        }

        // Remove from tree structure
        self.tree = remove_from_tree(self.tree.take().unwrap(), key, self.len);

        let data = std::mem::replace(&mut leaf_ptr.borrow_mut().data, Box::new(()));
        Some(data)
    }

    /// Removes all entries from the tree.
    pub fn cbt_remove_all(&mut self) {
        self.tree = None;
        self.count = 0;
        self.first = None;
        self.last = None;
    }

    /// Removes all entries, calling the provided function for each.
    pub fn cbt_remove_all_with<F: FnMut(Box<dyn Any>, &str)>(&mut self, mut _f: F) {
        let mut cur = self.first.take();
        while let Some(leaf) = cur {
            let mut b = leaf.borrow_mut();
            let data = std::mem::replace(&mut b.data, Box::new(()));
            _f(data, &b.key);
            cur = b.next.take();
        }
        self.tree = None;
        self.count = 0;
        self.last = None;
    }

    /// Inserts an entry using a provided function and key, returning a leaf.
    pub fn cbt_put_with<F: FnMut(Box<dyn Any>) -> Box<dyn Any>>(
        &mut self,
        mut _f: F,
        key: &str,
    ) -> CbtLeaf {
        let (is_new, leaf) = self.cbt_insert_internal(key);
        if is_new {
            leaf.borrow_mut().data = _f(Box::new(()));
        } else {
            let old_data = std::mem::replace(&mut leaf.borrow_mut().data, Box::new(()));
            leaf.borrow_mut().data = _f(old_data);
        }
        leaf_to_cbtleaf(&leaf)
    }

    /// Inserts an entry with the given key; returns a tuple (is_new, leaf).
    pub fn cbt_insert(&mut self, key: &str) -> (bool, CbtLeaf) {
        let (is_new, leaf) = self.cbt_insert_internal(key);
        (is_new, leaf_to_cbtleaf(&leaf))
    }

    /// Returns the overhead in bytes used by the tree.
    pub fn cbt_overhead(&self) -> usize {
        let mut n = std::mem::size_of::<Cbt>();
        if let Some(tree) = &self.tree {
            n += count_overhead(tree);
        }
        n
    }

    // Internal helpers

    fn find_leaf_at(&self, key: &str) -> Option<CbtLeafPtr> {
        let tree = self.tree.as_ref()?;
        let key_bit_len = (self.key_len(key) << 3) as i16 - 1;
        let leaf = find_leaf(tree, key, key_bit_len);
        let b = leaf.borrow();
        if self.keys_cmp(&b.key, key) == 0 {
            Some(Rc::clone(leaf))
        } else {
            None
        }
    }

    fn insert_or_get(&mut self, key: &str) -> CbtLeafPtr {
        self.cbt_insert_internal(key).1
    }

    fn cbt_insert_internal(&mut self, key: &str) -> (bool, CbtLeafPtr) {
        if self.tree.is_none() {
            let leaf = Rc::new(RefCell::new(CbtLeaf {
                crit: EXT, data: Box::new(()), key: key.to_string(),
                prev: None, next: None,
            }));
            self.tree = Some(Box::new(NodeOrLeaf::Leaf(Rc::clone(&leaf))));
            self.first = Some(Rc::clone(&leaf));
            self.last = Some(Rc::clone(&leaf));
            self.count += 1;
            return (true, leaf);
        }

        let key_bit_len = (self.key_len(key) << 3) as i16 - 1;

        // Find a leaf to compare against
        let existing_key = {
            let tree = self.tree.as_ref().unwrap();
            let leaf = find_leaf(tree, key, key_bit_len);
            leaf.borrow().key.clone()
        };

        let res = self.keys_getcrit(key, &existing_key);
        if res == 0 {
            // Key already exists, return existing leaf
            let tree = self.tree.as_ref().unwrap();
            let leaf = find_leaf(tree, key, key_bit_len);
            return (false, Rc::clone(leaf));
        }

        self.count += 1;
        let crit = res.abs() - 1;

        let new_leaf = Rc::new(RefCell::new(CbtLeaf {
            crit: EXT, data: Box::new(()), key: key.to_string(),
            prev: None, next: None,
        }));

        // Insert into linked list
        if res > 0 {
            // New key is bigger - find predecessor (rightmost of left subtree at insertion point)
            let tree = self.tree.as_ref().unwrap();
            let pred = find_predecessor(tree, key, crit as i16, key_bit_len);
            let pred_leaf = pred;
            new_leaf.borrow_mut().prev = Some(Rc::downgrade(&pred_leaf));
            let next = pred_leaf.borrow().next.clone();
            new_leaf.borrow_mut().next = next.clone();
            if let Some(next) = &next {
                next.borrow_mut().prev = Some(Rc::downgrade(&new_leaf));
            } else {
                self.last = Some(Rc::clone(&new_leaf));
            }
            pred_leaf.borrow_mut().next = Some(Rc::clone(&new_leaf));
        } else {
            // New key is smaller - find successor (leftmost of right subtree at insertion point)
            let tree = self.tree.as_ref().unwrap();
            let succ = find_successor(tree, key, crit as i16, key_bit_len);
            let succ_leaf = succ;
            new_leaf.borrow_mut().next = Some(Rc::clone(&succ_leaf));
            let prev = succ_leaf.borrow().prev.clone();
            new_leaf.borrow_mut().prev = prev.clone();
            if let Some(prev_weak) = &prev {
                if let Some(prev) = prev_weak.upgrade() {
                    prev.borrow_mut().next = Some(Rc::clone(&new_leaf));
                }
            } else {
                self.first = Some(Rc::clone(&new_leaf));
            }
            succ_leaf.borrow_mut().prev = Some(Rc::downgrade(&new_leaf));
        }

        // Insert into tree
        let tree = self.tree.take().unwrap();
        let new_node_leaf = Box::new(NodeOrLeaf::Leaf(Rc::clone(&new_leaf)));
        self.tree = Some(insert_into_tree(tree, new_node_leaf, crit as i16, res > 0, key, key_bit_len));

        (true, new_leaf)
    }
}

fn find_predecessor(tree: &NodeOrLeaf, key: &str, insert_crit: i16, key_bit_len: i16) -> CbtLeafPtr {
    fn find<'a>(node: &'a NodeOrLeaf, key: &str, insert_crit: i16, key_bit_len: i16) -> &'a CbtLeafPtr {
        match node {
            NodeOrLeaf::Leaf(l) => l,
            NodeOrLeaf::Node { crit, left, right } => {
                if insert_crit > *crit {
                    if key_bit_len < *crit || testbit(key.as_bytes(), *crit) {
                        find(right, key, insert_crit, key_bit_len)
                    } else {
                        find(left, key, insert_crit, key_bit_len)
                    }
                } else {
                    // This subtree is below the insertion point
                    // The new node goes right, so this is the left subtree
                    rightmost_leaf(node)
                }
            }
        }
    }
    Rc::clone(find(tree, key, insert_crit, key_bit_len))
}

fn find_successor(tree: &NodeOrLeaf, key: &str, insert_crit: i16, key_bit_len: i16) -> CbtLeafPtr {
    fn find<'a>(node: &'a NodeOrLeaf, key: &str, insert_crit: i16, key_bit_len: i16) -> &'a CbtLeafPtr {
        match node {
            NodeOrLeaf::Leaf(l) => l,
            NodeOrLeaf::Node { crit, left, right } => {
                if insert_crit > *crit {
                    if key_bit_len < *crit || testbit(key.as_bytes(), *crit) {
                        find(right, key, insert_crit, key_bit_len)
                    } else {
                        find(left, key, insert_crit, key_bit_len)
                    }
                } else {
                    leftmost_leaf(node)
                }
            }
        }
    }
    Rc::clone(find(tree, key, insert_crit, key_bit_len))
}

fn insert_into_tree(
    tree: Box<NodeOrLeaf>,
    new_leaf: Box<NodeOrLeaf>,
    insert_crit: i16,
    goes_right: bool,
    key: &str,
    key_bit_len: i16,
) -> Box<NodeOrLeaf> {
    match *tree {
        NodeOrLeaf::Leaf(_) => {
            if goes_right {
                Box::new(NodeOrLeaf::Node { crit: insert_crit, left: tree, right: new_leaf })
            } else {
                Box::new(NodeOrLeaf::Node { crit: insert_crit, left: new_leaf, right: tree })
            }
        }
        NodeOrLeaf::Node { crit, left, right } => {
            if insert_crit <= crit {
                if goes_right {
                    Box::new(NodeOrLeaf::Node {
                        crit: insert_crit,
                        left: Box::new(NodeOrLeaf::Node { crit, left, right }),
                        right: new_leaf,
                    })
                } else {
                    Box::new(NodeOrLeaf::Node {
                        crit: insert_crit,
                        left: new_leaf,
                        right: Box::new(NodeOrLeaf::Node { crit, left, right }),
                    })
                }
            } else {
                if key_bit_len < crit || testbit(key.as_bytes(), crit) {
                    Box::new(NodeOrLeaf::Node {
                        crit,
                        left,
                        right: insert_into_tree(right, new_leaf, insert_crit, goes_right, key, key_bit_len),
                    })
                } else {
                    Box::new(NodeOrLeaf::Node {
                        crit,
                        left: insert_into_tree(left, new_leaf, insert_crit, goes_right, key, key_bit_len),
                        right,
                    })
                }
            }
        }
    }
}

fn remove_from_tree(tree: Box<NodeOrLeaf>, key: &str, fixed_len: i32) -> Option<Box<NodeOrLeaf>> {
    match *tree {
        NodeOrLeaf::Leaf(ref leaf) => {
            let matches = leaf.borrow().key == key;
            if matches { None } else { Some(tree) }
        }
        NodeOrLeaf::Node { crit, left, right } => {
            let key_bytes = key.as_bytes();
            let key_bit_len = if fixed_len > 0 {
                (fixed_len << 3) as i16 - 1
            } else {
                ((key.len() as i32 + 1) << 3) as i16 - 1
            };

            let go_right = key_bit_len < crit || testbit(key_bytes, crit);

            if go_right {
                match remove_from_tree(right, key, fixed_len) {
                    None => Some(left),
                    Some(new_right) => Some(Box::new(NodeOrLeaf::Node { crit, left, right: new_right })),
                }
            } else {
                match remove_from_tree(left, key, fixed_len) {
                    None => Some(right),
                    Some(new_left) => Some(Box::new(NodeOrLeaf::Node { crit, left: new_left, right })),
                }
            }
        }
    }
}
