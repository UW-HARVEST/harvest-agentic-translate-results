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
    // ((i + 1) >> 1) - 1
    // For i=0 this would be -1 in C; we use a Rust-friendly equivalent
    if i == 0 {
        // C returned -1; mimic by panicking or returning 0; here we use saturating
        // But callers should check i > 0 first as in C code
        return 0_usize.wrapping_sub(1);
    }
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
        // re-build hash table mapping items to their indices
        self.ht.clear();
        for (i, item) in self.items.iter().enumerate() {
            self.ht.insert(item.clone(), i);
        }
    }
    pub fn remove(&mut self) {
        self.items.clear();
        self.ht.clear();
        self.n_items = 0;
    }
    pub fn pop(&mut self) -> Option<T> {
        if self.n_items == 0 {
            return None;
        }
        let top = self.items[0].clone();
        self.n_items -= 1;
        if self.n_items > 0 {
            // move tail to head
            let last = self.items[self.n_items].clone();
            self.items[0] = last.clone();
            if !self.ht.is_empty() {
                self.ht.insert(last, 0);
            }
        }
        // Pop the unused last slot
        self.items.pop();
        if self.n_items > 1 {
            self.heapify(0);
        }
        Some(top)
    }
    pub fn heapify(&mut self, i: usize) {
        let mut i = i;
        if i >= self.n_items {
            return;
        }
        loop {
            let l = left(i);
            let r = right(i);
            let mut mx = i;
            if l < self.n_items && (self.cmp)(&self.items[l], &self.items[i]) == Ordering::Less {
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
    pub fn insert(&mut self, item: T) {
        if self.limit == 0 && self.n_max == self.n_items {
            self.n_max *= HEAP_RESIZE_FACTOR;
        }
        // push to tail
        if self.items.len() <= self.n_items {
            self.items.push(item.clone());
        } else {
            self.items[self.n_items] = item.clone();
        }
        if !self.ht.is_empty() {
            self.ht.insert(item.clone(), self.n_items);
        }
        if self.limit == 0 || self.n_items < self.limit {
            let i = self.n_items;
            self.n_items += 1;
            self.update_idx(i);
        } else {
            // Delete max - find max element and replace with last
            self.delete_max();
        }
    }
    fn delete_max(&mut self) {
        if self.n_items == 0 {
            return;
        }
        let mut m = 0;
        for i in 1..self.n_items + 1 {
            if i >= self.items.len() {
                break;
            }
            if (self.cmp)(&self.items[m], &self.items[i]) == Ordering::Less {
                m = i;
            }
        }
        if m != self.n_items {
            self.items[m] = self.items[self.n_items].clone();
            if !self.ht.is_empty() {
                let it = self.items[m].clone();
                self.ht.insert(it, m);
            }
            self.update_idx(m);
        }
    }
    pub fn update(&mut self, item: T, i: usize) {
        if i >= self.n_items {
            return;
        }
        // Replace item at i
        self.items[i] = item.clone();
        if !self.ht.is_empty() {
            self.ht.insert(item, i);
        }
        self.update_idx(i);
    }
    fn update_idx(&mut self, i: usize) {
        let mut i = i;
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
            let it_i = self.items[i].clone();
            let it_j = self.items[j].clone();
            self.ht.insert(it_i, i);
            self.ht.insert(it_j, j);
        }
    }
}
