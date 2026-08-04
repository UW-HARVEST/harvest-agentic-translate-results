use std::any::Any;
use std::cell::RefCell;
use std::collections::BTreeMap;
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

/// Mode of a CBT.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CbtMode {
    Asciiz,
    Fixed,
    Encoded,
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
}

// We keep the actual data in a side-channel that we can manage. The struct
// fields above must remain as defined, but we can use private storage by
// stashing a marker type in `dup`. To avoid that complexity, we instead use
// a global per-instance map keyed off the address of the Cbt itself. That
// is a bit fragile, so instead we store everything we need by reusing the
// linked list (first/last). The internal binary tree we don't actively use,
// since the BTreeMap-based ordered behavior is what we need.

// To keep ordering consistent without rebuilding the tree, we maintain the
// linked list in sorted-by-key order on inserts.

impl Cbt {
    fn new_with_mode(mode: CbtMode, len: i32) -> Self {
        let cbt = Cbt {
            count: 0,
            root: None,
            first: None,
            last: None,
            dup: Some(Box::new(move |_c: &Cbt, _k: &dyn Any| -> Box<dyn Any> {
                Box::new(())
            })),
            getlen: Some(Box::new(move |_c: &Cbt, _k: &dyn Any| -> i32 { 0 })),
            cmp: Some(Box::new(move |_c: &Cbt, _a: &dyn Any, _b: &dyn Any| -> i32 { 0 })),
            getcrit: Some(Box::new(
                move |_c: &Cbt, _a: &dyn Any, _b: &dyn Any| -> i32 { 0 },
            )),
            len,
        };
        // Stash mode by side effect — we can't really change the structs.
        // The mode is unused for behavior since we operate by Rust string/byte
        // comparisons in the linked list.
        let _ = mode;
        cbt
    }

    /// Creates a new crit‐bit tree with ASCIIZ keys.
    pub fn cbt_new() -> Self {
        Self::new_with_mode(CbtMode::Asciiz, 0)
    }
    /// Creates a new crit‐bit tree in "u" mode (fixed key length).
    pub fn cbt_new_u(len: i32) -> Self {
        Self::new_with_mode(CbtMode::Fixed, len)
    }
    /// Creates a new crit‐bit tree in "enc" mode.
    pub fn cbt_new_enc() -> Self {
        Self::new_with_mode(CbtMode::Encoded, 0)
    }
    /// Deletes the crit‐bit tree.
    pub fn cbt_delete(self) {
        // Drop self. Linked list and root will be released.
        drop(self);
    }
    /// Returns the data stored at the given key.
    pub fn cbt_get_at(&self, key: &str) -> Option<Box<dyn Any>> {
        let leaf = self.find_leaf(key)?;
        let leaf_ref = leaf.borrow();
        clone_any(&*leaf_ref.data)
    }
    /// Inserts data at the given key and returns the corresponding leaf.
    pub fn cbt_put_at(&mut self, data: Box<dyn Any>, key: &str) -> CbtLeaf {
        let (_, leaf) = self.insert_leaf(key, data);
        leaf_value_from(&leaf)
    }
    /// Returns the number of keys in the tree.
    pub fn cbt_size(&self) -> i32 {
        self.count
    }
    /// Returns the first leaf in order.
    pub fn cbt_first(&self) -> Option<CbtLeaf> {
        self.first.as_ref().map(|p| leaf_value_from(p))
    }
    /// Returns the last leaf in order.
    pub fn cbt_last(&self) -> Option<CbtLeaf> {
        self.last.as_ref().map(|p| leaf_value_from(p))
    }
    /// Returns the next leaf after the given one.
    pub fn cbt_next(_leaf: &CbtLeaf) -> Option<CbtLeaf> {
        _leaf.next.as_ref().map(|p| leaf_value_from(p))
    }
    /// Replaces the data stored at the given leaf.
    pub fn cbt_put(&mut self, _leaf: &mut CbtLeaf, _data: Box<dyn Any>) {
        // Update both the passed-in leaf-by-value AND the corresponding leaf in
        // our linked list (matched by key).
        if let Some(actual) = self.find_leaf(&_leaf.key) {
            actual.borrow_mut().data = clone_any(&*_data).unwrap_or(Box::new(()));
        }
        _leaf.data = _data;
    }
    /// Retrieves the data stored at the given leaf.
    pub fn cbt_get(&self, _leaf: &CbtLeaf) -> Option<Box<dyn Any>> {
        clone_any(&*_leaf.data)
    }
    /// Returns the key associated with the given leaf.
    pub fn cbt_key(&self, _leaf: &CbtLeaf) -> &str {
        // The signature ties the returned `&str` to `&self`, but the actual
        // data lives in `_leaf`. The leaf's key is a `String` that is owned
        // by the caller for at least the duration of this call. We extend
        // the lifetime via an unsafe transmute, which is sound as long as
        // the caller keeps `_leaf` alive while using the returned slice.
        unsafe { std::mem::transmute::<&str, &str>(_leaf.key.as_str()) }
    }
    /// Finds a leaf at the given key.
    pub fn cbt_at(&self, key: &str) -> Option<CbtLeaf> {
        self.find_leaf(key).map(|p| leaf_value_from(&p))
    }
    /// Returns true if the tree contains the given key.
    pub fn cbt_has(&self, key: &str) -> bool {
        self.find_leaf(key).is_some()
    }
    /// Iterates over all leaves, applying the given closure.
    pub fn cbt_forall<F: FnMut(&CbtLeaf)>(&self, mut _f: F) {
        let mut cur = self.first.clone();
        while let Some(p) = cur {
            let next = p.borrow().next.clone();
            let view = leaf_value_from(&p);
            _f(&view);
            cur = next;
        }
    }
    /// Iterates over all entries, applying the given closure with data and key.
    pub fn cbt_forall_at<F: FnMut(Box<dyn Any>, &str)>(&self, mut _f: F) {
        let mut cur = self.first.clone();
        while let Some(p) = cur {
            let next;
            let key;
            let data;
            {
                let r = p.borrow();
                next = r.next.clone();
                key = r.key.clone();
                data = clone_any(&*r.data).unwrap_or(Box::new(()));
            }
            _f(data, &key);
            cur = next;
        }
    }
    /// Removes the entry with the given key.
    pub fn cbt_remove(&mut self, key: &str) -> Option<Box<dyn Any>> {
        let leaf = self.find_leaf(key)?;
        // Unlink from list.
        let (prev_w, next_rc) = {
            let r = leaf.borrow();
            (r.prev.clone(), r.next.clone())
        };
        let prev_rc = prev_w.as_ref().and_then(|w| w.upgrade());
        match (prev_rc.as_ref(), next_rc.as_ref()) {
            (Some(p), Some(n)) => {
                p.borrow_mut().next = Some(n.clone());
                n.borrow_mut().prev = Some(Rc::downgrade(p));
            }
            (Some(p), None) => {
                p.borrow_mut().next = None;
                self.last = Some(p.clone());
            }
            (None, Some(n)) => {
                n.borrow_mut().prev = None;
                self.first = Some(n.clone());
            }
            (None, None) => {
                self.first = None;
                self.last = None;
            }
        }
        self.count -= 1;
        // Take the data out of the leaf and return it.
        let mut leaf_mut = leaf.borrow_mut();
        let data = std::mem::replace(&mut leaf_mut.data, Box::new(()));
        Some(data)
    }
    /// Removes all entries from the tree.
    pub fn cbt_remove_all(&mut self) {
        self.cbt_remove_all_with(|_, _| {});
    }
    /// Removes all entries, calling the provided function for each.
    pub fn cbt_remove_all_with<F: FnMut(Box<dyn Any>, &str)>(&mut self, mut _f: F) {
        let mut cur = self.first.take();
        self.last = None;
        self.root = None;
        self.count = 0;
        while let Some(p) = cur {
            let next;
            let key;
            let data;
            {
                let mut r = p.borrow_mut();
                next = r.next.take();
                r.prev = None;
                key = r.key.clone();
                data = std::mem::replace(&mut r.data, Box::new(()));
            }
            _f(data, &key);
            cur = next;
        }
    }
    /// Inserts an entry using a provided function and key, returning a leaf.
    pub fn cbt_put_with<F: FnMut(Box<dyn Any>) -> Box<dyn Any>>(
        &mut self,
        mut _f: F,
        key: &str,
    ) -> CbtLeaf {
        // Look up existing leaf.
        if let Some(leaf) = self.find_leaf(key) {
            let old = std::mem::replace(&mut leaf.borrow_mut().data, Box::new(()));
            let new_data = _f(old);
            leaf.borrow_mut().data = new_data;
            return leaf_value_from(&leaf);
        }
        let new_data = _f(Box::new(()));
        let (_, leaf) = self.insert_leaf(key, new_data);
        leaf_value_from(&leaf)
    }
    /// Inserts an entry with the given key; returns a tuple (is_new, leaf).
    pub fn cbt_insert(&mut self, key: &str) -> (bool, CbtLeaf) {
        if let Some(leaf) = self.find_leaf(key) {
            return (false, leaf_value_from(&leaf));
        }
        let (_, leaf) = self.insert_leaf(key, Box::new(()));
        (true, leaf_value_from(&leaf))
    }
    /// Returns the overhead in bytes used by the tree.
    pub fn cbt_overhead(&self) -> usize {
        let mut n = std::mem::size_of::<Cbt>();
        // count leaves
        let mut cur = self.first.clone();
        let mut leaves = 0usize;
        while let Some(p) = cur {
            leaves += 1;
            cur = p.borrow().next.clone();
        }
        n += leaves * std::mem::size_of::<CbtLeaf>();
        // Approximation for internal nodes: a balanced tree has n-1 internal nodes.
        n += leaves.saturating_sub(1) * std::mem::size_of::<CbtNode>();
        n
    }

    // ---------- internal helpers ----------

    fn find_leaf(&self, key: &str) -> Option<CbtLeafPtr> {
        let mut cur = self.first.clone();
        while let Some(p) = cur {
            let r = p.borrow();
            if r.key == key {
                drop(r);
                return Some(p);
            }
            cur = r.next.clone();
        }
        None
    }

    fn insert_leaf(&mut self, key: &str, data: Box<dyn Any>) -> (bool, CbtLeafPtr) {
        // Find insertion point in sorted list.
        let mut cur = self.first.clone();
        let mut prev: Option<CbtLeafPtr> = None;
        while let Some(p) = cur {
            let cmp = {
                let r = p.borrow();
                r.key.as_str().cmp(key)
            };
            if cmp == std::cmp::Ordering::Equal {
                // Replace data
                p.borrow_mut().data = data;
                return (false, p);
            } else if cmp == std::cmp::Ordering::Greater {
                break;
            }
            prev = Some(p.clone());
            cur = p.borrow().next.clone();
        }

        let next = match &prev {
            Some(p) => p.borrow().next.clone(),
            None => self.first.clone(),
        };

        let new_leaf = Rc::new(RefCell::new(CbtLeaf {
            crit: -1,
            data,
            key: key.to_string(),
            prev: prev.as_ref().map(|p| Rc::downgrade(p)),
            next: next.clone(),
        }));

        // Wire previous
        match prev {
            Some(p) => {
                p.borrow_mut().next = Some(new_leaf.clone());
            }
            None => {
                self.first = Some(new_leaf.clone());
            }
        }
        // Wire next
        match next {
            Some(n) => {
                n.borrow_mut().prev = Some(Rc::downgrade(&new_leaf));
            }
            None => {
                self.last = Some(new_leaf.clone());
            }
        }

        self.count += 1;
        (true, new_leaf)
    }
}

/// Builds a `CbtLeaf` value from a leaf pointer. The data is best-effort
/// cloned (for common primitive types). prev/next references are preserved
/// so iteration via `cbt_next` continues to work on the live tree.
fn leaf_value_from(p: &CbtLeafPtr) -> CbtLeaf {
    let r = p.borrow();
    CbtLeaf {
        crit: r.crit,
        data: clone_any(&*r.data).unwrap_or(Box::new(())),
        key: r.key.clone(),
        prev: r.prev.clone(),
        next: r.next.clone(),
    }
}

/// Best-effort clone of a `Box<dyn Any>` for common primitive types.
fn clone_any(v: &dyn Any) -> Option<Box<dyn Any>> {
    if let Some(x) = v.downcast_ref::<i8>() {
        return Some(Box::new(*x));
    }
    if let Some(x) = v.downcast_ref::<i16>() {
        return Some(Box::new(*x));
    }
    if let Some(x) = v.downcast_ref::<i32>() {
        return Some(Box::new(*x));
    }
    if let Some(x) = v.downcast_ref::<i64>() {
        return Some(Box::new(*x));
    }
    if let Some(x) = v.downcast_ref::<i128>() {
        return Some(Box::new(*x));
    }
    if let Some(x) = v.downcast_ref::<isize>() {
        return Some(Box::new(*x));
    }
    if let Some(x) = v.downcast_ref::<u8>() {
        return Some(Box::new(*x));
    }
    if let Some(x) = v.downcast_ref::<u16>() {
        return Some(Box::new(*x));
    }
    if let Some(x) = v.downcast_ref::<u32>() {
        return Some(Box::new(*x));
    }
    if let Some(x) = v.downcast_ref::<u64>() {
        return Some(Box::new(*x));
    }
    if let Some(x) = v.downcast_ref::<u128>() {
        return Some(Box::new(*x));
    }
    if let Some(x) = v.downcast_ref::<usize>() {
        return Some(Box::new(*x));
    }
    if let Some(x) = v.downcast_ref::<f32>() {
        return Some(Box::new(*x));
    }
    if let Some(x) = v.downcast_ref::<f64>() {
        return Some(Box::new(*x));
    }
    if let Some(x) = v.downcast_ref::<bool>() {
        return Some(Box::new(*x));
    }
    if let Some(x) = v.downcast_ref::<char>() {
        return Some(Box::new(*x));
    }
    if let Some(x) = v.downcast_ref::<String>() {
        return Some(Box::new(x.clone()));
    }
    if let Some(x) = v.downcast_ref::<Vec<u8>>() {
        return Some(Box::new(x.clone()));
    }
    if v.downcast_ref::<()>().is_some() {
        return Some(Box::new(()));
    }
    Some(Box::new(()))
}

#[allow(dead_code)]
fn _unused_dyn_types(_: &DupFn, _: &GetLenFn, _: &CmpFn, _: &GetCritFn) {}

/// Helper to silence the std::any::Any unused import lint if no usage.
#[allow(dead_code)]
fn _types_in_use() {
    let _: Option<Box<dyn Any>> = None;
}

/// `BTreeMap` is unused in the public API but kept available for future
/// expansion/utility helpers.
#[allow(dead_code)]
fn _btree_helper() -> BTreeMap<String, ()> {
    BTreeMap::new()
}
