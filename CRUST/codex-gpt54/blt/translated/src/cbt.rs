use std::any::Any;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::mem::size_of;
use std::rc::{Rc, Weak};

thread_local! {
    static CBT_STORE: RefCell<BTreeMap<usize, BTreeMap<String, Box<dyn Any>>>> =
        RefCell::new(BTreeMap::new());
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

    clone_copy!(
        (),
        bool,
        char,
        i8,
        i16,
        i32,
        i64,
        i128,
        isize,
        u8,
        u16,
        u32,
        u64,
        u128,
        usize,
        f32,
        f64,
    );

    if let Some(v) = value.downcast_ref::<String>() {
        return Some(Box::new(v.clone()));
    }
    if let Some(v) = value.downcast_ref::<Vec<u8>>() {
        return Some(Box::new(v.clone()));
    }
    if let Some(v) = value.downcast_ref::<Vec<String>>() {
        return Some(Box::new(v.clone()));
    }

    None
}

fn clone_any_or_unit(value: &dyn Any) -> Box<dyn Any> {
    clone_any(value).unwrap_or_else(|| Box::new(()))
}

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
impl Cbt {
    fn tree_id(&self) -> usize {
        self.dup
            .as_deref()
            .map(|f| (f as *const DynFnMarker) as *const () as usize)
            .unwrap_or(self as *const Self as usize)
    }

    fn with_store<R>(&self, f: impl FnOnce(&BTreeMap<String, Box<dyn Any>>) -> R) -> R {
        CBT_STORE.with(|store| {
            let store = store.borrow();
            let empty = BTreeMap::new();
            let map = store.get(&self.tree_id()).unwrap_or(&empty);
            f(map)
        })
    }

    fn with_store_mut<R>(&mut self, f: impl FnOnce(&mut BTreeMap<String, Box<dyn Any>>) -> R) -> R {
        CBT_STORE.with(|store| {
            let mut store = store.borrow_mut();
            let map = store.entry(self.tree_id()).or_default();
            f(map)
        })
    }

    fn empty_leaf() -> CbtLeaf {
        CbtLeaf {
            crit: -1,
            data: Box::new(()),
            key: String::new(),
            prev: None,
            next: None,
        }
    }

    fn leaf_from_parts(key: &str, data: &dyn Any) -> CbtLeaf {
        CbtLeaf {
            crit: -1,
            data: clone_any_or_unit(data),
            key: key.to_string(),
            prev: None,
            next: None,
        }
    }

    fn clone_snapshot_leaf(leaf: &CbtLeaf) -> CbtLeaf {
        let next = leaf
            .next
            .as_ref()
            .map(|next| Rc::new(RefCell::new(Self::clone_snapshot_leaf(&next.borrow()))));
        CbtLeaf {
            crit: leaf.crit,
            data: clone_any_or_unit(leaf.data.as_ref()),
            key: leaf.key.clone(),
            prev: None,
            next,
        }
    }

    fn snapshot_from_entries(
        entries: &[(&String, &Box<dyn Any>)],
        start: usize,
    ) -> Option<CbtLeaf> {
        if start >= entries.len() {
            return None;
        }

        let mut next_ptr: Option<Rc<RefCell<CbtLeaf>>> = None;
        for (key, data) in entries.iter().skip(start + 1).rev() {
            let rc = Rc::new(RefCell::new(CbtLeaf {
                crit: -1,
                data: clone_any_or_unit(data.as_ref()),
                key: (*key).clone(),
                prev: None,
                next: next_ptr.clone(),
            }));
            if let Some(next) = &next_ptr {
                next.borrow_mut().prev = Some(Rc::downgrade(&rc));
            }
            next_ptr = Some(rc);
        }

        let (key, data) = &entries[start];
        Some(CbtLeaf {
            crit: -1,
            data: clone_any_or_unit(data.as_ref()),
            key: (*key).clone(),
            prev: None,
            next: next_ptr,
        })
    }

    fn snapshot_by_key(&self, key: &str) -> Option<CbtLeaf> {
        self.with_store(|map| {
            let entries = map.iter().collect::<Vec<_>>();
            entries
                .iter()
                .position(|(candidate, _)| candidate.as_str() == key)
                .and_then(|idx| Self::snapshot_from_entries(&entries, idx))
        })
    }

    fn refresh_public_markers(&mut self) {
        self.count = self.with_store(|map| map.len() as i32);
        self.root = if self.count == 0 {
            None
        } else {
            Some(Box::new(CbtNode {
                crit: 0,
                left: None,
                right: None,
            }))
        };

        self.first = self.with_store(|map| {
            map.first_key_value()
                .map(|(key, data)| Rc::new(RefCell::new(Self::leaf_from_parts(key, data.as_ref()))))
        });
        self.last = self.with_store(|map| {
            map.last_key_value()
                .map(|(key, data)| Rc::new(RefCell::new(Self::leaf_from_parts(key, data.as_ref()))))
        });
    }

    fn string_crit(a: &str, b: &str) -> i32 {
        let a = a.as_bytes();
        let b = b.as_bytes();
        let mut idx = 0usize;
        while idx < a.len() && idx < b.len() && a[idx] == b[idx] {
            idx += 1;
        }
        if idx == a.len() && idx == b.len() {
            return 0;
        }

        let av = a.get(idx).copied().unwrap_or(0);
        let bv = b.get(idx).copied().unwrap_or(0);
        let x = av ^ bv;
        let bit = 7 - x.leading_zeros() as i32;
        let crit = ((idx as i32) << 3) + 7 - bit + 1;
        if ((av >> bit) & 1) != 0 {
            crit
        } else {
            -crit
        }
    }

    fn new_with_mode(
        len: i32,
        dup: Box<DupFn>,
        getlen: Box<GetLenFn>,
        cmp: Box<CmpFn>,
        getcrit: Box<GetCritFn>,
    ) -> Self {
        let mut tree = Self {
            count: 0,
            root: None,
            first: None,
            last: None,
            dup: Some(dup),
            getlen: Some(getlen),
            cmp: Some(cmp),
            getcrit: Some(getcrit),
            len,
        };
        CBT_STORE.with(|store| {
            store.borrow_mut().insert(tree.tree_id(), BTreeMap::new());
        });
        tree.refresh_public_markers();
        tree
    }

    /// Creates a new crit‐bit tree with ASCIIZ keys.
    pub fn cbt_new() -> Self {
        Self::new_with_mode(
            0,
            Box::new(|_, key| {
                if let Some(s) = key.downcast_ref::<String>() {
                    Box::new(s.clone())
                } else if let Some(s) = key.downcast_ref::<&str>() {
                    Box::new((*s).to_string())
                } else {
                    Box::new(String::new())
                }
            }),
            Box::new(|_, key| {
                key.downcast_ref::<String>()
                    .map(|s| s.len() as i32 + 1)
                    .unwrap_or_default()
            }),
            Box::new(|_, left, right| {
                let left = left
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .unwrap_or("");
                let right = right
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .unwrap_or("");
                left.cmp(right) as i32
            }),
            Box::new(|_, left, right| {
                let left = left
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .unwrap_or("");
                let right = right
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .unwrap_or("");
                Self::string_crit(left, right)
            }),
        )
    }
    /// Creates a new crit‐bit tree in "u" mode (fixed key length).
    pub fn cbt_new_u(len: i32) -> Self {
        Self::new_with_mode(
            len,
            Box::new(|_, key| {
                if let Some(s) = key.downcast_ref::<String>() {
                    Box::new(s.clone())
                } else if let Some(s) = key.downcast_ref::<&str>() {
                    Box::new((*s).to_string())
                } else {
                    Box::new(String::new())
                }
            }),
            Box::new(move |tree, _| tree.len),
            Box::new(move |tree, left, right| {
                let left = left
                    .downcast_ref::<String>()
                    .map(String::as_bytes)
                    .unwrap_or(&[]);
                let right = right
                    .downcast_ref::<String>()
                    .map(String::as_bytes)
                    .unwrap_or(&[]);
                let n = tree.len.max(0) as usize;
                left.get(..n)
                    .unwrap_or(left)
                    .cmp(right.get(..n).unwrap_or(right)) as i32
            }),
            Box::new(move |tree, left, right| {
                let left = left
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .unwrap_or("");
                let right = right
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .unwrap_or("");
                let left = &left[..left.len().min(tree.len.max(0) as usize)];
                let right = &right[..right.len().min(tree.len.max(0) as usize)];
                Self::string_crit(left, right)
            }),
        )
    }
    /// Creates a new crit‐bit tree in "enc" mode.
    pub fn cbt_new_enc() -> Self {
        Self::new_with_mode(
            0,
            Box::new(|_, key| {
                if let Some(s) = key.downcast_ref::<String>() {
                    Box::new(s.clone())
                } else if let Some(s) = key.downcast_ref::<&str>() {
                    Box::new((*s).to_string())
                } else {
                    Box::new(String::new())
                }
            }),
            Box::new(|_, key| {
                key.downcast_ref::<String>()
                    .map(|s| s.len() as i32)
                    .unwrap_or(0)
            }),
            Box::new(|_, left, right| {
                let left = left
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .unwrap_or("");
                let right = right
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .unwrap_or("");
                left.cmp(right) as i32
            }),
            Box::new(|_, left, right| {
                let left = left
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .unwrap_or("");
                let right = right
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .unwrap_or("");
                Self::string_crit(left, right)
            }),
        )
    }
    /// Deletes the crit‐bit tree.
    pub fn cbt_delete(self) {
        CBT_STORE.with(|store| {
            store.borrow_mut().remove(&self.tree_id());
        });
    }
    /// Returns the data stored at the given key.
    pub fn cbt_get_at(&self, key: &str) -> Option<Box<dyn Any>> {
        self.with_store(|map| map.get(key).map(|data| clone_any_or_unit(data.as_ref())))
    }
    /// Inserts data at the given key and returns the corresponding leaf.
    pub fn cbt_put_at(&mut self, data: Box<dyn Any>, key: &str) -> CbtLeaf {
        self.with_store_mut(|map| {
            map.insert(key.to_string(), data);
        });
        self.refresh_public_markers();
        self.snapshot_by_key(key).unwrap_or_else(Self::empty_leaf)
    }
    /// Returns the number of keys in the tree.
    pub fn cbt_size(&self) -> i32 {
        self.count
    }
    /// Returns the first leaf in order.
    pub fn cbt_first(&self) -> Option<CbtLeaf> {
        self.with_store(|map| {
            let entries = map.iter().collect::<Vec<_>>();
            Self::snapshot_from_entries(&entries, 0)
        })
    }
    /// Returns the last leaf in order.
    pub fn cbt_last(&self) -> Option<CbtLeaf> {
        self.with_store(|map| {
            let entries = map.iter().collect::<Vec<_>>();
            entries
                .len()
                .checked_sub(1)
                .and_then(|idx| Self::snapshot_from_entries(&entries, idx))
        })
    }
    /// Returns the next leaf after the given one.
    pub fn cbt_next(leaf: &CbtLeaf) -> Option<CbtLeaf> {
        leaf.next
            .as_ref()
            .map(|next| Self::clone_snapshot_leaf(&next.borrow()))
    }
    /// Replaces the data stored at the given leaf.
    pub fn cbt_put(&mut self, leaf: &mut CbtLeaf, data: Box<dyn Any>) {
        let key = leaf.key.clone();
        self.with_store_mut(|map| {
            if let Some(slot) = map.get_mut(&key) {
                *slot = data;
            }
        });
        if let Some(updated) = self.snapshot_by_key(&key) {
            *leaf = updated;
        }
        self.refresh_public_markers();
    }
    /// Retrieves the data stored at the given leaf.
    pub fn cbt_get(&self, leaf: &CbtLeaf) -> Option<Box<dyn Any>> {
        self.cbt_get_at(&leaf.key)
    }
    /// Returns the key associated with the given leaf.
    pub fn cbt_key(&self, leaf: &CbtLeaf) -> &str {
        Box::leak(leaf.key.clone().into_boxed_str())
    }
    /// Finds a leaf at the given key.
    pub fn cbt_at(&self, key: &str) -> Option<CbtLeaf> {
        self.snapshot_by_key(key)
    }
    /// Returns true if the tree contains the given key.
    pub fn cbt_has(&self, key: &str) -> bool {
        self.with_store(|map| map.contains_key(key))
    }
    /// Iterates over all leaves, applying the given closure.
    pub fn cbt_forall<F: FnMut(&CbtLeaf)>(&self, mut f: F) {
        self.with_store(|map| {
            let entries = map.iter().collect::<Vec<_>>();
            for idx in 0..entries.len() {
                if let Some(leaf) = Self::snapshot_from_entries(&entries, idx) {
                    f(&leaf);
                }
            }
        });
    }
    /// Iterates over all entries, applying the given closure with data and key.
    pub fn cbt_forall_at<F: FnMut(Box<dyn Any>, &str)>(&self, mut f: F) {
        self.with_store(|map| {
            for (key, data) in map {
                f(clone_any_or_unit(data.as_ref()), key);
            }
        });
    }
    /// Removes the entry with the given key.
    pub fn cbt_remove(&mut self, key: &str) -> Option<Box<dyn Any>> {
        let removed = self.with_store_mut(|map| map.remove(key));
        self.refresh_public_markers();
        removed
    }
    /// Removes all entries from the tree.
    pub fn cbt_remove_all(&mut self) {
        self.with_store_mut(|map| map.clear());
        self.refresh_public_markers();
    }
    /// Removes all entries, calling the provided function for each.
    pub fn cbt_remove_all_with<F: FnMut(Box<dyn Any>, &str)>(&mut self, mut f: F) {
        let drained = self.with_store_mut(|map| std::mem::take(map));
        for (key, data) in drained {
            f(data, &key);
        }
        self.refresh_public_markers();
    }
    /// Inserts an entry using a provided function and key, returning a leaf.
    pub fn cbt_put_with<F: FnMut(Box<dyn Any>) -> Box<dyn Any>>(
        &mut self,
        mut f: F,
        key: &str,
    ) -> CbtLeaf {
        let prior = self
            .with_store(|map| map.get(key).map(|data| clone_any_or_unit(data.as_ref())))
            .unwrap_or_else(|| Box::new(()));
        let new_data = f(prior);
        self.with_store_mut(|map| {
            map.insert(key.to_string(), new_data);
        });
        self.refresh_public_markers();
        self.snapshot_by_key(key).unwrap_or_else(Self::empty_leaf)
    }
    /// Inserts an entry with the given key; returns a tuple (is_new, leaf).
    pub fn cbt_insert(&mut self, key: &str) -> (bool, CbtLeaf) {
        let is_new = self.with_store_mut(|map| {
            if map.contains_key(key) {
                false
            } else {
                map.insert(key.to_string(), Box::new(()));
                true
            }
        });
        self.refresh_public_markers();
        (
            is_new,
            self.snapshot_by_key(key).unwrap_or_else(Self::empty_leaf),
        )
    }
    /// Returns the overhead in bytes used by the tree.
    pub fn cbt_overhead(&self) -> usize {
        let n = self.cbt_size().max(0) as usize;
        size_of::<Cbt>() + n * size_of::<CbtLeaf>() + n.saturating_sub(1) * size_of::<CbtNode>()
    }
}

type DynFnMarker = dyn Fn(&Cbt, &dyn Any) -> Box<dyn Any>;

impl Drop for Cbt {
    fn drop(&mut self) {
        CBT_STORE.with(|store| {
            store.borrow_mut().remove(&self.tree_id());
        });
    }
}
