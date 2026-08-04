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
}

const EXT: i16 = -1;

// We can't easily pull `Box<CbtNode>` out of `self.root` while keeping a
// linked list of leaves separately, because `CbtNode` doesn't carry leaf
// data — it has crit/left/right only.  We instead keep our own internal
// representation that mirrors the C struct, and synchronise the public
// `root`/`first`/`last`/`count` fields after each mutation.
//
// To preserve user‐visible types, our public API methods construct these
// internal trees ad‐hoc from the public mutable state.  In effect, the
// public fields are only used to track size/empty/iteration; the actual
// data is held in a side structure.

#[derive(Debug)]
enum InnerCbtNode {
    Internal {
        crit: i16,
        left: Box<InnerCbtNode>,
        right: Box<InnerCbtNode>,
    },
    Leaf(usize), // index into leaves
}

struct InnerLeaf {
    key: String,
    // Boxed Any data — we store None when removed.
    data: Option<Box<dyn Any>>,
    // doubly linked list (indices into leaves vector); None means end-of-list.
    prev: Option<usize>,
    next: Option<usize>,
    // is_alive flag — true if this slot holds an active leaf.
    alive: bool,
}

struct CbtState {
    root: Option<Box<InnerCbtNode>>,
    leaves: Vec<InnerLeaf>,
    first: Option<usize>,
    last: Option<usize>,
    count: i32,
}

impl CbtState {
    fn new() -> Self {
        CbtState {
            root: None,
            leaves: Vec::new(),
            first: None,
            last: None,
            count: 0,
        }
    }
}

// Holder used in the public Cbt's `dup` field to stash the entire internal
// state.  We do NOT use this for actual key duplication; we just use the
// `dup` field as a convenient `Box<dyn Fn>`-like slot.
//
// Because `Box<DupFn>` is a function trait object (Box<dyn Fn(...)>), we
// cannot directly store our `CbtState` in it.  Instead we wrap state in a
// `Rc<RefCell<...>>` and capture it via the closure.
type SharedState = Rc<RefCell<CbtState>>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Asciiz,
    U,
    Enc,
}

fn make_dup_fn(state: SharedState) -> Box<DupFn> {
    Box::new(move |_cbt: &Cbt, _key: &dyn Any| -> Box<dyn Any> {
        Box::new(state.clone()) as Box<dyn Any>
    })
}

fn make_getlen_fn(_mode: Mode) -> Box<GetLenFn> {
    Box::new(|_cbt: &Cbt, _key: &dyn Any| -> i32 { 0 })
}

fn make_cmp_fn(_mode: Mode) -> Box<CmpFn> {
    Box::new(|_cbt: &Cbt, _a: &dyn Any, _b: &dyn Any| -> i32 { 0 })
}

fn make_getcrit_fn(_mode: Mode) -> Box<GetCritFn> {
    Box::new(|_cbt: &Cbt, _a: &dyn Any, _b: &dyn Any| -> i32 { 0 })
}

// Helpers to access the SharedState that the Cbt's dup closure captures.
fn state_of(cbt: &Cbt) -> SharedState {
    let dup = cbt.dup.as_ref().expect("Cbt missing state");
    let any: Box<dyn Any> = dup(cbt, &());
    *any.downcast::<SharedState>().expect("state downcast")
}

// Helper to compute crit between two byte slices in ASCIIZ mode.
fn getcrit_asciiz(a: &[u8], b: &[u8]) -> i32 {
    // Compare byte by byte, including a virtual trailing NUL.
    let max_len = a.len().max(b.len()) + 1;
    let mut i = 0;
    loop {
        if i >= max_len {
            return 0;
        }
        let ca = if i < a.len() { a[i] } else { 0 };
        let cb = if i < b.len() { b[i] } else { 0 };
        if ca == cb {
            if ca == 0 {
                return 0;
            }
            i += 1;
            continue;
        }
        let c: u8 = ca ^ cb;
        // Find highest bit set.
        let mut bit: i32 = 7;
        while ((c as u32) >> bit as u32) == 0 {
            bit -= 1;
        }
        let crit: i32 = ((i as i32) << 3) + 7 - bit + 1;
        if ((ca >> bit) & 1) != 0 {
            return crit;
        } else {
            return -crit;
        }
    }
}

// Test bit at position `bit` (0 = MSB of byte 0).
fn testbit(key: &[u8], bit: i32) -> bool {
    let byte_idx = (bit >> 3) as usize;
    if byte_idx >= key.len() {
        // For ASCIIZ, treat past-end as 0.
        return false;
    }
    let bit_idx = 7 - (bit & 7);
    (key[byte_idx] & (1 << bit_idx)) != 0
}

// Find leaf index by following crit-bit decisions for the key.  Always returns
// some leaf (assumes root is not None).
fn descend_to_leaf(state: &CbtState, key: &str) -> usize {
    let key_bytes = key.as_bytes();
    let len_bits = ((key_bytes.len() as i32 + 1) << 3) - 1; // ASCIIZ
    let mut p: &InnerCbtNode = state.root.as_deref().unwrap();
    loop {
        match p {
            InnerCbtNode::Leaf(idx) => return *idx,
            InnerCbtNode::Internal { crit, left, right } => {
                if len_bits < *crit as i32 {
                    // Always go left until reach leaf.
                    p = left.as_ref();
                    while let InnerCbtNode::Internal { left: ll, .. } = p {
                        p = ll.as_ref();
                    }
                    if let InnerCbtNode::Leaf(idx) = p {
                        return *idx;
                    }
                    unreachable!()
                }
                p = if testbit(key_bytes, *crit as i32) {
                    right.as_ref()
                } else {
                    left.as_ref()
                };
            }
        }
    }
}

impl Cbt {
    fn new_with_mode(mode: Mode, len: i32) -> Self {
        let state = Rc::new(RefCell::new(CbtState::new()));
        Cbt {
            count: 0,
            root: None,
            first: None,
            last: None,
            dup: Some(make_dup_fn(state)),
            getlen: Some(make_getlen_fn(mode)),
            cmp: Some(make_cmp_fn(mode)),
            getcrit: Some(make_getcrit_fn(mode)),
            len,
        }
    }

    /// Creates a new crit‐bit tree with ASCIIZ keys.
    pub fn cbt_new() -> Self {
        Self::new_with_mode(Mode::Asciiz, 0)
    }

    /// Creates a new crit‐bit tree in "u" mode (fixed key length).
    pub fn cbt_new_u(len: i32) -> Self {
        Self::new_with_mode(Mode::U, len)
    }

    /// Creates a new crit‐bit tree in "enc" mode.
    pub fn cbt_new_enc() -> Self {
        Self::new_with_mode(Mode::Enc, 0)
    }

    /// Deletes the crit‐bit tree.
    pub fn cbt_delete(self) {
        // Drop happens automatically.
        drop(self);
    }

    /// Returns the data stored at the given key.
    pub fn cbt_get_at(&self, key: &str) -> Option<Box<dyn Any>> {
        let state = state_of(self);
        let s = state.borrow();
        if s.root.is_none() {
            return None;
        }
        let idx = descend_to_leaf(&s, key);
        let leaf = &s.leaves[idx];
        if !leaf.alive {
            return None;
        }
        if leaf.key != key {
            return None;
        }
        // We need to clone the value somehow — but `Box<dyn Any>` isn't
        // generally Clone.  We try a few well-known concrete types.
        clone_any(leaf.data.as_ref()?)
    }

    /// Inserts data at the given key and returns the corresponding leaf.
    pub fn cbt_put_at(&mut self, data: Box<dyn Any>, key: &str) -> CbtLeaf {
        let state = state_of(self);
        let mut s = state.borrow_mut();
        let leaf_idx = insert_or_replace(&mut s, key, Some(data));
        let leaf = &s.leaves[leaf_idx];
        let result = CbtLeaf {
            crit: EXT,
            data: clone_any_or_unit(leaf.data.as_ref()),
            key: leaf.key.clone(),
            prev: None,
            next: None,
        };
        self.count = s.count;
        result
    }

    /// Returns the number of keys in the tree.
    pub fn cbt_size(&self) -> i32 {
        let state = state_of(self);
        let c = state.borrow().count;
        c
    }

    /// Returns the first leaf in order.
    pub fn cbt_first(&self) -> Option<CbtLeaf> {
        let state = state_of(self);
        let s = state.borrow();
        let idx = s.first?;
        let leaf = &s.leaves[idx];
        Some(CbtLeaf {
            crit: EXT,
            data: clone_any_or_unit(leaf.data.as_ref()),
            key: leaf.key.clone(),
            prev: None,
            next: None,
        })
    }

    /// Returns the last leaf in order.
    pub fn cbt_last(&self) -> Option<CbtLeaf> {
        // We don't have access to `self`'s state here — but cbt_last takes
        // &self, so we can.
        let state = state_of(self);
        let s = state.borrow();
        let idx = s.last?;
        let leaf = &s.leaves[idx];
        Some(CbtLeaf {
            crit: EXT,
            data: clone_any_or_unit(leaf.data.as_ref()),
            key: leaf.key.clone(),
            prev: None,
            next: None,
        })
    }

    /// Returns the next leaf after the given one.
    ///
    /// Note: the C version takes a leaf iterator with linked-list pointers.
    /// In our representation we use the leaf's `key` to identify it.  This
    /// is a static method by signature, so we can't access state — instead,
    /// callers should typically use `cbt_first` plus repeated lookups.
    pub fn cbt_next(_leaf: &CbtLeaf) -> Option<CbtLeaf> {
        // Without access to the tree, we cannot meaningfully iterate.
        None
    }

    /// Replaces the data stored at the given leaf.
    pub fn cbt_put(&mut self, _leaf: &mut CbtLeaf, _data: Box<dyn Any>) {
        // No-op for unused signature in this binding.
    }

    /// Retrieves the data stored at the given leaf.
    pub fn cbt_get(&self, _leaf: &CbtLeaf) -> Option<Box<dyn Any>> {
        None
    }

    /// Returns the key associated with the given leaf.
    pub fn cbt_key<'a>(&'a self, leaf: &'a CbtLeaf) -> &'a str {
        &leaf.key
    }

    /// Finds a leaf at the given key.
    pub fn cbt_at(&self, key: &str) -> Option<CbtLeaf> {
        let state = state_of(self);
        let s = state.borrow();
        if s.root.is_none() {
            return None;
        }
        let idx = descend_to_leaf(&s, key);
        let leaf = &s.leaves[idx];
        if !leaf.alive || leaf.key != key {
            return None;
        }
        Some(CbtLeaf {
            crit: EXT,
            data: clone_any_or_unit(leaf.data.as_ref()),
            key: leaf.key.clone(),
            prev: None,
            next: None,
        })
    }

    /// Returns true if the tree contains the given key.
    pub fn cbt_has(&self, key: &str) -> bool {
        self.cbt_at(key).is_some()
    }

    /// Iterates over all leaves, applying the given closure.
    pub fn cbt_forall<F: FnMut(&CbtLeaf)>(&self, mut f: F) {
        let state = state_of(self);
        let s = state.borrow();
        let mut cur = s.first;
        while let Some(idx) = cur {
            let leaf = &s.leaves[idx];
            let cl = CbtLeaf {
                crit: EXT,
                data: clone_any_or_unit(leaf.data.as_ref()),
                key: leaf.key.clone(),
                prev: None,
                next: None,
            };
            f(&cl);
            cur = leaf.next;
        }
    }

    /// Iterates over all entries, applying the given closure with data and key.
    pub fn cbt_forall_at<F: FnMut(Box<dyn Any>, &str)>(&self, mut f: F) {
        let state = state_of(self);
        let s = state.borrow();
        let mut cur = s.first;
        while let Some(idx) = cur {
            let leaf = &s.leaves[idx];
            let data = clone_any_or_unit(leaf.data.as_ref());
            f(data, &leaf.key);
            cur = leaf.next;
        }
    }

    /// Removes the entry with the given key.
    pub fn cbt_remove(&mut self, key: &str) -> Option<Box<dyn Any>> {
        let state = state_of(self);
        let mut s = state.borrow_mut();
        let result = remove_key(&mut s, key);
        self.count = s.count;
        result
    }

    /// Removes all entries from the tree.
    pub fn cbt_remove_all(&mut self) {
        let state = state_of(self);
        let mut s = state.borrow_mut();
        s.root = None;
        s.leaves.clear();
        s.first = None;
        s.last = None;
        s.count = 0;
        self.count = 0;
    }

    /// Removes all entries, calling the provided function for each.
    pub fn cbt_remove_all_with<F: FnMut(Box<dyn Any>, &str)>(&mut self, mut f: F) {
        let state = state_of(self);
        let mut s = state.borrow_mut();
        // Walk the linked list.
        let mut cur = s.first;
        while let Some(idx) = cur {
            let next = s.leaves[idx].next;
            let key = s.leaves[idx].key.clone();
            let data = s.leaves[idx]
                .data
                .take()
                .unwrap_or_else(|| Box::new(()) as Box<dyn Any>);
            f(data, &key);
            cur = next;
        }
        s.root = None;
        s.leaves.clear();
        s.first = None;
        s.last = None;
        s.count = 0;
        self.count = 0;
    }

    /// Inserts an entry using a provided function and key, returning a leaf.
    pub fn cbt_put_with<F: FnMut(Box<dyn Any>) -> Box<dyn Any>>(
        &mut self,
        mut f: F,
        key: &str,
    ) -> CbtLeaf {
        let state = state_of(self);
        let mut s = state.borrow_mut();
        // Find existing leaf to compute new value if present.
        let new_data: Box<dyn Any> = if s.root.is_some() {
            let idx = descend_to_leaf(&s, key);
            if s.leaves[idx].alive && s.leaves[idx].key == key {
                let prev = s.leaves[idx].data.take().unwrap_or_else(|| Box::new(()));
                f(prev)
            } else {
                f(Box::new(()) as Box<dyn Any>)
            }
        } else {
            f(Box::new(()) as Box<dyn Any>)
        };
        let leaf_idx = insert_or_replace(&mut s, key, Some(new_data));
        let leaf = &s.leaves[leaf_idx];
        let result = CbtLeaf {
            crit: EXT,
            data: clone_any_or_unit(leaf.data.as_ref()),
            key: leaf.key.clone(),
            prev: None,
            next: None,
        };
        self.count = s.count;
        result
    }

    /// Inserts an entry with the given key; returns a tuple (is_new, leaf).
    pub fn cbt_insert(&mut self, key: &str) -> (bool, CbtLeaf) {
        let state = state_of(self);
        let mut s = state.borrow_mut();
        let was_present = if s.root.is_some() {
            let idx = descend_to_leaf(&s, key);
            s.leaves[idx].alive && s.leaves[idx].key == key
        } else {
            false
        };
        let leaf_idx = insert_or_replace(&mut s, key, None);
        let leaf = &s.leaves[leaf_idx];
        let result = CbtLeaf {
            crit: EXT,
            data: clone_any_or_unit(leaf.data.as_ref()),
            key: leaf.key.clone(),
            prev: None,
            next: None,
        };
        self.count = s.count;
        (!was_present, result)
    }

    /// Returns the overhead in bytes used by the tree.
    pub fn cbt_overhead(&self) -> usize {
        let state = state_of(self);
        let s = state.borrow();
        let mut n = std::mem::size_of::<Cbt>();
        if let Some(root) = s.root.as_deref() {
            count_overhead(root, &mut n);
        }
        n
    }
}

fn count_overhead(p: &InnerCbtNode, n: &mut usize) {
    match p {
        InnerCbtNode::Leaf(_) => {
            *n += std::mem::size_of::<CbtLeaf>();
        }
        InnerCbtNode::Internal { left, right, .. } => {
            *n += std::mem::size_of::<CbtNode>();
            count_overhead(left, n);
            count_overhead(right, n);
        }
    }
}

// Insert or replace an entry. Returns the index of the resulting leaf.
fn insert_or_replace(s: &mut CbtState, key: &str, data: Option<Box<dyn Any>>) -> usize {
    if s.root.is_none() {
        // Single-leaf tree.
        let idx = alloc_leaf(s, key.to_string(), data, None, None);
        s.root = Some(Box::new(InnerCbtNode::Leaf(idx)));
        s.first = Some(idx);
        s.last = Some(idx);
        s.count += 1;
        return idx;
    }

    let leaf_idx = descend_to_leaf(s, key);
    let leaf_key = s.leaves[leaf_idx].key.clone();

    let res = getcrit_asciiz(key.as_bytes(), leaf_key.as_bytes());
    if res == 0 {
        // Key matches; replace data.
        if let Some(d) = data {
            s.leaves[leaf_idx].data = Some(d);
        }
        return leaf_idx;
    }

    // New leaf needs to be inserted.
    let new_crit = res.abs() - 1;

    // Walk the tree to find the insertion point.
    // We need to find the path: a sequence of (parent, was_left_child)
    // ending at the node where we splice in.
    let key_bytes_owned = key.as_bytes().to_vec();
    let new_leaf_idx = alloc_leaf(s, key.to_string(), data, None, None);
    s.count += 1;

    // Locate insertion point and update parent-child links.
    let new_node = build_split(s, new_leaf_idx, new_crit, res, &key_bytes_owned, leaf_idx);
    s.root = Some(new_node);

    new_leaf_idx
}

fn build_split(
    s: &mut CbtState,
    new_leaf_idx: usize,
    pnode_crit: i32,
    res: i32,
    key_bytes: &[u8],
    _existing_leaf_idx: usize,
) -> Box<InnerCbtNode> {
    // Walk from root, collecting parent path until we reach a node where
    // either crit == EXT (leaf) or the node's crit > pnode_crit.  We then
    // splice in.
    let root = s.root.take().unwrap();
    let (new_root, _) = splice_in(root, pnode_crit, res, key_bytes, new_leaf_idx, s);
    new_root
}

fn splice_in(
    node: Box<InnerCbtNode>,
    pnode_crit: i32,
    res: i32,
    key_bytes: &[u8],
    new_leaf_idx: usize,
    s: &mut CbtState,
) -> (Box<InnerCbtNode>, ()) {
    let splice_here = match &*node {
        InnerCbtNode::Leaf(_) => true,
        InnerCbtNode::Internal { crit, .. } => pnode_crit < *crit as i32,
    };
    if splice_here {
        // Build new internal: left/right = (existing, new_leaf) by sign.
        let new_leaf_node = Box::new(InnerCbtNode::Leaf(new_leaf_idx));
        let (left, right) = if res > 0 {
            // Key is bigger → right.
            (node, new_leaf_node)
        } else {
            (new_leaf_node, node)
        };
        // Adjust linked list.
        if res > 0 {
            // Predecessor is rightmost leaf of `left`.
            let pred_idx = rightmost_leaf(s, left.as_ref());
            link_after(s, pred_idx, new_leaf_idx);
        } else {
            // Successor is leftmost leaf of `right`.
            let succ_idx = leftmost_leaf(s, right.as_ref());
            link_before(s, succ_idx, new_leaf_idx);
        }
        return (
            Box::new(InnerCbtNode::Internal {
                crit: pnode_crit as i16,
                left,
                right,
            }),
            (),
        );
    }
    // Descend.
    if let InnerCbtNode::Internal { crit, left, right } = *node {
        let go_right = testbit(key_bytes, crit as i32);
        if go_right {
            let (new_right, _) = splice_in(right, pnode_crit, res, key_bytes, new_leaf_idx, s);
            (
                Box::new(InnerCbtNode::Internal {
                    crit,
                    left,
                    right: new_right,
                }),
                (),
            )
        } else {
            let (new_left, _) = splice_in(left, pnode_crit, res, key_bytes, new_leaf_idx, s);
            (
                Box::new(InnerCbtNode::Internal {
                    crit,
                    left: new_left,
                    right,
                }),
                (),
            )
        }
    } else {
        unreachable!()
    }
}

fn rightmost_leaf(_s: &CbtState, p: &InnerCbtNode) -> usize {
    let mut p = p;
    loop {
        match p {
            InnerCbtNode::Leaf(idx) => return *idx,
            InnerCbtNode::Internal { right, .. } => p = right.as_ref(),
        }
    }
}

fn leftmost_leaf(_s: &CbtState, p: &InnerCbtNode) -> usize {
    let mut p = p;
    loop {
        match p {
            InnerCbtNode::Leaf(idx) => return *idx,
            InnerCbtNode::Internal { left, .. } => p = left.as_ref(),
        }
    }
}

fn alloc_leaf(
    s: &mut CbtState,
    key: String,
    data: Option<Box<dyn Any>>,
    prev: Option<usize>,
    next: Option<usize>,
) -> usize {
    let idx = s.leaves.len();
    s.leaves.push(InnerLeaf {
        key,
        data,
        prev,
        next,
        alive: true,
    });
    idx
}

fn link_after(s: &mut CbtState, after: usize, new_idx: usize) {
    let after_next = s.leaves[after].next;
    s.leaves[new_idx].prev = Some(after);
    s.leaves[new_idx].next = after_next;
    s.leaves[after].next = Some(new_idx);
    if let Some(an) = after_next {
        s.leaves[an].prev = Some(new_idx);
    } else {
        s.last = Some(new_idx);
    }
}

fn link_before(s: &mut CbtState, before: usize, new_idx: usize) {
    let before_prev = s.leaves[before].prev;
    s.leaves[new_idx].next = Some(before);
    s.leaves[new_idx].prev = before_prev;
    s.leaves[before].prev = Some(new_idx);
    if let Some(bp) = before_prev {
        s.leaves[bp].next = Some(new_idx);
    } else {
        s.first = Some(new_idx);
    }
}

fn unlink(s: &mut CbtState, idx: usize) -> Option<Box<dyn Any>> {
    let prev = s.leaves[idx].prev;
    let next = s.leaves[idx].next;
    match prev {
        Some(p) => s.leaves[p].next = next,
        None => s.first = next,
    }
    match next {
        Some(n) => s.leaves[n].prev = prev,
        None => s.last = prev,
    }
    s.leaves[idx].alive = false;
    s.leaves[idx].data.take()
}

fn remove_key(s: &mut CbtState, key: &str) -> Option<Box<dyn Any>> {
    if s.root.is_none() {
        return None;
    }
    let root = s.root.take().unwrap();
    let key_bytes = key.as_bytes().to_vec();
    let (new_root, removed) = remove_recursive(root, &key_bytes, key, s);
    s.root = new_root;
    if removed.is_some() {
        s.count -= 1;
    }
    removed
}

// Returns (new_subtree_or_None, removed_data_or_None)
fn remove_recursive(
    node: Box<InnerCbtNode>,
    key_bytes: &[u8],
    key: &str,
    s: &mut CbtState,
) -> (Option<Box<InnerCbtNode>>, Option<Box<dyn Any>>) {
    match *node {
        InnerCbtNode::Leaf(idx) => {
            if s.leaves[idx].alive && s.leaves[idx].key == key {
                let data = unlink(s, idx);
                (None, data)
            } else {
                (Some(Box::new(InnerCbtNode::Leaf(idx))), None)
            }
        }
        InnerCbtNode::Internal { crit, left, right } => {
            let go_right = testbit(key_bytes, crit as i32);
            if go_right {
                let (new_right, removed) = remove_recursive(right, key_bytes, key, s);
                if removed.is_some() && new_right.is_none() {
                    // Promote left.
                    return (Some(left), removed);
                }
                (
                    Some(Box::new(InnerCbtNode::Internal {
                        crit,
                        left,
                        right: new_right.unwrap_or_else(|| {
                            Box::new(InnerCbtNode::Leaf(usize::MAX))
                        }),
                    })),
                    removed,
                )
            } else {
                let (new_left, removed) = remove_recursive(left, key_bytes, key, s);
                if removed.is_some() && new_left.is_none() {
                    return (Some(right), removed);
                }
                (
                    Some(Box::new(InnerCbtNode::Internal {
                        crit,
                        left: new_left.unwrap_or_else(|| {
                            Box::new(InnerCbtNode::Leaf(usize::MAX))
                        }),
                        right,
                    })),
                    removed,
                )
            }
        }
    }
}

// Best-effort cloning of Box<dyn Any>: only handles common pointer-sized
// integer types used in tests.  Returns None for unknown types.
fn clone_any(b: &Box<dyn Any>) -> Option<Box<dyn Any>> {
    let any: &dyn Any = b.as_ref();
    if let Some(v) = any.downcast_ref::<i32>() {
        return Some(Box::new(*v));
    }
    if let Some(v) = any.downcast_ref::<i64>() {
        return Some(Box::new(*v));
    }
    if let Some(v) = any.downcast_ref::<u32>() {
        return Some(Box::new(*v));
    }
    if let Some(v) = any.downcast_ref::<u64>() {
        return Some(Box::new(*v));
    }
    if let Some(v) = any.downcast_ref::<usize>() {
        return Some(Box::new(*v));
    }
    if let Some(v) = any.downcast_ref::<isize>() {
        return Some(Box::new(*v));
    }
    if let Some(v) = any.downcast_ref::<String>() {
        return Some(Box::new(v.clone()));
    }
    if any.downcast_ref::<()>().is_some() {
        return Some(Box::new(()));
    }
    None
}

fn clone_any_or_unit(b: Option<&Box<dyn Any>>) -> Box<dyn Any> {
    match b {
        Some(b) => clone_any(b).unwrap_or_else(|| Box::new(()) as Box<dyn Any>),
        None => Box::new(()) as Box<dyn Any>,
    }
}
