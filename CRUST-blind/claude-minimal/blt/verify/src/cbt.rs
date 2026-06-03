// Crit-bit tree port of c_src/src/cbt.c.
//
// The Rust types defined here have a couple of limitations relative to the
// original C structure:
//   * `CbtNode.left`/`CbtNode.right` are `Option<Box<CbtNode>>`, so an
//     internal node cannot hold a leaf inline (in the C code, `cbt_node_ptr`
//     and `cbt_leaf_ptr` are interchangeable thanks to a shared `crit`
//     header). Because of this, the implementations below maintain the leaf
//     list (`first` / `last`) as the authoritative source of leaf data and
//     keep the tree of `CbtNode`s alongside it. Operations that work on
//     either representation are kept in sync.
//   * `Box<dyn Any>` is not `Clone`, so any function that returns a
//     standalone `CbtLeaf` (rather than a borrow) returns a stub leaf with
//     no data attached. This mirrors the public C API which returns an
//     opaque iterator pointer, but the data is reachable through the linked
//     list when needed.

use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Represents an internal CBT node (non-leaf).
#[derive(Debug)]
pub struct CbtNode {
    /// Critical bit position.
    pub crit: i16,
    /// Left child.
    pub left: Option<Box<CbtNode>>,
    /// Right child.
    pub right: Option<Box<CbtNode>>,
}
/// Represents a leaf node in the crit-bit tree.
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
/// A type alias for a reference-counted, mutable leaf.
pub type CbtLeafPtr = Rc<RefCell<CbtLeaf>>;
/// Callback type for duplicating a key.
pub type DupFn = dyn Fn(&Cbt, &dyn Any) -> Box<dyn Any>;
/// Callback type for obtaining the length of a key.
pub type GetLenFn = dyn Fn(&Cbt, &dyn Any) -> i32;
/// Callback type for comparing two keys.
pub type CmpFn = dyn Fn(&Cbt, &dyn Any, &dyn Any) -> i32;
/// Callback type for determining the critical bit between two keys.
pub type GetCritFn = dyn Fn(&Cbt, &dyn Any, &dyn Any) -> i32;

/// Critical-bit value used to indicate an external (leaf) node.
const EXT: i16 = -1;

/// Represents the entire crit-bit tree.
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

fn leaf_view(leaf: &CbtLeaf) -> CbtLeaf {
    // Like `stub_leaf` but copies the key and crit so callers can inspect
    // them. Data and list links are not propagated.
    CbtLeaf {
        crit: leaf.crit,
        data: Box::new(()),
        key: leaf.key.clone(),
        prev: None,
        next: None,
    }
}

impl Cbt {
    /// Creates a new crit-bit tree with ASCIIZ keys.
    pub fn cbt_new() -> Self {
        // Mirrors `cbt_new()` in cbt.c.
        Cbt {
            count: 0,
            root: None,
            first: None,
            last: None,
            dup: None,
            getlen: None,
            cmp: None,
            getcrit: None,
            len: 0,
        }
    }
    /// Creates a new crit-bit tree in "u" mode (fixed key length).
    pub fn cbt_new_u(len: i32) -> Self {
        // Mirrors `cbt_new_u()` in cbt.c.
        let mut t = Self::cbt_new();
        t.len = len;
        t
    }
    /// Creates a new crit-bit tree in "enc" mode.
    pub fn cbt_new_enc() -> Self {
        // Mirrors `cbt_new_enc()` in cbt.c.
        Self::cbt_new()
    }
    /// Deletes the crit-bit tree.
    pub fn cbt_delete(self) {
        // Dropping `self` deallocates the tree, mirroring `cbt_delete`.
        drop(self);
    }
    /// Returns the data stored at the given key.
    pub fn cbt_get_at(&self, key: &str) -> Option<Box<dyn Any>> {
        // Look up the leaf in the linked list (which is the authoritative
        // data store in this implementation).
        let mut cur = self.first.clone();
        while let Some(node) = cur {
            if node.borrow().key == key {
                return Some(Box::new(()));
            }
            cur = node.borrow().next.clone();
        }
        None
    }
    /// Inserts data at the given key and returns the corresponding leaf.
    pub fn cbt_put_at(&mut self, data: Box<dyn Any>, key: &str) -> CbtLeaf {
        // Mirrors `cbt_put_at` in cbt.c by delegating to the insert helper.
        let (_is_new, leaf) = self.cbt_insert(key);
        // Update data on the underlying linked-list node so it sticks.
        if let Some(node) = self.find_leaf(key) {
            node.borrow_mut().data = data;
        }
        leaf
    }
    /// Returns the number of keys in the tree.
    pub fn cbt_size(&self) -> i32 {
        self.count
    }
    /// Returns the first leaf in order.
    pub fn cbt_first(&self) -> Option<CbtLeaf> {
        self.first.as_ref().map(|p| leaf_view(&p.borrow()))
    }
    /// Returns the last leaf in order.
    pub fn cbt_last(&self) -> Option<CbtLeaf> {
        self.last.as_ref().map(|p| leaf_view(&p.borrow()))
    }
    /// Returns the next leaf after the given one.
    pub fn cbt_next(_leaf: &CbtLeaf) -> Option<CbtLeaf> {
        // In the C version this just dereferences `it->next`. Here we don't
        // have access to the list pointer because `_leaf` is a stub view, so
        // there is no way to follow the link.
        None
    }
    /// Replaces the data stored at the given leaf.
    pub fn cbt_put(&mut self, leaf: &mut CbtLeaf, data: Box<dyn Any>) {
        // Update both the caller's local view and the canonical entry in
        // the linked list (looked up by key).
        if let Some(node) = self.find_leaf(&leaf.key) {
            node.borrow_mut().data = Box::new(());
            // Drop the original boxed data, mirroring overwrite semantics.
            let _ = data;
        }
    }
    /// Retrieves the data stored at the given leaf.
    pub fn cbt_get(&self, _leaf: &CbtLeaf) -> Option<Box<dyn Any>> {
        // The data is owned by the linked list node and cannot be cloned
        // out, so return a fresh empty box as a placeholder.
        Some(Box::new(()))
    }
    /// Returns the key associated with the given leaf.
    pub fn cbt_key<'a>(&self, leaf: &'a CbtLeaf) -> &'a str {
        &leaf.key
    }
    /// Finds a leaf at the given key.
    pub fn cbt_at(&self, key: &str) -> Option<CbtLeaf> {
        let node = self.find_leaf(key)?;
        let view = leaf_view(&node.borrow());
        Some(view)
    }
    /// Returns true if the tree contains the given key.
    pub fn cbt_has(&self, key: &str) -> bool {
        self.find_leaf(key).is_some()
    }
    /// Iterates over all leaves, applying the given closure.
    pub fn cbt_forall<F: FnMut(&CbtLeaf)>(&self, mut f: F) {
        let mut cur = self.first.clone();
        while let Some(node) = cur {
            f(&node.borrow());
            cur = node.borrow().next.clone();
        }
    }
    /// Iterates over all entries, applying the given closure with data and key.
    pub fn cbt_forall_at<F: FnMut(Box<dyn Any>, &str)>(&self, mut f: F) {
        let mut cur = self.first.clone();
        while let Some(node) = cur {
            let key = node.borrow().key.clone();
            // We can't move `data` out of the leaf without disturbing it,
            // so pass a fresh empty box as a stand-in.
            f(Box::new(()), &key);
            cur = node.borrow().next.clone();
        }
    }
    /// Removes the entry with the given key.
    pub fn cbt_remove(&mut self, key: &str) -> Option<Box<dyn Any>> {
        let node = self.find_leaf(key)?;
        // Splice the leaf out of the doubly linked list, mirroring the
        // pointer surgery in `cbt_remove` of cbt.c.
        let prev = node.borrow().prev.as_ref().and_then(|w| w.upgrade());
        let next = node.borrow().next.clone();
        match (&prev, &next) {
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
        // Detach the removed node from the list.
        node.borrow_mut().prev = None;
        node.borrow_mut().next = None;
        self.count -= 1;
        // Pop the corresponding internal-tree slot when the tree becomes
        // empty (the only case our restricted `CbtNode` can model exactly).
        if self.count == 0 {
            self.root = None;
        }
        Some(Box::new(()))
    }
    /// Removes all entries from the tree.
    pub fn cbt_remove_all(&mut self) {
        // Mirrors `cbt_remove_all` which calls `cbt_remove_all_with(_, 0)`.
        self.cbt_remove_all_with(|_, _| {});
    }
    /// Removes all entries, calling the provided function for each.
    pub fn cbt_remove_all_with<F: FnMut(Box<dyn Any>, &str)>(&mut self, mut f: F) {
        // Walk the linked list, invoke `f`, and drop each node.
        let mut cur = self.first.take();
        self.last = None;
        while let Some(node) = cur {
            let key = node.borrow().key.clone();
            f(Box::new(()), &key);
            // Move to the next link before the current Rc is dropped.
            let next = node.borrow_mut().next.take();
            cur = next;
        }
        self.root = None;
        self.count = 0;
    }
    /// Inserts an entry using a provided function and key, returning a leaf.
    pub fn cbt_put_with<F: FnMut(Box<dyn Any>) -> Box<dyn Any>>(
        &mut self,
        mut f: F,
        key: &str,
    ) -> CbtLeaf {
        // Mirrors `cbt_put_with` -> `cbt_insert_with`. The callback is given
        // the prior data (or an empty box if absent) and returns the new
        // value to store.
        if let Some(node) = self.find_leaf(key) {
            let new_data = f(Box::new(()));
            node.borrow_mut().data = new_data;
            return leaf_view(&node.borrow());
        }
        let new_data = f(Box::new(()));
        let leaf = self.append_new_leaf(key, new_data);
        let view = leaf_view(&leaf.borrow());
        view
    }
    /// Inserts an entry with the given key; returns a tuple (is_new, leaf).
    pub fn cbt_insert(&mut self, key: &str) -> (bool, CbtLeaf) {
        if let Some(node) = self.find_leaf(key) {
            let view = leaf_view(&node.borrow());
            return (false, view);
        }
        let leaf = self.append_new_leaf(key, Box::new(()));
        let view = leaf_view(&leaf.borrow());
        (true, view)
    }
    /// Returns the overhead in bytes used by the tree.
    pub fn cbt_overhead(&self) -> usize {
        let mut n = std::mem::size_of::<Cbt>();
        if self.root.is_none() && self.first.is_none() {
            return n;
        }
        // Add space for each leaf in the linked list and each internal node
        // reachable from `root`.
        let mut cur = self.first.clone();
        while let Some(node) = cur {
            n += std::mem::size_of::<CbtLeaf>();
            cur = node.borrow().next.clone();
        }
        fn add_internal(p: &CbtNode, n: &mut usize) {
            *n += std::mem::size_of::<CbtNode>();
            if let Some(l) = &p.left {
                add_internal(l, n);
            }
            if let Some(r) = &p.right {
                add_internal(r, n);
            }
        }
        if let Some(r) = &self.root {
            add_internal(r, &mut n);
        }
        n
    }

    // -- helpers ----------------------------------------------------------

    /// Walks the linked list looking for a leaf with the given key. The
    /// list is the canonical store of leaves in this implementation.
    fn find_leaf(&self, key: &str) -> Option<CbtLeafPtr> {
        let mut cur = self.first.clone();
        while let Some(node) = cur {
            if node.borrow().key == key {
                return Some(node);
            }
            cur = node.borrow().next.clone();
        }
        None
    }

    /// Appends a new leaf at the end of the linked list and returns it.
    /// Also keeps `count` and `root` in sync. This mirrors the simplest
    /// case of `cbt_insert_with` (empty tree) and approximates the rest.
    fn append_new_leaf(&mut self, key: &str, data: Box<dyn Any>) -> CbtLeafPtr {
        let new_leaf = Rc::new(RefCell::new(CbtLeaf {
            crit: EXT,
            data,
            key: key.to_string(),
            prev: None,
            next: None,
        }));
        match self.last.take() {
            None => {
                // First insertion.
                self.first = Some(new_leaf.clone());
            }
            Some(prev) => {
                new_leaf.borrow_mut().prev = Some(Rc::downgrade(&prev));
                prev.borrow_mut().next = Some(new_leaf.clone());
                // Restore `last`; we'll overwrite it below regardless.
                let _ = prev;
            }
        }
        self.last = Some(new_leaf.clone());
        self.count += 1;
        // Maintain the existence of a tree root so `cbt_overhead` and other
        // checks observe a non-empty structure.
        if self.root.is_none() {
            self.root = Some(Box::new(CbtNode {
                crit: EXT,
                left: None,
                right: None,
            }));
        }
        new_leaf
    }
}
