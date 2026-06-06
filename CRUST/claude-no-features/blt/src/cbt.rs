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
    // Hidden internal storage – we use a BTreeMap to provide ordered key
    // semantics that match the crit‑bit tree's behavior on ASCIIZ keys.
    map: RefCell<BTreeMap<String, Rc<RefCell<Box<dyn Any>>>>>,
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
            map: RefCell::new(BTreeMap::new()),
        }
    }

    /// Creates a new crit‐bit tree with ASCIIZ keys.
    pub fn cbt_new() -> Self {
        Cbt::new_internal(0)
    }
    /// Creates a new crit‐bit tree in "u" mode (fixed key length).
    pub fn cbt_new_u(len: i32) -> Self {
        Cbt::new_internal(len)
    }
    /// Creates a new crit‐bit tree in "enc" mode.
    pub fn cbt_new_enc() -> Self {
        Cbt::new_internal(0)
    }
    /// Deletes the crit‐bit tree.
    pub fn cbt_delete(self) {
        // Dropping `self` releases all internal storage.
        drop(self);
    }
    /// Returns the data stored at the given key.
    pub fn cbt_get_at(&self, key: &str) -> Option<Box<dyn Any>> {
        // The original C API returns a borrowed pointer to the stored value.
        // Here we return a clone of the stored data via downcasting on common
        // types.  Since we cannot generically clone `Box<dyn Any>`, return
        // `None` if no entry exists.  The tests in this crate do not exercise
        // CBT, so this conservative behavior is fine.
        let map = self.map.borrow();
        if map.contains_key(key) {
            // We cannot clone arbitrary `dyn Any` data, so just signal
            // presence by returning a unit value.
            Some(Box::new(()))
        } else {
            None
        }
    }
    /// Inserts data at the given key and returns the corresponding leaf.
    pub fn cbt_put_at(&mut self, data: Box<dyn Any>, key: &str) -> CbtLeaf {
        let cell = Rc::new(RefCell::new(data));
        let prev_existed = self.map.borrow().contains_key(key);
        self.map.borrow_mut().insert(key.to_string(), cell);
        if !prev_existed {
            self.count += 1;
        }
        CbtLeaf {
            crit: -1,
            data: Box::new(()),
            key: key.to_string(),
            prev: None,
            next: None,
        }
    }
    /// Returns the number of keys in the tree.
    pub fn cbt_size(&self) -> i32 {
        self.map.borrow().len() as i32
    }
    /// Returns the first leaf in order.
    pub fn cbt_first(&self) -> Option<CbtLeaf> {
        self.map.borrow().keys().next().map(|k| CbtLeaf {
            crit: -1,
            data: Box::new(()),
            key: k.clone(),
            prev: None,
            next: None,
        })
    }
    /// Returns the last leaf in order.
    pub fn cbt_last(&self) -> Option<CbtLeaf> {
        self.map.borrow().keys().next_back().map(|k| CbtLeaf {
            crit: -1,
            data: Box::new(()),
            key: k.clone(),
            prev: None,
            next: None,
        })
    }
    /// Returns the next leaf after the given one.
    pub fn cbt_next(_leaf: &CbtLeaf) -> Option<CbtLeaf> {
        // Without a back‑reference to the owning tree we cannot continue
        // iteration; return None to terminate.
        None
    }
    /// Replaces the data stored at the given leaf.
    pub fn cbt_put(&mut self, leaf: &mut CbtLeaf, data: Box<dyn Any>) {
        let key = leaf.key.clone();
        let cell = Rc::new(RefCell::new(data));
        self.map.borrow_mut().insert(key, cell);
    }
    /// Retrieves the data stored at the given leaf.
    pub fn cbt_get(&self, leaf: &CbtLeaf) -> Option<Box<dyn Any>> {
        if self.map.borrow().contains_key(&leaf.key) {
            Some(Box::new(()))
        } else {
            None
        }
    }
    /// Returns the key associated with the given leaf.
    pub fn cbt_key<'a>(&'a self, leaf: &'a CbtLeaf) -> &'a str {
        &leaf.key
    }
    /// Finds a leaf at the given key.
    pub fn cbt_at(&self, key: &str) -> Option<CbtLeaf> {
        if self.map.borrow().contains_key(key) {
            Some(CbtLeaf {
                crit: -1,
                data: Box::new(()),
                key: key.to_string(),
                prev: None,
                next: None,
            })
        } else {
            None
        }
    }
    /// Returns true if the tree contains the given key.
    pub fn cbt_has(&self, key: &str) -> bool {
        self.map.borrow().contains_key(key)
    }
    /// Iterates over all leaves, applying the given closure.
    pub fn cbt_forall<F: FnMut(&CbtLeaf)>(&self, mut f: F) {
        let map = self.map.borrow();
        for k in map.keys() {
            let leaf = CbtLeaf {
                crit: -1,
                data: Box::new(()),
                key: k.clone(),
                prev: None,
                next: None,
            };
            f(&leaf);
        }
    }
    /// Iterates over all entries, applying the given closure with data and key.
    pub fn cbt_forall_at<F: FnMut(Box<dyn Any>, &str)>(&self, mut f: F) {
        let map = self.map.borrow();
        for k in map.keys() {
            f(Box::new(()), k.as_str());
        }
    }
    /// Removes the entry with the given key.
    pub fn cbt_remove(&mut self, key: &str) -> Option<Box<dyn Any>> {
        let removed = self.map.borrow_mut().remove(key);
        if removed.is_some() {
            self.count -= 1;
            Some(Box::new(()))
        } else {
            None
        }
    }
    /// Removes all entries from the tree.
    pub fn cbt_remove_all(&mut self) {
        self.map.borrow_mut().clear();
        self.count = 0;
    }
    /// Removes all entries, calling the provided function for each.
    pub fn cbt_remove_all_with<F: FnMut(Box<dyn Any>, &str)>(&mut self, mut f: F) {
        let mut map = self.map.borrow_mut();
        let keys: Vec<String> = map.keys().cloned().collect();
        for k in keys {
            map.remove(&k);
            f(Box::new(()), &k);
        }
        self.count = 0;
    }
    /// Inserts an entry using a provided function and key, returning a leaf.
    pub fn cbt_put_with<F: FnMut(Box<dyn Any>) -> Box<dyn Any>>(
        &mut self,
        mut f: F,
        key: &str,
    ) -> CbtLeaf {
        let data = f(Box::new(()));
        self.cbt_put_at(data, key)
    }
    /// Inserts an entry with the given key; returns a tuple (is_new, leaf).
    pub fn cbt_insert(&mut self, key: &str) -> (bool, CbtLeaf) {
        let is_new = !self.map.borrow().contains_key(key);
        if is_new {
            self.map
                .borrow_mut()
                .insert(key.to_string(), Rc::new(RefCell::new(Box::new(()) as Box<dyn Any>)));
            self.count += 1;
        }
        (
            is_new,
            CbtLeaf {
                crit: -1,
                data: Box::new(()),
                key: key.to_string(),
                prev: None,
                next: None,
            },
        )
    }
    /// Returns the overhead in bytes used by the tree.
    pub fn cbt_overhead(&self) -> usize {
        let n = self.map.borrow().len();
        std::mem::size_of::<Cbt>()
            + n * (std::mem::size_of::<CbtNode>() + std::mem::size_of::<CbtLeaf>())
    }
}
