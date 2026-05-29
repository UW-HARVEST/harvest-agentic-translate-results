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
            self.items[0] = self.items[self.n_items].clone();
            if !self.ht.is_empty() {
                self.ht.insert(self.items[0].clone(), 0);
            }
        }
        if self.n_items > 1 {
            self.heapify(0);
        }
        Some(top)
    }
    pub fn heapify(&mut self, mut i: usize) {
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
    fn delete_max(&mut self) {
        // find max element across whole heap
        let mut m = 0;
        for i in 1..self.n_items + 1 {
            if (self.cmp)(&self.items[m], &self.items[i]) == Ordering::Less {
                m = i;
            }
        }
        if m != self.n_items {
            self.items[m] = self.items[self.n_items].clone();
            if !self.ht.is_empty() {
                self.ht.insert(self.items[m].clone(), m);
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
            self.ht.insert(item.clone(), self.n_items);
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
        // assert that we're decreasing priority
        assert_eq!((self.cmp)(&item, &self.items[i]), Ordering::Less);
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
        if !self.ht.is_empty() {
            return self.ht.get(item).copied();
        }
        for i in 0..self.n_items {
            if &self.items[i] == item {
                return Some(i);
            }
        }
        None
    }
    fn swap_items(&mut self, i: usize, j: usize) {
        self.items.swap(i, j);
        if !self.ht.is_empty() {
            self.ht.insert(self.items[i].clone(), i);
            self.ht.insert(self.items[j].clone(), j);
        }
    }
}
