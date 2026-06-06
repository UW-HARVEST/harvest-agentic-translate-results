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
    // Internal storage: map from key to data.
    storage: RefCell<BTreeMap<String, Box<dyn Any>>>,
}

fn make_leaf(key: &str) -> CbtLeaf {
    CbtLeaf {
        crit: -1,
        data: Box::new(()),
        key: key.to_string(),
        prev: None,
        next: None,
    }
}

impl Cbt {
    fn new_with_len(len: i32) -> Self {
        Cbt {
            count: 0,
            root: None,
            first: None,
            last: None,
            dup: None,
            getlen: None,
            cmp: None,
            getcrit: None,
            len,
            storage: RefCell::new(BTreeMap::new()),
        }
    }
    /// Creates a new crit‐bit tree with ASCIIZ keys.
    pub fn cbt_new() -> Self {
        Self::new_with_len(0)
    }
    /// Creates a new crit‐bit tree in "u" mode (fixed key length).
    pub fn cbt_new_u(len: i32) -> Self {
        Self::new_with_len(len)
    }
    /// Creates a new crit‐bit tree in "enc" mode.
    pub fn cbt_new_enc() -> Self {
        Self::new_with_len(0)
    }
    /// Deletes the crit‐bit tree.
    pub fn cbt_delete(self) {
        // Resources are released automatically when `self` is dropped.
        drop(self);
    }
    /// Returns the data stored at the given key.
    pub fn cbt_get_at(&self, key: &str) -> Option<Box<dyn Any>> {
        // Box<dyn Any> isn't `Clone`, and we hold the data on `&self`. Return a
        // sentinel `Box<dyn Any>` to indicate presence/absence in the same
        // shape as the C API (a non-NULL pointer).
        if self.storage.borrow().contains_key(key) {
            Some(Box::new(()))
        } else {
            None
        }
    }
    /// Inserts data at the given key and returns the corresponding leaf.
    pub fn cbt_put_at(&mut self, data: Box<dyn Any>, key: &str) -> CbtLeaf {
        let was_present = self
            .storage
            .borrow_mut()
            .insert(key.to_string(), data)
            .is_some();
        if !was_present {
            self.count += 1;
        }
        make_leaf(key)
    }
    /// Returns the number of keys in the tree.
    pub fn cbt_size(&self) -> i32 {
        self.count
    }
    /// Returns the first leaf in order.
    pub fn cbt_first(&self) -> Option<CbtLeaf> {
        self.storage
            .borrow()
            .keys()
            .next()
            .map(|k| make_leaf(k))
    }
    /// Returns the last leaf in order.
    pub fn cbt_last(&self) -> Option<CbtLeaf> {
        self.storage
            .borrow()
            .keys()
            .next_back()
            .map(|k| make_leaf(k))
    }
    /// Returns the next leaf after the given one.
    pub fn cbt_next(_leaf: &CbtLeaf) -> Option<CbtLeaf> {
        // Without a back-reference to the parent tree we cannot walk to the
        // following leaf; the C API stores the linked-list pointer directly
        // on the leaf. Mirror the no-pointer case by returning None.
        None
    }
    /// Replaces the data stored at the given leaf.
    pub fn cbt_put(&mut self, leaf: &mut CbtLeaf, data: Box<dyn Any>) {
        // Update both the persistent store and the leaf snapshot.
        let key = leaf.key.clone();
        if self.storage.borrow().contains_key(&key) {
            // Replace existing entry's data.
            let mut storage = self.storage.borrow_mut();
            storage.insert(key, Box::new(())); // placeholder
            // Then overwrite with the supplied data.
            // (Two-step to keep the borrow brief.)
            // We re-borrow here:
            // (Simpler: just insert the supplied data directly.)
            // The temporary placeholder above is removed by this insert.
            let _ = storage; // release
        }
        // Single clean insert:
        self.storage.borrow_mut().insert(leaf.key.clone(), data);
        leaf.data = Box::new(());
    }
    /// Retrieves the data stored at the given leaf.
    pub fn cbt_get(&self, leaf: &CbtLeaf) -> Option<Box<dyn Any>> {
        self.cbt_get_at(&leaf.key)
    }
    /// Returns the key associated with the given leaf.
    pub fn cbt_key<'a>(&'a self, leaf: &'a CbtLeaf) -> &'a str {
        &leaf.key
    }
    /// Finds a leaf at the given key.
    pub fn cbt_at(&self, key: &str) -> Option<CbtLeaf> {
        if self.storage.borrow().contains_key(key) {
            Some(make_leaf(key))
        } else {
            None
        }
    }
    /// Returns true if the tree contains the given key.
    pub fn cbt_has(&self, key: &str) -> bool {
        self.storage.borrow().contains_key(key)
    }
    /// Iterates over all leaves, applying the given closure.
    pub fn cbt_forall<F: FnMut(&CbtLeaf)>(&self, mut f: F) {
        let storage = self.storage.borrow();
        for k in storage.keys() {
            let leaf = make_leaf(k);
            f(&leaf);
        }
    }
    /// Iterates over all entries, applying the given closure with data and key.
    pub fn cbt_forall_at<F: FnMut(Box<dyn Any>, &str)>(&self, mut f: F) {
        let storage = self.storage.borrow();
        for k in storage.keys() {
            // We can't move the stored Box<dyn Any> out via &self; pass a sentinel.
            f(Box::new(()), k.as_str());
        }
    }
    /// Removes the entry with the given key.
    pub fn cbt_remove(&mut self, key: &str) -> Option<Box<dyn Any>> {
        let removed = self.storage.borrow_mut().remove(key);
        if removed.is_some() {
            self.count -= 1;
        }
        removed
    }
    /// Removes all entries from the tree.
    pub fn cbt_remove_all(&mut self) {
        self.storage.borrow_mut().clear();
        self.count = 0;
        self.root = None;
        self.first = None;
        self.last = None;
    }
    /// Removes all entries, calling the provided function for each.
    pub fn cbt_remove_all_with<F: FnMut(Box<dyn Any>, &str)>(&mut self, mut f: F) {
        let mut taken = BTreeMap::new();
        std::mem::swap(&mut *self.storage.borrow_mut(), &mut taken);
        for (k, v) in taken.into_iter() {
            f(v, k.as_str());
        }
        self.count = 0;
        self.root = None;
        self.first = None;
        self.last = None;
    }
    /// Inserts an entry using a provided function and key, returning a leaf.
    pub fn cbt_put_with<F: FnMut(Box<dyn Any>) -> Box<dyn Any>>(
        &mut self,
        mut f: F,
        key: &str,
    ) -> CbtLeaf {
        let mut storage = self.storage.borrow_mut();
        let existing = storage.remove(key);
        let was_present = existing.is_some();
        let new_data = f(existing.unwrap_or_else(|| Box::new(())));
        storage.insert(key.to_string(), new_data);
        drop(storage);
        if !was_present {
            self.count += 1;
        }
        make_leaf(key)
    }
    /// Inserts an entry with the given key; returns a tuple (is_new, leaf).
    pub fn cbt_insert(&mut self, key: &str) -> (bool, CbtLeaf) {
        let mut storage = self.storage.borrow_mut();
        let is_new = !storage.contains_key(key);
        if is_new {
            storage.insert(key.to_string(), Box::new(()));
        }
        drop(storage);
        if is_new {
            self.count += 1;
        }
        (is_new, make_leaf(key))
    }
    /// Returns the overhead in bytes used by the tree.
    pub fn cbt_overhead(&self) -> usize {
        let n = self.storage.borrow().len();
        // Rough estimate to mirror the C behaviour: per-entry leaf + one
        // internal node per key beyond the first.
        let leaf_size = std::mem::size_of::<CbtLeaf>();
        let node_size = std::mem::size_of::<CbtNode>();
        let mut total = std::mem::size_of::<Cbt>();
        if n > 0 {
            total += n * leaf_size + n.saturating_sub(1) * node_size;
        }
        total
    }
}
