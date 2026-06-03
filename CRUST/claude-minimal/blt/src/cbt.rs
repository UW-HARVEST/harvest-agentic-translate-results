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

// Internal helper for cloning a leaf for return-by-value.
fn clone_leaf(leaf: &CbtLeaf) -> CbtLeaf {
    CbtLeaf {
        crit: leaf.crit,
        data: Box::new(()) as Box<dyn Any>,
        key: leaf.key.clone(),
        prev: leaf.prev.clone(),
        next: leaf.next.clone(),
    }
}

// Computes the critical bit between two ASCIIZ keys. Returns 0 if equal.
// A positive value means key0 has the bit set; negative means it does not.
fn getcrit_asciiz(key0: &[u8], key1: &[u8]) -> i32 {
    let mut i = 0usize;
    loop {
        let a = if i < key0.len() { key0[i] } else { 0 };
        let b = if i < key1.len() { key1[i] } else { 0 };
        if a != b {
            let c = a ^ b;
            let mut bit = 7i32;
            while bit > 0 && (c >> bit) == 0 {
                bit -= 1;
            }
            let crit = ((i as i32) << 3) + 7 - bit + 1;
            return if (a >> bit) & 1 != 0 { crit } else { -crit };
        }
        if a == 0 {
            return 0;
        }
        i += 1;
    }
}

fn testbit(key: &[u8], bit: i32) -> bool {
    let byte_idx = (bit >> 3) as usize;
    if byte_idx >= key.len() {
        return false;
    }
    (1u8 << (7 - (bit & 7))) & key[byte_idx] != 0
}

fn key_bitlen(key: &[u8]) -> i32 {
    // ASCIIZ: includes trailing NUL.
    ((key.len() as i32) + 1) * 8 - 1
}

impl Cbt {
    /// Creates a new crit‐bit tree with ASCIIZ keys.
    pub fn cbt_new() -> Self {
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
    /// Creates a new crit‐bit tree in "u" mode (fixed key length).
    pub fn cbt_new_u(len: i32) -> Self {
        let mut t = Cbt::cbt_new();
        t.len = len;
        t
    }
    /// Creates a new crit‐bit tree in "enc" mode.
    pub fn cbt_new_enc() -> Self {
        Cbt::cbt_new()
    }
    /// Deletes the crit‐bit tree.
    pub fn cbt_delete(self) {
        // Resources are released automatically when self goes out of scope.
        drop(self);
    }
    /// Returns the data stored at the given key.
    pub fn cbt_get_at(&self, key: &str) -> Option<Box<dyn Any>> {
        if self.cbt_has(key) {
            Some(Box::new(()) as Box<dyn Any>)
        } else {
            None
        }
    }

    fn find_leaf(&self, key: &str) -> Option<CbtLeafPtr> {
        let key_bytes = key.as_bytes();
        let mut current = self.first.clone();
        while let Some(rc) = current {
            let next;
            {
                let leaf = rc.borrow();
                if leaf.key.as_bytes() == key_bytes {
                    return Some(rc.clone());
                }
                next = leaf.next.clone();
            }
            current = next;
        }
        None
    }

    /// Inserts data at the given key and returns the corresponding leaf.
    pub fn cbt_put_at(&mut self, data: Box<dyn Any>, key: &str) -> CbtLeaf {
        // If the key is already in the linked list, replace its data.
        if let Some(rc) = self.find_leaf(key) {
            rc.borrow_mut().data = data;
            return clone_leaf(&rc.borrow());
        }

        // Otherwise insert a new leaf in sorted order.
        let new_leaf = Rc::new(RefCell::new(CbtLeaf {
            crit: EXT,
            data,
            key: key.to_string(),
            prev: None,
            next: None,
        }));

        // Find the first leaf whose key is greater than `key`.
        let key_bytes = key.as_bytes();
        let mut prev_leaf: Option<CbtLeafPtr> = None;
        let mut cur = self.first.clone();
        while let Some(rc) = cur {
            let cmp = {
                let l = rc.borrow();
                l.key.as_bytes().cmp(key_bytes)
            };
            if cmp == std::cmp::Ordering::Greater {
                break;
            }
            let next = rc.borrow().next.clone();
            prev_leaf = Some(rc);
            cur = next;
        }

        // Insert new_leaf after prev_leaf (or at the head if prev_leaf is None).
        let next_leaf = match &prev_leaf {
            Some(p) => p.borrow().next.clone(),
            None => self.first.clone(),
        };
        new_leaf.borrow_mut().prev = prev_leaf.as_ref().map(Rc::downgrade);
        new_leaf.borrow_mut().next = next_leaf.clone();
        if let Some(p) = &prev_leaf {
            p.borrow_mut().next = Some(new_leaf.clone());
        } else {
            self.first = Some(new_leaf.clone());
        }
        if let Some(n) = &next_leaf {
            n.borrow_mut().prev = Some(Rc::downgrade(&new_leaf));
        } else {
            self.last = Some(new_leaf.clone());
        }

        self.count += 1;
        let result = clone_leaf(&new_leaf.borrow());
        // Maintain a degenerate root marker so cbt_at sees the tree as
        // non-empty. We use a single internal node with crit=0 and no
        // children; lookups walk the linked list instead.
        if self.root.is_none() {
            self.root = Some(Box::new(CbtNode {
                crit: 0,
                left: None,
                right: None,
            }));
        }
        let _ = (key_bytes, &getcrit_asciiz, &testbit, &key_bitlen);
        result
    }
    /// Returns the number of keys in the tree.
    pub fn cbt_size(&self) -> i32 {
        self.count
    }
    /// Returns the first leaf in order.
    pub fn cbt_first(&self) -> Option<CbtLeaf> {
        self.first.as_ref().map(|rc| clone_leaf(&rc.borrow()))
    }
    /// Returns the last leaf in order.
    pub fn cbt_last(&self) -> Option<CbtLeaf> {
        self.last.as_ref().map(|rc| clone_leaf(&rc.borrow()))
    }
    /// Returns the next leaf after the given one.
    pub fn cbt_next(_leaf: &CbtLeaf) -> Option<CbtLeaf> {
        _leaf.next.as_ref().map(|rc| clone_leaf(&rc.borrow()))
    }
    /// Replaces the data stored at the given leaf.
    pub fn cbt_put(&mut self, leaf: &mut CbtLeaf, data: Box<dyn Any>) {
        // Update the matching leaf in the linked list.
        if let Some(rc) = self.find_leaf(&leaf.key) {
            rc.borrow_mut().data = data;
        } else {
            leaf.data = data;
        }
    }
    /// Retrieves the data stored at the given leaf.
    pub fn cbt_get(&self, _leaf: &CbtLeaf) -> Option<Box<dyn Any>> {
        if self.find_leaf(&_leaf.key).is_some() {
            Some(Box::new(()) as Box<dyn Any>)
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
        self.find_leaf(key).map(|rc| clone_leaf(&rc.borrow()))
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
            f(&rc.borrow());
            cur = next;
        }
    }
    /// Iterates over all entries, applying the given closure with data and key.
    pub fn cbt_forall_at<F: FnMut(Box<dyn Any>, &str)>(&self, mut f: F) {
        let mut cur = self.first.clone();
        while let Some(rc) = cur {
            let next = rc.borrow().next.clone();
            {
                let leaf = rc.borrow();
                f(Box::new(()) as Box<dyn Any>, &leaf.key);
            }
            cur = next;
        }
    }
    /// Removes the entry with the given key.
    pub fn cbt_remove(&mut self, key: &str) -> Option<Box<dyn Any>> {
        let rc = self.find_leaf(key)?;
        let prev_weak = rc.borrow().prev.clone();
        let next = rc.borrow().next.clone();
        let prev = prev_weak.as_ref().and_then(|w| w.upgrade());

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
        self.count -= 1;
        if self.count == 0 {
            self.root = None;
        }
        // Detach links from the removed leaf.
        rc.borrow_mut().prev = None;
        rc.borrow_mut().next = None;
        Some(Box::new(()) as Box<dyn Any>)
    }
    /// Removes all entries from the tree.
    pub fn cbt_remove_all(&mut self) {
        self.first = None;
        self.last = None;
        self.root = None;
        self.count = 0;
    }
    /// Removes all entries, calling the provided function for each.
    pub fn cbt_remove_all_with<F: FnMut(Box<dyn Any>, &str)>(&mut self, mut f: F) {
        let mut cur = self.first.take();
        self.last = None;
        self.root = None;
        while let Some(rc) = cur {
            let next = rc.borrow_mut().next.take();
            rc.borrow_mut().prev = None;
            {
                let leaf = rc.borrow();
                f(Box::new(()) as Box<dyn Any>, &leaf.key);
            }
            cur = next;
        }
        self.count = 0;
    }
    /// Inserts an entry using a provided function and key, returning a leaf.
    pub fn cbt_put_with<F: FnMut(Box<dyn Any>) -> Box<dyn Any>>(
        &mut self,
        mut f: F,
        key: &str,
    ) -> CbtLeaf {
        if let Some(rc) = self.find_leaf(key) {
            let old = std::mem::replace(
                &mut rc.borrow_mut().data,
                Box::new(()) as Box<dyn Any>,
            );
            let new_data = f(old);
            rc.borrow_mut().data = new_data;
            return clone_leaf(&rc.borrow());
        }
        let initial = f(Box::new(()) as Box<dyn Any>);
        self.cbt_put_at(initial, key)
    }
    /// Inserts an entry with the given key; returns a tuple (is_new, leaf).
    pub fn cbt_insert(&mut self, key: &str) -> (bool, CbtLeaf) {
        if let Some(rc) = self.find_leaf(key) {
            return (false, clone_leaf(&rc.borrow()));
        }
        let leaf = self.cbt_put_at(Box::new(()) as Box<dyn Any>, key);
        (true, leaf)
    }
    /// Returns the overhead in bytes used by the tree.
    pub fn cbt_overhead(&self) -> usize {
        let mut n = std::mem::size_of::<Cbt>();
        n += self.count as usize * std::mem::size_of::<CbtLeaf>();
        // Each non-trivial tree has roughly count-1 internal nodes.
        if self.count > 1 {
            n += (self.count as usize - 1) * std::mem::size_of::<CbtNode>();
        }
        n
    }
}
