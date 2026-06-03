use std::collections::HashMap;
use std::cmp::Ordering;
const HEAP_INIT_SIZE: usize = 0xff;
const HEAP_RESIZE_FACTOR: usize = 2;
pub type HeapCmp<T> = fn(&T, &T) -> Ordering;
pub struct Heap<T> {
    pub n_items: usize,
    pub n_max: usize,
    pub limit: usize,
    pub cmp: HeapCmp<T>,
    pub ht: HashMap<T, usize>,
    pub items: Vec<T>,
}
pub fn parent(i: usize) -> usize {
    ((i + 1) >> 1) - 1
}
pub fn left(i: usize) -> usize {
    ((i + 1) << 1) - 1
}
pub fn right(i: usize) -> usize {
    (i + 1) << 1
}
impl<T: Clone + std::hash::Hash + Eq> Heap<T> {
    pub fn new(cmp: HeapCmp<T>, _item_size: usize, init_size: usize, limit: usize) -> Self {
        let n_max = if limit != 0 {
            limit + 1
        } else if init_size == 0 {
            HEAP_INIT_SIZE
        } else {
            init_size
        };
        Heap {
            n_items: 0,
            n_max,
            limit,
            cmp,
            ht: HashMap::new(),
            items: Vec::with_capacity(n_max),
        }
    }
    pub fn index(&mut self, _hsh: fn(&T) -> u64, _hcmp: fn(&T, &T) -> bool) {
        // Build/refresh the index over the items.
        self.ht.clear();
        for (i, it) in self.items.iter().enumerate() {
            self.ht.insert(it.clone(), i);
        }
    }
    pub fn remove(&mut self) {
        self.items.clear();
        self.ht.clear();
    }
    pub fn pop(&mut self) -> Option<T> {
        if self.n_items == 0 {
            return None;
        }
        let top = self.items[0].clone();
        self.n_items -= 1;
        if self.n_items > 0 {
            // move tail to head
            let last = self.items.swap_remove(0);
            // swap_remove already moved tail into index 0; but only if items.len() > 1
            // We must handle index correctly: use `last` only as a sanity placeholder.
            let _ = last;
            if !self.ht.is_empty() {
                if let Some(item0) = self.items.get(0).cloned() {
                    self.ht.insert(item0, 0);
                }
            }
        } else {
            self.items.clear();
        }
        if self.n_items > 1 {
            self.heapify(0);
        }
        Some(top)
    }
    pub fn heapify(&mut self, i: usize) {
        let mut i = i;
        loop {
            let l = left(i);
            let r = right(i);
            let mut mx = i;
            if l < self.n_items && (self.cmp)(&self.items[l], &self.items[mx]) == Ordering::Less {
                mx = l;
            }
            if r < self.n_items && (self.cmp)(&self.items[r], &self.items[mx]) == Ordering::Less {
                mx = r;
            }
            if mx != i {
                self.swap_items(i, mx);
                i = mx;
            } else {
                break;
            }
        }
    }
    fn delete_max(&mut self) {
        // Linear search for the max element.
        let mut m = 0;
        for i in 1..self.n_items + 1 {
            if (self.cmp)(&self.items[m], &self.items[i]) == Ordering::Less {
                m = i;
            }
        }
        if m != self.n_items {
            self.items.swap(m, self.n_items);
            if !self.ht.is_empty() {
                let it = self.items[m].clone();
                self.ht.insert(it, m);
            }
            self.update_internal(m);
        }
    }
    pub fn insert(&mut self, item: T) {
        if self.limit == 0 && self.n_max == self.n_items {
            self.n_max *= HEAP_RESIZE_FACTOR;
        }
        if self.items.len() <= self.n_items {
            self.items.push(item.clone());
        } else {
            self.items[self.n_items] = item.clone();
        }
        if !self.ht.is_empty() {
            self.ht.insert(item, self.n_items);
        }
        if self.limit == 0 || self.n_items < self.limit {
            let i = self.n_items;
            self.n_items += 1;
            self.update_internal(i);
        } else {
            self.delete_max();
        }
    }
    pub fn update(&mut self, item: T, i: usize) {
        assert!(i < self.n_items);
        // The C version asserts that we're decreasing the priority.
        self.items[i] = item.clone();
        if !self.ht.is_empty() {
            self.ht.insert(item, i);
        }
        self.update_internal(i);
    }
    fn update_internal(&mut self, mut i: usize) {
        while i > 0 {
            let p = parent(i);
            if (self.cmp)(&self.items[i], &self.items[p]) == Ordering::Less {
                self.swap_items(i, p);
                i = p;
            } else {
                break;
            }
        }
    }
    pub fn find(&self, item: &T) -> Option<usize> {
        self.ht.get(item).copied()
    }
    fn swap_items(&mut self, i: usize, j: usize) {
        self.items.swap(i, j);
        if !self.ht.is_empty() {
            let a = self.items[i].clone();
            let b = self.items[j].clone();
            self.ht.insert(a, i);
            self.ht.insert(b, j);
        }
    }
}
