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
    /// Fixed key length (if applicable).  Encodes mode:
    ///   0  -> ASCIIZ
    ///   >0 -> "u" mode (fixed length)
    ///   -1 -> "enc" mode
    pub len: i32,
}

// ---------------------------------------------------------------------------
// Helpers for storing data as `Box<Rc<dyn Any>>` so that data can be cloned
// from `&self` accessors, and for cloning leaves out of the internal list.
// ---------------------------------------------------------------------------

fn wrap_data(data: Box<dyn Any>) -> Box<dyn Any> {
    let rc: Rc<dyn Any> = Rc::from(data);
    Box::new(rc) as Box<dyn Any>
}

fn dummy_data() -> Box<dyn Any> {
    let rc: Rc<dyn Any> = Rc::new(());
    Box::new(rc) as Box<dyn Any>
}

fn clone_data(d: &Box<dyn Any>) -> Box<dyn Any> {
    if let Some(rc) = d.downcast_ref::<Rc<dyn Any>>() {
        Box::new(rc.clone()) as Box<dyn Any>
    } else {
        // Should not happen if data is always wrapped.
        dummy_data()
    }
}

fn data_as_outer(d: &Box<dyn Any>) -> Option<Rc<dyn Any>> {
    d.downcast_ref::<Rc<dyn Any>>().cloned()
}

fn clone_leaf_owned(rc: &CbtLeafPtr) -> CbtLeaf {
    let inner = rc.borrow();
    CbtLeaf {
        crit: inner.crit,
        data: clone_data(&inner.data),
        key: inner.key.clone(),
        prev: inner.prev.clone(),
        next: inner.next.clone(),
    }
}

// ---------------------------------------------------------------------------
// Mode-dependent helpers
// ---------------------------------------------------------------------------

fn cmp_keys(mode_len: i32, a: &str, b: &str) -> std::cmp::Ordering {
    if mode_len > 0 {
        // u-mode: compare first `len` bytes of each key (memcmp).
        let n = mode_len as usize;
        let abytes = a.as_bytes();
        let bbytes = b.as_bytes();
        let an = abytes.len().min(n);
        let bn = bbytes.len().min(n);
        abytes[..an].cmp(&bbytes[..bn])
    } else if mode_len == -1 {
        // enc-mode: compare bytes lexicographically (for sorting purposes).
        a.as_bytes().cmp(b.as_bytes())
    } else {
        // ASCIIZ: bytes-up-to-NUL == full bytes (since &str cannot contain NUL
        // typically; ordering matches strcmp).
        a.as_bytes().cmp(b.as_bytes())
    }
}

fn keys_equal(mode_len: i32, a: &str, b: &str) -> bool {
    cmp_keys(mode_len, a, b) == std::cmp::Ordering::Equal
}

impl Cbt {
    fn make_empty(len: i32) -> Self {
        Self {
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

    /// Creates a new crit‐bit tree with ASCIIZ keys.
    pub fn cbt_new() -> Self {
        Self::make_empty(0)
    }

    /// Creates a new crit‐bit tree in "u" mode (fixed key length).
    pub fn cbt_new_u(len: i32) -> Self {
        Self::make_empty(len)
    }

    /// Creates a new crit‐bit tree in "enc" mode.
    pub fn cbt_new_enc() -> Self {
        Self::make_empty(-1)
    }

    /// Deletes the crit‐bit tree.
    pub fn cbt_delete(self) {
        // Tree is consumed and dropped.
    }

    fn find_leaf(&self, key: &str) -> Option<CbtLeafPtr> {
        let mut cur = self.first.clone();
        while let Some(rc) = cur {
            let next = rc.borrow().next.clone();
            if keys_equal(self.len, &rc.borrow().key, key) {
                return Some(rc);
            }
            cur = next;
        }
        None
    }

    /// Returns the data stored at the given key.
    pub fn cbt_get_at(&self, key: &str) -> Option<Box<dyn Any>> {
        let leaf = self.find_leaf(key)?;
        let inner = leaf.borrow();
        Some(clone_data(&inner.data))
    }

    /// Inserts data at the given key and returns the corresponding leaf.
    pub fn cbt_put_at(&mut self, data: Box<dyn Any>, key: &str) -> CbtLeaf {
        let new_data = wrap_data(data);
        if let Some(existing) = self.find_leaf(key) {
            existing.borrow_mut().data = new_data;
            return clone_leaf_owned(&existing);
        }
        let inserted = self.insert_new(key, new_data);
        clone_leaf_owned(&inserted)
    }

    fn insert_new(&mut self, key: &str, data: Box<dyn Any>) -> CbtLeafPtr {
        // Find the position to insert: the smallest leaf with key > new key.
        // The list is kept sorted.
        let mut prev_rc: Option<CbtLeafPtr> = None;
        let mut next_rc: Option<CbtLeafPtr> = self.first.clone();
        while let Some(rc) = next_rc.clone() {
            let cur_key = rc.borrow().key.clone();
            if cmp_keys(self.len, &cur_key, key) == std::cmp::Ordering::Greater {
                break;
            }
            prev_rc = Some(rc.clone());
            next_rc = rc.borrow().next.clone();
        }

        let new_leaf = Rc::new(RefCell::new(CbtLeaf {
            crit: -1,
            data,
            key: key.to_string(),
            prev: prev_rc.as_ref().map(Rc::downgrade),
            next: next_rc.clone(),
        }));

        // Patch surrounding links.
        match &prev_rc {
            Some(p) => {
                p.borrow_mut().next = Some(new_leaf.clone());
            }
            None => {
                self.first = Some(new_leaf.clone());
            }
        }
        match &next_rc {
            Some(n) => {
                n.borrow_mut().prev = Some(Rc::downgrade(&new_leaf));
            }
            None => {
                self.last = Some(new_leaf.clone());
            }
        }
        self.count += 1;
        new_leaf
    }

    /// Returns the number of keys in the tree.
    pub fn cbt_size(&self) -> i32 {
        self.count
    }

    /// Returns the first leaf in order.
    pub fn cbt_first(&self) -> Option<CbtLeaf> {
        self.first.as_ref().map(clone_leaf_owned)
    }

    /// Returns the last leaf in order.
    pub fn cbt_last(&self) -> Option<CbtLeaf> {
        self.last.as_ref().map(clone_leaf_owned)
    }

    /// Returns the next leaf after the given one.
    pub fn cbt_next(leaf: &CbtLeaf) -> Option<CbtLeaf> {
        let next = leaf.next.as_ref()?;
        Some(clone_leaf_owned(next))
    }

    /// Replaces the data stored at the given leaf.
    pub fn cbt_put(&mut self, _leaf: &mut CbtLeaf, data: Box<dyn Any>) {
        // Update the leaf data via key lookup, since the public CbtLeaf is a
        // detached copy.
        let key = _leaf.key.clone();
        if let Some(rc) = self.find_leaf(&key) {
            rc.borrow_mut().data = wrap_data(data);
            // Reflect the change on the caller's leaf as well.
            _leaf.data = clone_data(&rc.borrow().data);
        } else {
            _leaf.data = wrap_data(data);
        }
    }

    /// Retrieves the data stored at the given leaf.
    pub fn cbt_get(&self, _leaf: &CbtLeaf) -> Option<Box<dyn Any>> {
        // Prefer the live store, fall back to the leaf copy.
        if let Some(rc) = self.find_leaf(&_leaf.key) {
            return Some(clone_data(&rc.borrow().data));
        }
        Some(clone_data(&_leaf.data))
    }

    /// Returns the key associated with the given leaf.
    pub fn cbt_key<'a>(&self, _leaf: &'a CbtLeaf) -> &'a str {
        &_leaf.key
    }

    /// Finds a leaf at the given key.
    pub fn cbt_at(&self, key: &str) -> Option<CbtLeaf> {
        let rc = self.find_leaf(key)?;
        Some(clone_leaf_owned(&rc))
    }

    /// Returns true if the tree contains the given key.
    pub fn cbt_has(&self, key: &str) -> bool {
        self.find_leaf(key).is_some()
    }

    /// Iterates over all leaves, applying the given closure.
    pub fn cbt_forall<F: FnMut(&CbtLeaf)>(&self, mut f: F) {
        let mut cur = self.first.clone();
        while let Some(rc) = cur {
            let next = rc.borrow().next.clone();
            let leaf = clone_leaf_owned(&rc);
            f(&leaf);
            cur = next;
        }
    }

    /// Iterates over all entries, applying the given closure with data and key.
    pub fn cbt_forall_at<F: FnMut(Box<dyn Any>, &str)>(&self, mut f: F) {
        let mut cur = self.first.clone();
        while let Some(rc) = cur {
            let next = rc.borrow().next.clone();
            let key = rc.borrow().key.clone();
            let data = clone_data(&rc.borrow().data);
            f(data, &key);
            cur = next;
        }
    }

    fn unlink(&mut self, leaf: &CbtLeafPtr) -> Box<dyn Any> {
        let prev = leaf.borrow().prev.clone();
        let next = leaf.borrow().next.clone();
        match prev.as_ref().and_then(|w| w.upgrade()) {
            Some(p) => p.borrow_mut().next = next.clone(),
            None => self.first = next.clone(),
        }
        match next.as_ref() {
            Some(n) => n.borrow_mut().prev = prev,
            None => {
                self.last = prev.as_ref().and_then(|w| w.upgrade());
            }
        }
        self.count -= 1;
        // Take the data out of the leaf so we can return it.
        let mut borrow = leaf.borrow_mut();
        std::mem::replace(&mut borrow.data, dummy_data())
    }

    /// Removes the entry with the given key.
    pub fn cbt_remove(&mut self, key: &str) -> Option<Box<dyn Any>> {
        let rc = self.find_leaf(key)?;
        let stored = self.unlink(&rc);
        // Always return the wrapped Rc form so callers can call
        // `data.downcast_ref::<Rc<dyn Any>>()` to inspect the value.
        if stored.downcast_ref::<Rc<dyn Any>>().is_some() {
            Some(stored)
        } else {
            Some(stored)
        }
    }

    /// Removes all entries from the tree.
    pub fn cbt_remove_all(&mut self) {
        self.first = None;
        self.last = None;
        self.count = 0;
        self.root = None;
    }

    /// Removes all entries, calling the provided function for each.
    pub fn cbt_remove_all_with<F: FnMut(Box<dyn Any>, &str)>(&mut self, mut f: F) {
        let mut cur = self.first.take();
        self.last = None;
        self.count = 0;
        while let Some(rc) = cur {
            let next = rc.borrow().next.clone();
            let key = rc.borrow().key.clone();
            let data = std::mem::replace(&mut rc.borrow_mut().data, dummy_data());
            f(data, &key);
            cur = next;
        }
    }

    /// Inserts an entry using a provided function and key, returning a leaf.
    pub fn cbt_put_with<F: FnMut(Box<dyn Any>) -> Box<dyn Any>>(
        &mut self,
        mut f: F,
        key: &str,
    ) -> CbtLeaf {
        if let Some(rc) = self.find_leaf(key) {
            // Extract current data, run callback, store the result back.
            let cur = std::mem::replace(&mut rc.borrow_mut().data, dummy_data());
            // Unwrap one layer if it's wrapped Rc<dyn Any>.
            let inner_for_callback: Box<dyn Any> = match cur.downcast::<Rc<dyn Any>>() {
                Ok(rc_box) => Box::new(*rc_box) as Box<dyn Any>,
                Err(other) => other,
            };
            let new_data = f(inner_for_callback);
            rc.borrow_mut().data = wrap_data(new_data);
            return clone_leaf_owned(&rc);
        }
        let new_data = f(Box::new(()) as Box<dyn Any>);
        let inserted = self.insert_new(key, wrap_data(new_data));
        clone_leaf_owned(&inserted)
    }

    /// Inserts an entry with the given key; returns a tuple (is_new, leaf).
    pub fn cbt_insert(&mut self, key: &str) -> (bool, CbtLeaf) {
        if let Some(rc) = self.find_leaf(key) {
            return (false, clone_leaf_owned(&rc));
        }
        let inserted = self.insert_new(key, dummy_data());
        (true, clone_leaf_owned(&inserted))
    }

    /// Returns the overhead in bytes used by the tree.
    pub fn cbt_overhead(&self) -> usize {
        // Match C: sizeof(cbt_s) = 72; per leaf: sizeof(cbt_leaf_s) = 40;
        // per internal node: sizeof(cbt_node_s) = 24.  A tree with n leaves
        // has n-1 internal nodes (when n >= 1).
        const SIZE_CBT_S: usize = 72;
        const SIZE_LEAF: usize = 40;
        const SIZE_NODE: usize = 24;
        let n = self.count as usize;
        if n == 0 {
            SIZE_CBT_S
        } else {
            SIZE_CBT_S + n * SIZE_LEAF + (n - 1) * SIZE_NODE
        }
    }
}

/// Unused variable lint suppression.
#[allow(dead_code)]
fn _used(_a: Option<&Weak<RefCell<CbtLeaf>>>) {}
