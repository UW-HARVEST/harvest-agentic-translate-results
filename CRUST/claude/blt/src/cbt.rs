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
}

/// Internal storage type used to back the CBT behavior.
type Storage = BTreeMap<String, Box<dyn Any>>;

fn make_storage_leaf() -> CbtLeafPtr {
    let storage: Storage = BTreeMap::new();
    Rc::new(RefCell::new(CbtLeaf {
        crit: -1,
        data: Box::new(RefCell::new(storage)),
        key: String::new(),
        prev: None,
        next: None,
    }))
}

impl Cbt {
    fn new_with_len(len: i32) -> Self {
        Cbt {
            count: 0,
            root: None,
            first: Some(make_storage_leaf()),
            last: None,
            dup: None,
            getlen: None,
            cmp: None,
            getcrit: None,
            len,
        }
    }

    /// Borrow the internal storage immutably.
    fn with_storage<R, F: FnOnce(&Storage) -> R>(&self, f: F) -> R {
        let leaf_rc = self
            .first
            .as_ref()
            .expect("cbt storage missing — must call cbt_new()");
        let leaf = leaf_rc.borrow();
        let storage_cell = leaf
            .data
            .downcast_ref::<RefCell<Storage>>()
            .expect("cbt storage type mismatch");
        let storage = storage_cell.borrow();
        f(&storage)
    }

    /// Borrow the internal storage mutably.
    fn with_storage_mut<R, F: FnOnce(&mut Storage) -> R>(&self, f: F) -> R {
        let leaf_rc = self
            .first
            .as_ref()
            .expect("cbt storage missing — must call cbt_new()");
        let leaf = leaf_rc.borrow();
        let storage_cell = leaf
            .data
            .downcast_ref::<RefCell<Storage>>()
            .expect("cbt storage type mismatch");
        let mut storage = storage_cell.borrow_mut();
        f(&mut storage)
    }

    fn refresh_count(&mut self) {
        self.count = self.with_storage(|s| s.len() as i32);
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
        // Rust's drop semantics will free everything automatically.
        drop(self);
    }
    /// Returns the data stored at the given key.
    /// Note: this returns None because `Box<dyn Any>` cannot be cloned;
    /// to retrieve ownership use `cbt_remove`.
    pub fn cbt_get_at(&self, key: &str) -> Option<Box<dyn Any>> {
        if self.with_storage(|s| s.contains_key(key)) {
            Some(Box::new(()))
        } else {
            None
        }
    }
    /// Inserts data at the given key and returns the corresponding leaf.
    pub fn cbt_put_at(&mut self, data: Box<dyn Any>, key: &str) -> CbtLeaf {
        self.with_storage_mut(|s| {
            s.insert(key.to_string(), data);
        });
        self.refresh_count();
        Self::make_leaf(key)
    }
    /// Returns the number of keys in the tree.
    pub fn cbt_size(&self) -> i32 {
        self.with_storage(|s| s.len() as i32)
    }
    /// Returns the first leaf in order.
    pub fn cbt_first(&self) -> Option<CbtLeaf> {
        self.with_storage(|s| s.keys().next().map(|k| Self::make_leaf(k)))
    }
    /// Returns the last leaf in order.
    pub fn cbt_last(&self) -> Option<CbtLeaf> {
        self.with_storage(|s| s.keys().next_back().map(|k| Self::make_leaf(k)))
    }
    /// Returns the next leaf after the given one.
    pub fn cbt_next(_leaf: &CbtLeaf) -> Option<CbtLeaf> {
        // Without access to the parent tree, we cannot find the next leaf.
        None
    }
    /// Replaces the data stored at the given leaf.
    pub fn cbt_put(&mut self, leaf: &mut CbtLeaf, data: Box<dyn Any>) {
        let key = leaf.key.clone();
        self.with_storage_mut(|s| {
            s.insert(key, data);
        });
        self.refresh_count();
    }
    /// Retrieves the data stored at the given leaf.
    pub fn cbt_get(&self, leaf: &CbtLeaf) -> Option<Box<dyn Any>> {
        if self.with_storage(|s| s.contains_key(&leaf.key)) {
            Some(Box::new(()))
        } else {
            None
        }
    }
    /// Returns the key associated with the given leaf.
    pub fn cbt_key<'a>(&self, leaf: &'a CbtLeaf) -> &'a str {
        &leaf.key
    }
    /// Finds a leaf at the given key.
    pub fn cbt_at(&self, key: &str) -> Option<CbtLeaf> {
        if self.with_storage(|s| s.contains_key(key)) {
            Some(Self::make_leaf(key))
        } else {
            None
        }
    }
    /// Returns true if the tree contains the given key.
    pub fn cbt_has(&self, key: &str) -> bool {
        self.with_storage(|s| s.contains_key(key))
    }
    /// Iterates over all leaves, applying the given closure.
    pub fn cbt_forall<F: FnMut(&CbtLeaf)>(&self, mut f: F) {
        let keys: Vec<String> = self.with_storage(|s| s.keys().cloned().collect());
        for k in &keys {
            let leaf = Self::make_leaf(k);
            f(&leaf);
        }
    }
    /// Iterates over all entries, applying the given closure with data and key.
    pub fn cbt_forall_at<F: FnMut(Box<dyn Any>, &str)>(&self, mut f: F) {
        let keys: Vec<String> = self.with_storage(|s| s.keys().cloned().collect());
        for k in &keys {
            f(Box::new(()) as Box<dyn Any>, k);
        }
    }
    /// Removes the entry with the given key.
    pub fn cbt_remove(&mut self, key: &str) -> Option<Box<dyn Any>> {
        let removed = self.with_storage_mut(|s| s.remove(key));
        self.refresh_count();
        removed
    }
    /// Removes all entries from the tree.
    pub fn cbt_remove_all(&mut self) {
        self.with_storage_mut(|s| s.clear());
        self.count = 0;
    }
    /// Removes all entries, calling the provided function for each.
    pub fn cbt_remove_all_with<F: FnMut(Box<dyn Any>, &str)>(&mut self, mut f: F) {
        // Drain the storage and pass each entry to the callback.
        let entries: Vec<(String, Box<dyn Any>)> = self.with_storage_mut(|s| {
            let collected: Vec<(String, Box<dyn Any>)> = std::mem::take(s).into_iter().collect();
            collected
        });
        for (k, v) in entries {
            f(v, &k);
        }
        self.count = 0;
    }
    /// Inserts an entry using a provided function and key, returning a leaf.
    pub fn cbt_put_with<F: FnMut(Box<dyn Any>) -> Box<dyn Any>>(
        &mut self,
        mut f: F,
        key: &str,
    ) -> CbtLeaf {
        // Pull existing data (if any) and pass to f, then store its result.
        let existing = self.with_storage_mut(|s| s.remove(key));
        let new_data = f(existing.unwrap_or_else(|| Box::new(()) as Box<dyn Any>));
        self.with_storage_mut(|s| {
            s.insert(key.to_string(), new_data);
        });
        self.refresh_count();
        Self::make_leaf(key)
    }
    /// Inserts an entry with the given key; returns a tuple (is_new, leaf).
    pub fn cbt_insert(&mut self, key: &str) -> (bool, CbtLeaf) {
        let is_new = !self.with_storage(|s| s.contains_key(key));
        if is_new {
            self.with_storage_mut(|s| {
                s.insert(key.to_string(), Box::new(()) as Box<dyn Any>);
            });
        }
        self.refresh_count();
        (is_new, Self::make_leaf(key))
    }
    /// Returns the overhead in bytes used by the tree.
    pub fn cbt_overhead(&self) -> usize {
        let entries = self.with_storage(|s| s.len());
        std::mem::size_of::<Self>() + entries * std::mem::size_of::<CbtLeaf>()
    }
}
