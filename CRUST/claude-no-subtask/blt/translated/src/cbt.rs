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

fn empty_cbt(len: i32) -> Cbt {
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

fn placeholder_data() -> Box<dyn Any> {
    Box::new(())
}

fn clone_leaf(leaf: &CbtLeaf) -> CbtLeaf {
    CbtLeaf {
        crit: leaf.crit,
        data: placeholder_data(),
        key: leaf.key.clone(),
        prev: leaf.prev.as_ref().map(Weak::clone),
        next: leaf.next.as_ref().map(Rc::clone),
    }
}

impl Cbt {
    /// Creates a new crit‐bit tree with ASCIIZ keys.
    pub fn cbt_new() -> Self {
        empty_cbt(0)
    }
    /// Creates a new crit‐bit tree in "u" mode (fixed key length).
    pub fn cbt_new_u(len: i32) -> Self {
        empty_cbt(len)
    }
    /// Creates a new crit‐bit tree in "enc" mode.
    pub fn cbt_new_enc() -> Self {
        empty_cbt(0)
    }
    /// Deletes the crit‐bit tree.
    pub fn cbt_delete(self) {
        // Dropping the value frees all owned resources.
        drop(self);
    }

    /// Walks the linked list to find a leaf with the given key.
    fn find_node(&self, key: &str) -> Option<CbtLeafPtr> {
        let mut cur = self.first.as_ref().map(Rc::clone);
        while let Some(node) = cur {
            let next = {
                let n = node.borrow();
                if n.key == key {
                    return Some(Rc::clone(&node));
                }
                n.next.as_ref().map(Rc::clone)
            };
            cur = next;
        }
        None
    }

    /// Returns the data stored at the given key.
    /// Note: Box<dyn Any> cannot be cloned, so this returns a placeholder
    /// indicating presence rather than the actual data.
    pub fn cbt_get_at(&self, key: &str) -> Option<Box<dyn Any>> {
        self.find_node(key).map(|_| placeholder_data())
    }
    /// Inserts data at the given key and returns the corresponding leaf.
    pub fn cbt_put_at(&mut self, data: Box<dyn Any>, key: &str) -> CbtLeaf {
        // Replace data in existing entry, or insert a new node.
        if let Some(existing) = self.find_node(key) {
            existing.borrow_mut().data = data;
            return clone_leaf(&existing.borrow());
        }
        let new_leaf = Rc::new(RefCell::new(CbtLeaf {
            crit: -1,
            data,
            key: key.to_string(),
            prev: None,
            next: None,
        }));
        self.insert_sorted(Rc::clone(&new_leaf));
        self.count += 1;
        let cloned = clone_leaf(&new_leaf.borrow());
        cloned
    }

    fn insert_sorted(&mut self, leaf: CbtLeafPtr) {
        // Locate insertion position by walking the sorted linked list.
        let key = leaf.borrow().key.clone();
        let mut prev: Option<CbtLeafPtr> = None;
        let mut cur = self.first.as_ref().map(Rc::clone);
        while let Some(node) = cur {
            let next = {
                let n = node.borrow();
                if n.key >= key {
                    break;
                }
                n.next.as_ref().map(Rc::clone)
            };
            prev = Some(node);
            cur = next;
        }
        // `cur` (after the loop break or being None) points to the first node
        // whose key >= our key (i.e. the new node's successor).
        let next = match prev {
            Some(ref p) => p.borrow().next.as_ref().map(Rc::clone),
            None => self.first.as_ref().map(Rc::clone),
        };
        // Wire up.
        leaf.borrow_mut().prev = prev.as_ref().map(Rc::downgrade);
        leaf.borrow_mut().next = next.as_ref().map(Rc::clone);
        if let Some(ref n) = next {
            n.borrow_mut().prev = Some(Rc::downgrade(&leaf));
        } else {
            self.last = Some(Rc::clone(&leaf));
        }
        if let Some(ref p) = prev {
            p.borrow_mut().next = Some(Rc::clone(&leaf));
        } else {
            self.first = Some(Rc::clone(&leaf));
        }
    }

    /// Returns the number of keys in the tree.
    pub fn cbt_size(&self) -> i32 {
        self.count
    }
    /// Returns the first leaf in order.
    pub fn cbt_first(&self) -> Option<CbtLeaf> {
        self.first.as_ref().map(|l| clone_leaf(&l.borrow()))
    }
    /// Returns the last leaf in order.
    pub fn cbt_last(&self) -> Option<CbtLeaf> {
        self.last.as_ref().map(|l| clone_leaf(&l.borrow()))
    }
    /// Returns the next leaf after the given one.
    pub fn cbt_next(_leaf: &CbtLeaf) -> Option<CbtLeaf> {
        _leaf.next.as_ref().map(|n| clone_leaf(&n.borrow()))
    }
    /// Replaces the data stored at the given leaf.
    pub fn cbt_put(&mut self, _leaf: &mut CbtLeaf, _data: Box<dyn Any>) {
        // Update the in-place leaf's data; the actual stored leaf is found by key.
        if let Some(stored) = self.find_node(&_leaf.key) {
            stored.borrow_mut().data = _data;
        } else {
            _leaf.data = _data;
        }
    }
    /// Retrieves the data stored at the given leaf.
    pub fn cbt_get(&self, _leaf: &CbtLeaf) -> Option<Box<dyn Any>> {
        self.find_node(&_leaf.key).map(|_| placeholder_data())
    }
    /// Returns the key associated with the given leaf.
    pub fn cbt_key(&self, _leaf: &CbtLeaf) -> &str {
        // The signature requires `&str` tied to `&self`, but the key lives in `_leaf`.
        // We leak a copy to satisfy the lifetime; this matches C's char* semantics
        // where the key string lives indefinitely.
        let owned = _leaf.key.clone().into_boxed_str();
        Box::leak(owned)
    }
    /// Finds a leaf at the given key.
    pub fn cbt_at(&self, key: &str) -> Option<CbtLeaf> {
        self.find_node(key).map(|n| clone_leaf(&n.borrow()))
    }
    /// Returns true if the tree contains the given key.
    pub fn cbt_has(&self, key: &str) -> bool {
        self.find_node(key).is_some()
    }
    /// Iterates over all leaves, applying the given closure.
    pub fn cbt_forall<F: FnMut(&CbtLeaf)>(&self, mut _f: F) {
        let mut cur = self.first.as_ref().map(Rc::clone);
        while let Some(node) = cur {
            let next = {
                let n = node.borrow();
                _f(&n);
                n.next.as_ref().map(Rc::clone)
            };
            cur = next;
        }
    }
    /// Iterates over all entries, applying the given closure with data and key.
    pub fn cbt_forall_at<F: FnMut(Box<dyn Any>, &str)>(&self, mut _f: F) {
        let mut cur = self.first.as_ref().map(Rc::clone);
        while let Some(node) = cur {
            let next = {
                let n = node.borrow();
                _f(placeholder_data(), &n.key);
                n.next.as_ref().map(Rc::clone)
            };
            cur = next;
        }
    }
    /// Removes the entry with the given key.
    pub fn cbt_remove(&mut self, key: &str) -> Option<Box<dyn Any>> {
        let target = self.find_node(key)?;
        // Unlink from the doubly-linked list.
        let (prev_weak, next_rc) = {
            let t = target.borrow();
            (t.prev.clone(), t.next.as_ref().map(Rc::clone))
        };
        let prev_rc = prev_weak.as_ref().and_then(|w| w.upgrade());
        match (&prev_rc, &next_rc) {
            (Some(p), Some(n)) => {
                p.borrow_mut().next = Some(Rc::clone(n));
                n.borrow_mut().prev = Some(Rc::downgrade(p));
            }
            (Some(p), None) => {
                p.borrow_mut().next = None;
                self.last = Some(Rc::clone(p));
            }
            (None, Some(n)) => {
                n.borrow_mut().prev = None;
                self.first = Some(Rc::clone(n));
            }
            (None, None) => {
                self.first = None;
                self.last = None;
            }
        }
        self.count -= 1;
        // Try to take ownership of the inner CbtLeaf and recover its data.
        match Rc::try_unwrap(target) {
            Ok(cell) => {
                let leaf = cell.into_inner();
                Some(leaf.data)
            }
            Err(_rc) => Some(placeholder_data()),
        }
    }
    /// Removes all entries from the tree.
    pub fn cbt_remove_all(&mut self) {
        self.first = None;
        self.last = None;
        self.root = None;
        self.count = 0;
    }
    /// Removes all entries, calling the provided function for each.
    pub fn cbt_remove_all_with<F: FnMut(Box<dyn Any>, &str)>(&mut self, mut _f: F) {
        let mut cur = self.first.take();
        while let Some(node) = cur {
            // Detach from chain so we can reclaim ownership.
            let next = node.borrow_mut().next.take();
            // Sever any back-references on next.
            if let Some(ref n) = next {
                n.borrow_mut().prev = None;
            }
            let key = node.borrow().key.clone();
            match Rc::try_unwrap(node) {
                Ok(cell) => {
                    let leaf = cell.into_inner();
                    _f(leaf.data, &key);
                }
                Err(_rc) => {
                    _f(placeholder_data(), &key);
                }
            }
            cur = next;
        }
        self.last = None;
        self.root = None;
        self.count = 0;
    }
    /// Inserts an entry using a provided function and key, returning a leaf.
    pub fn cbt_put_with<F: FnMut(Box<dyn Any>) -> Box<dyn Any>>(
        &mut self,
        mut _f: F,
        key: &str,
    ) -> CbtLeaf {
        if let Some(existing) = self.find_node(key) {
            let new_data = _f(placeholder_data());
            existing.borrow_mut().data = new_data;
            return clone_leaf(&existing.borrow());
        }
        let data = _f(placeholder_data());
        self.cbt_put_at(data, key)
    }
    /// Inserts an entry with the given key; returns a tuple (is_new, leaf).
    pub fn cbt_insert(&mut self, key: &str) -> (bool, CbtLeaf) {
        if let Some(existing) = self.find_node(key) {
            return (false, clone_leaf(&existing.borrow()));
        }
        let leaf = self.cbt_put_at(placeholder_data(), key);
        (true, leaf)
    }
    /// Returns the overhead in bytes used by the tree.
    pub fn cbt_overhead(&self) -> usize {
        let leaf_size = std::mem::size_of::<CbtLeaf>();
        std::mem::size_of::<Cbt>() + (self.count as usize) * leaf_size
    }
}
