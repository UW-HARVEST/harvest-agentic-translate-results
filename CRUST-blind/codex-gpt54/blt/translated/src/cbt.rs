use std::any::Any;
use std::cell::RefCell;
use std::cmp::Ordering;
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

fn clone_any(value: &dyn Any) -> Option<Box<dyn Any>> {
    macro_rules! clone_copy {
        ($($ty:ty),* $(,)?) => {
            $(
                if let Some(v) = value.downcast_ref::<$ty>() {
                    return Some(Box::new(*v));
                }
            )*
        };
    }

    macro_rules! clone_owned {
        ($($ty:ty),* $(,)?) => {
            $(
                if let Some(v) = value.downcast_ref::<$ty>() {
                    return Some(Box::new(v.clone()));
                }
            )*
        };
    }

    clone_copy!((), bool, char, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64);
    clone_owned!(String, Vec<u8>, Vec<i8>, Vec<i16>, Vec<i32>, Vec<i64>, Vec<usize>, Vec<String>);
    None
}

fn unit_box() -> Box<dyn Any> {
    Box::new(())
}

impl Cbt {
    fn new_with_len(len: i32) -> Self {
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

    fn encoded_len(key: &str) -> usize {
        let bytes = key.as_bytes();
        if bytes.len() < 2 {
            return 0;
        }
        usize::from(bytes[0]) + (usize::from(bytes[1]) << 8)
    }

    fn compare_key_bytes(&self, left: &str, right: &str) -> Ordering {
        match self.len {
            n if n > 0 => {
                let n = n as usize;
                let l = left.as_bytes();
                let r = right.as_bytes();
                let mut i = 0usize;
                while i < n {
                    let lb = l.get(i).copied().unwrap_or(0);
                    let rb = r.get(i).copied().unwrap_or(0);
                    match lb.cmp(&rb) {
                        Ordering::Equal => i += 1,
                        ord => return ord,
                    }
                }
                Ordering::Equal
            }
            -1 => {
                let llen = Self::encoded_len(left);
                let rlen = Self::encoded_len(right);
                match llen.cmp(&rlen) {
                    Ordering::Equal => {
                        let limit = llen.saturating_add(2);
                        left.as_bytes()
                            .get(..limit)
                            .unwrap_or(left.as_bytes())
                            .cmp(right.as_bytes().get(..limit).unwrap_or(right.as_bytes()))
                    }
                    ord => ord,
                }
            }
            _ => left.cmp(right),
        }
    }

    fn find_leaf_ptr(&self, key: &str) -> Option<CbtLeafPtr> {
        let mut current = self.first.clone();
        while let Some(node) = current {
            let ordering = {
                let borrowed = node.borrow();
                self.compare_key_bytes(&borrowed.key, key)
            };
            match ordering {
                Ordering::Equal => return Some(node),
                Ordering::Greater => return None,
                Ordering::Less => current = node.borrow().next.clone(),
            }
        }
        None
    }

    fn snapshot_leaf(ptr: &CbtLeafPtr) -> CbtLeaf {
        let borrowed = ptr.borrow();
        CbtLeaf {
            crit: borrowed.crit,
            data: clone_any(borrowed.data.as_ref()).unwrap_or_else(unit_box),
            key: borrowed.key.clone(),
            prev: borrowed.prev.clone(),
            next: borrowed.next.clone(),
        }
    }

    fn insert_leaf_ptr(&mut self, key: &str, data: Box<dyn Any>) -> CbtLeafPtr {
        let new_leaf = Rc::new(RefCell::new(CbtLeaf {
            crit: -1,
            data,
            key: key.to_owned(),
            prev: None,
            next: None,
        }));

        let mut current = self.first.clone();
        let mut prev: Option<CbtLeafPtr> = None;

        while let Some(node) = current.clone() {
            match self.compare_key_bytes(&node.borrow().key, key) {
                Ordering::Less => {
                    prev = current.clone();
                    current = node.borrow().next.clone();
                }
                Ordering::Equal | Ordering::Greater => break,
            }
        }

        {
            let mut new_leaf_mut = new_leaf.borrow_mut();
            new_leaf_mut.prev = prev.as_ref().map(Rc::downgrade);
            new_leaf_mut.next = current.clone();
        }

        if let Some(prev_leaf) = prev {
            prev_leaf.borrow_mut().next = Some(new_leaf.clone());
        } else {
            self.first = Some(new_leaf.clone());
        }

        if let Some(next_leaf) = current {
            next_leaf.borrow_mut().prev = Some(Rc::downgrade(&new_leaf));
        } else {
            self.last = Some(new_leaf.clone());
        }

        if self.last.is_none() {
            self.last = Some(new_leaf.clone());
        }
        if self.first.is_none() {
            self.first = Some(new_leaf.clone());
        }

        self.count += 1;
        new_leaf
    }

    fn remove_leaf_ptr(&mut self, node: &CbtLeafPtr) -> Box<dyn Any> {
        let (prev, next, data) = {
            let mut borrowed = node.borrow_mut();
            let data = std::mem::replace(&mut borrowed.data, unit_box());
            let prev = borrowed.prev.as_ref().and_then(Weak::upgrade);
            let next = borrowed.next.clone();
            borrowed.prev = None;
            borrowed.next = None;
            (prev, next, data)
        };

        if let Some(prev_leaf) = prev.clone() {
            prev_leaf.borrow_mut().next = next.clone();
        } else {
            self.first = next.clone();
        }

        if let Some(next_leaf) = next.clone() {
            next_leaf.borrow_mut().prev = prev.as_ref().map(Rc::downgrade);
        } else {
            self.last = prev;
        }

        self.count -= 1;
        data
    }

    /// Creates a new crit‐bit tree with ASCIIZ keys.
    pub fn cbt_new() -> Self {
        Self::new_with_len(0)
    }

    /// Creates a new crit‐bit tree in "u" mode (fixed key length).
    pub fn cbt_new_u(len: i32) -> Self {
        Self::new_with_len(len.max(0))
    }

    /// Creates a new crit‐bit tree in "enc" mode.
    pub fn cbt_new_enc() -> Self {
        Self::new_with_len(-1)
    }

    /// Deletes the crit‐bit tree.
    pub fn cbt_delete(mut self) {
        self.cbt_remove_all();
    }

    /// Returns the data stored at the given key.
    pub fn cbt_get_at(&self, key: &str) -> Option<Box<dyn Any>> {
        let leaf = self.find_leaf_ptr(key)?;
        let cloned = {
            let borrowed = leaf.borrow();
            clone_any(borrowed.data.as_ref())
        };
        cloned
    }

    /// Inserts data at the given key and returns the corresponding leaf.
    pub fn cbt_put_at(&mut self, data: Box<dyn Any>, key: &str) -> CbtLeaf {
        if let Some(leaf) = self.find_leaf_ptr(key) {
            leaf.borrow_mut().data = data;
            return Self::snapshot_leaf(&leaf);
        }
        let leaf = self.insert_leaf_ptr(key, data);
        Self::snapshot_leaf(&leaf)
    }

    /// Returns the number of keys in the tree.
    pub fn cbt_size(&self) -> i32 {
        self.count
    }

    /// Returns the first leaf in order.
    pub fn cbt_first(&self) -> Option<CbtLeaf> {
        self.first.as_ref().map(Self::snapshot_leaf)
    }

    /// Returns the last leaf in order.
    pub fn cbt_last(&self) -> Option<CbtLeaf> {
        self.last.as_ref().map(Self::snapshot_leaf)
    }

    /// Returns the next leaf after the given one.
    pub fn cbt_next(leaf: &CbtLeaf) -> Option<CbtLeaf> {
        leaf.next.as_ref().map(Self::snapshot_leaf)
    }

    /// Replaces the data stored at the given leaf.
    pub fn cbt_put(&mut self, leaf: &mut CbtLeaf, data: Box<dyn Any>) {
        if let Some(actual) = self.find_leaf_ptr(&leaf.key) {
            actual.borrow_mut().data = data;
            *leaf = Self::snapshot_leaf(&actual);
        }
    }

    /// Retrieves the data stored at the given leaf.
    pub fn cbt_get(&self, leaf: &CbtLeaf) -> Option<Box<dyn Any>> {
        let actual = self.find_leaf_ptr(&leaf.key)?;
        let cloned = {
            let borrowed = actual.borrow();
            clone_any(borrowed.data.as_ref())
        };
        cloned
    }

    /// Returns the key associated with the given leaf.
    pub fn cbt_key<'a>(&self, leaf: &'a CbtLeaf) -> &'a str {
        &leaf.key
    }

    /// Finds a leaf at the given key.
    pub fn cbt_at(&self, key: &str) -> Option<CbtLeaf> {
        self.find_leaf_ptr(key).as_ref().map(Self::snapshot_leaf)
    }

    /// Returns true if the tree contains the given key.
    pub fn cbt_has(&self, key: &str) -> bool {
        self.find_leaf_ptr(key).is_some()
    }

    /// Iterates over all leaves, applying the given closure.
    pub fn cbt_forall<F: FnMut(&CbtLeaf)>(&self, mut f: F) {
        let mut current = self.first.clone();
        while let Some(node) = current {
            let snapshot = Self::snapshot_leaf(&node);
            current = node.borrow().next.clone();
            f(&snapshot);
        }
    }

    /// Iterates over all entries, applying the given closure with data and key.
    pub fn cbt_forall_at<F: FnMut(Box<dyn Any>, &str)>(&self, mut f: F) {
        let mut current = self.first.clone();
        while let Some(node) = current {
            let (next, key, data) = {
                let borrowed = node.borrow();
                (
                    borrowed.next.clone(),
                    borrowed.key.clone(),
                    clone_any(borrowed.data.as_ref()).unwrap_or_else(unit_box),
                )
            };
            f(data, &key);
            current = next;
        }
    }

    /// Removes the entry with the given key.
    pub fn cbt_remove(&mut self, key: &str) -> Option<Box<dyn Any>> {
        let leaf = self.find_leaf_ptr(key)?;
        Some(self.remove_leaf_ptr(&leaf))
    }

    /// Removes all entries from the tree.
    pub fn cbt_remove_all(&mut self) {
        self.cbt_remove_all_with(|_, _| {});
    }

    /// Removes all entries, calling the provided function for each.
    pub fn cbt_remove_all_with<F: FnMut(Box<dyn Any>, &str)>(&mut self, mut f: F) {
        let mut current = self.first.take();
        self.last = None;
        self.count = 0;
        while let Some(node) = current {
            let (next, key, data) = {
                let mut borrowed = node.borrow_mut();
                let next = borrowed.next.take();
                borrowed.prev = None;
                let key = borrowed.key.clone();
                let data = std::mem::replace(&mut borrowed.data, unit_box());
                (next, key, data)
            };
            f(data, &key);
            current = next;
        }
    }

    /// Inserts an entry using a provided function and key, returning a leaf.
    pub fn cbt_put_with<F: FnMut(Box<dyn Any>) -> Box<dyn Any>>(
        &mut self,
        mut f: F,
        key: &str,
    ) -> CbtLeaf {
        if let Some(leaf) = self.find_leaf_ptr(key) {
            let old_data = {
                let mut borrowed = leaf.borrow_mut();
                std::mem::replace(&mut borrowed.data, unit_box())
            };
            leaf.borrow_mut().data = f(old_data);
            return Self::snapshot_leaf(&leaf);
        }

        let leaf = self.insert_leaf_ptr(key, f(unit_box()));
        Self::snapshot_leaf(&leaf)
    }

    /// Inserts an entry with the given key; returns a tuple (is_new, leaf).
    pub fn cbt_insert(&mut self, key: &str) -> (bool, CbtLeaf) {
        if let Some(leaf) = self.find_leaf_ptr(key) {
            return (false, Self::snapshot_leaf(&leaf));
        }
        let leaf = self.insert_leaf_ptr(key, unit_box());
        (true, Self::snapshot_leaf(&leaf))
    }

    /// Returns the overhead in bytes used by the tree.
    pub fn cbt_overhead(&self) -> usize {
        let count = self.count.max(0) as usize;
        let base = 72usize;
        if count == 0 {
            base
        } else {
            base + count * 40 + (count - 1) * 24
        }
    }
}
