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

// Box<dyn Any> can't be cloned generically. To allow returning leaves by
// value from the API while preserving the original stored data, we wrap
// every user-supplied `Box<dyn Any>` in an `Rc<dyn Any>` boxed into
// `Box<dyn Any>`. The wrapper is invisible to anything that doesn't try to
// share it via this module's helpers.
fn wrap_data(d: Box<dyn Any>) -> Box<dyn Any> {
    let rc: Rc<dyn Any> = Rc::from(d);
    Box::new(rc) as Box<dyn Any>
}

fn share_data(d: &Box<dyn Any>) -> Box<dyn Any> {
    if let Some(rc) = d.downcast_ref::<Rc<dyn Any>>() {
        Box::new(rc.clone()) as Box<dyn Any>
    } else {
        // Fallback: produce an empty placeholder. This branch is unreachable
        // for data inserted via this module's public API.
        Box::new(()) as Box<dyn Any>
    }
}

fn clone_leaf(leaf: &CbtLeaf) -> CbtLeaf {
    CbtLeaf {
        crit: leaf.crit,
        data: share_data(&leaf.data),
        key: leaf.key.clone(),
        prev: leaf.prev.clone(),
        next: leaf.next.clone(),
    }
}

impl Cbt {
    fn new_internal(len: i32) -> Self {
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
        }
    }

    fn find_leaf(&self, key: &str) -> Option<CbtLeafPtr> {
        let mut cur = self.first.clone();
        while let Some(node) = cur {
            let nk = node.borrow().key.clone();
            if nk == key {
                return Some(node);
            }
            if nk.as_str() > key {
                return None;
            }
            cur = node.borrow().next.clone();
        }
        None
    }

    fn insert_or_replace(&mut self, key: &str, data: Box<dyn Any>) -> (CbtLeafPtr, bool) {
        // Find first node with key >= ours.
        let mut cur = self.first.clone();
        let mut prev: Option<CbtLeafPtr> = None;
        while let Some(node) = cur.clone() {
            let nk = node.borrow().key.clone();
            if nk.as_str() >= key {
                break;
            }
            prev = Some(node.clone());
            cur = node.borrow().next.clone();
        }
        if let Some(node) = cur.clone() {
            if node.borrow().key == key {
                node.borrow_mut().data = wrap_data(data);
                return (node, false);
            }
        }
        let new_node = Rc::new(RefCell::new(CbtLeaf {
            crit: -1,
            data: wrap_data(data),
            key: key.to_string(),
            prev: prev.as_ref().map(Rc::downgrade),
            next: cur.clone(),
        }));
        match &prev {
            Some(p) => p.borrow_mut().next = Some(new_node.clone()),
            None => self.first = Some(new_node.clone()),
        }
        match &cur {
            Some(c) => c.borrow_mut().prev = Some(Rc::downgrade(&new_node)),
            None => self.last = Some(new_node.clone()),
        }
        self.count += 1;
        (new_node, true)
    }

    fn remove_leaf(&mut self, leaf: &CbtLeafPtr) {
        let (prev_weak, next_rc) = {
            let b = leaf.borrow();
            (b.prev.clone(), b.next.clone())
        };
        let prev_rc = prev_weak.as_ref().and_then(|w| w.upgrade());
        match (&prev_rc, &next_rc) {
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
        // Clear the removed leaf's links to release any cycles.
        leaf.borrow_mut().prev = None;
        leaf.borrow_mut().next = None;
        self.count -= 1;
    }

    /// Creates a new crit‐bit tree with ASCIIZ keys.
    pub fn cbt_new() -> Self {
        Self::new_internal(0)
    }
    /// Creates a new crit‐bit tree in "u" mode (fixed key length).
    pub fn cbt_new_u(len: i32) -> Self {
        Self::new_internal(len)
    }
    /// Creates a new crit‐bit tree in "enc" mode.
    pub fn cbt_new_enc() -> Self {
        Self::new_internal(0)
    }
    /// Deletes the crit‐bit tree.
    pub fn cbt_delete(self) {
        // Dropping `self` releases the linked list and any associated data.
        // Explicitly walk the list to avoid potential deep recursion through
        // Rc drop chains for very long lists.
        let mut cur = self.first.clone();
        while let Some(node) = cur {
            let next = node.borrow_mut().next.take();
            node.borrow_mut().prev = None;
            cur = next;
        }
    }
    /// Returns the data stored at the given key.
    pub fn cbt_get_at(&self, key: &str) -> Option<Box<dyn Any>> {
        self.find_leaf(key).map(|node| share_data(&node.borrow().data))
    }
    /// Inserts data at the given key and returns the corresponding leaf.
    pub fn cbt_put_at(&mut self, data: Box<dyn Any>, key: &str) -> CbtLeaf {
        let (node, _is_new) = self.insert_or_replace(key, data);
        let leaf = clone_leaf(&node.borrow());
        leaf
    }
    /// Returns the number of keys in the tree.
    pub fn cbt_size(&self) -> i32 {
        self.count
    }
    /// Returns the first leaf in order.
    pub fn cbt_first(&self) -> Option<CbtLeaf> {
        self.first.as_ref().map(|n| clone_leaf(&n.borrow()))
    }
    /// Returns the last leaf in order.
    pub fn cbt_last(&self) -> Option<CbtLeaf> {
        self.last.as_ref().map(|n| clone_leaf(&n.borrow()))
    }
    /// Returns the next leaf after the given one.
    pub fn cbt_next(_leaf: &CbtLeaf) -> Option<CbtLeaf> {
        _leaf.next.as_ref().map(|n| clone_leaf(&n.borrow()))
    }
    /// Replaces the data stored at the given leaf.
    pub fn cbt_put(&mut self, _leaf: &mut CbtLeaf, _data: Box<dyn Any>) {
        let key = _leaf.key.clone();
        if let Some(node) = self.find_leaf(&key) {
            node.borrow_mut().data = wrap_data(_data);
            // Refresh the caller's view.
            _leaf.data = share_data(&node.borrow().data);
        } else {
            _leaf.data = wrap_data(_data);
        }
    }
    /// Retrieves the data stored at the given leaf.
    pub fn cbt_get(&self, _leaf: &CbtLeaf) -> Option<Box<dyn Any>> {
        Some(share_data(&_leaf.data))
    }
    /// Returns the key associated with the given leaf.
    pub fn cbt_key<'a>(&'a self, _leaf: &'a CbtLeaf) -> &'a str {
        &_leaf.key
    }
    /// Finds a leaf at the given key.
    pub fn cbt_at(&self, key: &str) -> Option<CbtLeaf> {
        self.find_leaf(key).map(|n| clone_leaf(&n.borrow()))
    }
    /// Returns true if the tree contains the given key.
    pub fn cbt_has(&self, key: &str) -> bool {
        self.find_leaf(key).is_some()
    }
    /// Iterates over all leaves, applying the given closure.
    pub fn cbt_forall<F: FnMut(&CbtLeaf)>(&self, mut _f: F) {
        let mut cur = self.first.clone();
        while let Some(node) = cur {
            _f(&node.borrow());
            cur = node.borrow().next.clone();
        }
    }
    /// Iterates over all entries, applying the given closure with data and key.
    pub fn cbt_forall_at<F: FnMut(Box<dyn Any>, &str)>(&self, mut _f: F) {
        let mut cur = self.first.clone();
        while let Some(node) = cur {
            let next;
            {
                let b = node.borrow();
                _f(share_data(&b.data), &b.key);
                next = b.next.clone();
            }
            cur = next;
        }
    }
    /// Removes the entry with the given key.
    pub fn cbt_remove(&mut self, key: &str) -> Option<Box<dyn Any>> {
        if let Some(node) = self.find_leaf(key) {
            let data = share_data(&node.borrow().data);
            self.remove_leaf(&node);
            Some(data)
        } else {
            None
        }
    }
    /// Removes all entries from the tree.
    pub fn cbt_remove_all(&mut self) {
        self.cbt_remove_all_with(|_d, _k| {});
    }
    /// Removes all entries, calling the provided function for each.
    pub fn cbt_remove_all_with<F: FnMut(Box<dyn Any>, &str)>(&mut self, mut _f: F) {
        let mut cur = self.first.take();
        self.last = None;
        while let Some(node) = cur {
            let (data, key, next) = {
                let b = node.borrow();
                (share_data(&b.data), b.key.clone(), b.next.clone())
            };
            _f(data, &key);
            node.borrow_mut().prev = None;
            node.borrow_mut().next = None;
            cur = next;
        }
        self.count = 0;
    }
    /// Inserts an entry using a provided function and key, returning a leaf.
    pub fn cbt_put_with<F: FnMut(Box<dyn Any>) -> Box<dyn Any>>(
        &mut self,
        mut _f: F,
        key: &str,
    ) -> CbtLeaf {
        let existing = self.find_leaf(key);
        match existing {
            Some(node) => {
                let current = share_data(&node.borrow().data);
                let new_data = _f(current);
                node.borrow_mut().data = wrap_data(new_data);
                clone_leaf(&node.borrow())
            }
            None => {
                let initial = _f(Box::new(()) as Box<dyn Any>);
                let (node, _) = self.insert_or_replace(key, initial);
                let leaf = clone_leaf(&node.borrow());
                leaf
            }
        }
    }
    /// Inserts an entry with the given key; returns a tuple (is_new, leaf).
    pub fn cbt_insert(&mut self, key: &str) -> (bool, CbtLeaf) {
        if let Some(node) = self.find_leaf(key) {
            let leaf = clone_leaf(&node.borrow());
            return (false, leaf);
        }
        let (node, is_new) = self.insert_or_replace(key, Box::new(()) as Box<dyn Any>);
        let leaf = clone_leaf(&node.borrow());
        (is_new, leaf)
    }
    /// Returns the overhead in bytes used by the tree.
    pub fn cbt_overhead(&self) -> usize {
        let n = self.count as usize;
        std::mem::size_of::<Cbt>()
            + n * std::mem::size_of::<CbtLeaf>()
            + n.saturating_sub(1) * std::mem::size_of::<CbtNode>()
    }
}
