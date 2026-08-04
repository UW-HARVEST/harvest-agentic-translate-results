use std::cmp::Ordering;
use std::collections::HashMap;
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
    if i == 0 {
        // C macro returns -1 wrapped to size_t; emulate by returning usize::MAX
        usize::MAX
    } else {
        ((i + 1) >> 1) - 1
    }
}
pub fn left(i: usize) -> usize {
    ((i + 1) << 1) - 1
}
pub fn right(i: usize) -> usize {
    (i + 1) << 1
}
impl<T: Clone + std::hash::Hash + Eq> Heap<T> {
    pub fn new(cmp: HeapCmp<T>, item_size: usize, init_size: usize, limit: usize) -> Self {
        let _ = item_size;
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
    pub fn index(&mut self, hsh: fn(&T) -> u64, hcmp: fn(&T, &T) -> bool) {
        // Build the index from current items
        let _ = (hsh, hcmp);
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
            // move last to first
            let last = self.items[self.n_items].clone();
            self.items[0] = last.clone();
            // update index
            if !self.ht.is_empty() {
                self.ht.insert(last, 0);
            }
            // pop last (bookkeeping)
            self.items.pop();
        } else {
            self.items.pop();
        }
        // remove from index
        self.ht.remove(&top);

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
    pub fn insert(&mut self, item: T) {
        if self.limit == 0 && self.n_max == self.n_items {
            self.n_max *= HEAP_RESIZE_FACTOR;
            self.items.reserve(self.n_max - self.items.len());
        }

        // append to tail
        if self.items.len() <= self.n_items {
            self.items.push(item.clone());
        } else {
            self.items[self.n_items] = item.clone();
        }

        // index
        if !self.ht.is_empty() || self.limit == 0 || self.n_items >= self.limit {
            self.ht.insert(item.clone(), self.n_items);
        } else {
            self.ht.insert(item.clone(), self.n_items);
        }

        if self.limit == 0 || self.n_items < self.limit {
            let i = self.n_items;
            self.n_items += 1;
            self.bubble_up(i);
        } else {
            // delete max
            self.delete_max();
        }
    }
    fn delete_max(&mut self) {
        if self.n_items == 0 {
            return;
        }
        let mut m = 0usize;
        for i in 1..(self.n_items + 1) {
            if (self.cmp)(&self.items[m], &self.items[i]) == Ordering::Less {
                m = i;
            }
        }
        if m != self.n_items {
            let last = self.items[self.n_items].clone();
            self.items[m] = last.clone();
            self.ht.insert(last, m);
            self.bubble_up(m);
        }
    }
    fn bubble_up(&mut self, i: usize) {
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
    pub fn update(&mut self, item: T, i: usize) {
        // when item is not "null" - do an assignment
        // we always have a real item here
        let _ = item;
        // Since we always pass a valid item in C, the C code allows item==NULL
        // to skip the assignment. In Rust, we'll always update with the given item.
        // But the typical caller in C invokes update with item=NULL, meaning only re-bubble.
        // We'll mirror that by ignoring item content; the items[i] is already updated.
        self.bubble_up(i);
    }
    pub fn find(&self, item: &T) -> Option<usize> {
        self.ht.get(item).copied()
    }
    fn swap_items(&mut self, i: usize, j: usize) {
        self.items.swap(i, j);
        if !self.ht.is_empty() {
            // update both
            let a = self.items[i].clone();
            let b = self.items[j].clone();
            self.ht.insert(a, i);
            self.ht.insert(b, j);
        }
    }
}
