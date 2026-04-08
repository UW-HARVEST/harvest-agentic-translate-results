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
    ((i + 1) >> 1).wrapping_sub(1)
}
pub fn left(i: usize) -> usize {
    ((i + 1) << 1) - 1
}
pub fn right(i: usize) -> usize {
    (i + 1) << 1
}
impl<T: Ord + Clone + std::hash::Hash + Eq> Heap<T> {
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
        for i in 0..self.n_items {
            self.ht.insert(self.items[i].clone(), i);
        }
    }
    pub fn remove(&mut self) {
        self.items.clear();
        self.ht.clear();
        self.n_items = 0;
    }
    pub fn pop(&mut self) -> Option<T> {
        if self.n_items == 0 { return None; }
        let top = self.items[0].clone();
        self.n_items -= 1;
        if self.n_items > 0 {
            self.items[0] = self.items[self.n_items].clone();
            self.ht.insert(self.items[0].clone(), 0);
        }
        self.items.truncate(self.n_items);
        if self.n_items > 1 { self.heapify(0); }
        Some(top)
    }
    pub fn heapify(&mut self, mut i: usize) {
        loop {
            let l = left(i);
            let r = right(i);
            let mut mx = if l < self.n_items && (self.cmp)(&self.items[l], &self.items[i]) == Ordering::Less {
                l
            } else {
                i
            };
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
        if self.n_items >= self.items.len() {
            self.items.push(item.clone());
        } else {
            self.items[self.n_items] = item.clone();
        }
        self.ht.insert(item, self.n_items);
        if self.limit == 0 || self.n_items < self.limit {
            let idx = self.n_items;
            self.n_items += 1;
            self.bubble_up(idx);
        } else {
            self.delete_max();
        }
    }
    pub fn update(&mut self, item: T, i: usize) {
        self.items[i] = item.clone();
        self.ht.insert(item, i);
        self.bubble_up(i);
    }
    pub fn find(&self, item: &T) -> Option<usize> {
        self.ht.get(item).copied()
    }
    fn swap_items(&mut self, i: usize, j: usize) {
        self.items.swap(i, j);
        let ki = self.items[i].clone();
        let kj = self.items[j].clone();
        self.ht.insert(ki, i);
        self.ht.insert(kj, j);
    }
    fn bubble_up(&mut self, mut i: usize) {
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
    fn delete_max(&mut self) {
        let mut m = 0usize;
        for i in 1..=self.n_items {
            if (self.cmp)(&self.items[m], &self.items[i]) == Ordering::Less {
                m = i;
            }
        }
        if m != self.n_items {
            self.items[m] = self.items[self.n_items].clone();
            self.ht.insert(self.items[m].clone(), m);
            let idx = m;
            self.bubble_up(idx);
            self.heapify(idx);
        }
    }
}
