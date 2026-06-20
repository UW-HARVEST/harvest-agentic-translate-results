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
    pub fn new(cmp: HeapCmp<T>, item_size: usize, init_size: usize, limit: usize) -> Self {
        let n_max = if limit != 0 {
            limit + 1
        } else if init_size == 0 {
            HEAP_INIT_SIZE
        } else {
            init_size
        };
        let _ = item_size;
        Self {
            n_items: 0,
            n_max,
            limit,
            cmp,
            ht: HashMap::new(),
            items: Vec::with_capacity(n_max),
        }
    }
    pub fn index(&mut self, hsh: fn(&T) -> u64, hcmp: fn(&T, &T) -> bool) {
        let _ = hsh;
        let _ = hcmp;
        self.ht.clear();
        for (i, item) in self.items.iter().take(self.n_items).enumerate() {
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
        self.ht.remove(&top);
        self.n_items -= 1;

        if self.n_items > 0 {
            let tail = self.items[self.n_items].clone();
            self.items[0] = tail.clone();
            self.ht.insert(tail, 0);
        }

        self.items.truncate(self.n_items);

        if self.n_items > 1 {
            self.heapify(0);
        }

        Some(top)
    }
    pub fn heapify(&mut self, i: usize) {
        let mut i = i;
        while i < self.n_items {
            let l = left(i);
            let r = right(i);

            let mut best = if l < self.n_items && (self.cmp)(&self.items[l], &self.items[i]) == Ordering::Less {
                l
            } else {
                i
            };

            if r < self.n_items && (self.cmp)(&self.items[r], &self.items[best]) == Ordering::Less {
                best = r;
            }

            if best == i {
                break;
            }

            self.swap_items(i, best);
            i = best;
        }
    }
    pub fn insert(&mut self, item: T) {
        if self.limit == 0 && self.n_items == self.n_max {
            self.n_max *= HEAP_RESIZE_FACTOR;
            self.items.reserve(self.n_max.saturating_sub(self.items.capacity()));
        }

        if self.limit == 0 || self.n_items < self.limit {
            if self.items.len() == self.n_items {
                self.items.push(item.clone());
            } else {
                self.items[self.n_items] = item.clone();
            }
            self.ht.insert(item, self.n_items);
            self.n_items += 1;
            self.update(self.items[self.n_items - 1].clone(), self.n_items - 1);
            return;
        }

        let mut max_idx = 0usize;
        for i in 1..self.n_items {
            if (self.cmp)(&self.items[max_idx], &self.items[i]) == Ordering::Less {
                max_idx = i;
            }
        }

        if (self.cmp)(&item, &self.items[max_idx]) == Ordering::Less {
            let old = self.items[max_idx].clone();
            self.ht.remove(&old);
            self.items[max_idx] = item.clone();
            self.ht.insert(item, max_idx);
            self.update(self.items[max_idx].clone(), max_idx);
        }
    }
    pub fn update(&mut self, item: T, i: usize) {
        assert!(i < self.n_items);
        self.items[i] = item.clone();
        self.ht.insert(item, i);

        let mut i = i;
        while i > 0 {
            let j = parent(i);
            if (self.cmp)(&self.items[i], &self.items[j]) != Ordering::Less {
                break;
            }
            self.swap_items(i, j);
            i = j;
        }
    }
    pub fn find(&self, item: &T) -> Option<usize> {
        self.ht.get(item).copied()
    }
    fn swap_items(&mut self, i: usize, j: usize) {
        self.items.swap(i, j);
        let left = self.items[i].clone();
        let right = self.items[j].clone();
        self.ht.insert(left, i);
        self.ht.insert(right, j);
    }
}
