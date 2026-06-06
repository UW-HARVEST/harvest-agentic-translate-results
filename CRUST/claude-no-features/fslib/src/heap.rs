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

impl<T: Ord + Clone + std::hash::Hash + Eq> Heap<T> {
    pub fn new(cmp: HeapCmp<T>, _item_size: usize, init_size: usize, limit: usize) -> Self {
        let n_max = if limit != 0 {
            limit + 1
        } else if init_size == 0 {
            HEAP_INIT_SIZE
        } else {
            init_size
        };
        Self {
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
        let item = self.items[0].clone();
        self.n_items -= 1;
        if self.n_items > 0 {
            self.items[0] = self.items[self.n_items].clone();
            // index update happens here in C - we set the moved item's idx to 0 in ht
        }
        self.items.truncate(self.n_items);
        if self.n_items > 1 {
            self.heapify(0);
        }
        Some(item)
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
        if self.n_items == 0 {
            return;
        }
        let mut m = 0usize;
        for i in 1..self.n_items {
            if (self.cmp)(&self.items[m], &self.items[i]) == Ordering::Less {
                m = i;
            }
        }
        // We are about to remove (overwrite) position m with the last element,
        // and then decrement n_items.
        let last = self.n_items - 1;
        if m != last {
            self.items[m] = self.items[last].clone();
            self.update_pos(m);
        }
        self.items.truncate(last);
        self.n_items = last;
    }
    fn update_pos(&mut self, i: usize) {
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
    pub fn insert(&mut self, item: T) {
        if self.limit == 0 && self.n_max == self.n_items {
            self.n_max *= HEAP_RESIZE_FACTOR;
        }
        if self.limit == 0 || self.n_items < self.limit {
            self.items.push(item);
            let i = self.n_items;
            self.n_items += 1;
            self.update_pos(i);
        } else {
            // Replace one element if better than max
            // The C code adds it to tail then deletes max
            self.items.push(item);
            self.n_items += 1;
            self.delete_max();
        }
    }
    pub fn update(&mut self, item: T, i: usize) {
        if i >= self.n_items {
            return;
        }
        // Remove old item from ht
        // Set new item
        self.items[i] = item;
        self.update_pos(i);
    }
    pub fn find(&self, item: &T) -> Option<usize> {
        for (i, x) in self.items.iter().enumerate() {
            if x == item {
                return Some(i);
            }
        }
        None
    }
    fn swap_items(&mut self, i: usize, j: usize) {
        self.items.swap(i, j);
    }
}
