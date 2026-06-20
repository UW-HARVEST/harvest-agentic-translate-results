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
    ((i + 1) >> 1).saturating_sub(1)
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
        for (idx, item) in self.items.iter().take(self.n_items).cloned().enumerate() {
            self.ht.insert(item, idx);
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
            let last = self.items[self.n_items].clone();
            self.items[0] = last.clone();
            self.ht.insert(last, 0);
            self.items.pop();
        } else {
            self.items.pop();
        }

        if self.n_items > 1 {
            self.heapify(0);
        }

        Some(top)
    }

    pub fn heapify(&mut self, mut i: usize) {
        while i < self.n_items {
            let l = left(i);
            let r = right(i);

            let mut best = if l < self.n_items
                && (self.cmp)(&self.items[l], &self.items[i]) == Ordering::Less
            {
                l
            } else {
                i
            };

            if r < self.n_items
                && (self.cmp)(&self.items[r], &self.items[best]) == Ordering::Less
            {
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

        self.items.push(item.clone());
        self.ht.insert(item.clone(), self.n_items);
        self.n_items += 1;

        if self.limit == 0 || self.n_items <= self.limit {
            self.update(item, self.n_items - 1);
            return;
        }

        let mut worst = 0usize;
        for i in 1..self.n_items {
            if (self.cmp)(&self.items[worst], &self.items[i]) == Ordering::Less {
                worst = i;
            }
        }

        let removed = self.items.swap_remove(worst);
        self.ht.remove(&removed);
        self.n_items -= 1;

        if worst < self.n_items {
            let replacement = self.items[worst].clone();
            self.ht.insert(replacement.clone(), worst);
            if worst > 0 && (self.cmp)(&replacement, &self.items[parent(worst)]) == Ordering::Less {
                self.update(replacement, worst);
            } else if self.n_items > 1 {
                self.heapify(worst);
            }
        }
    }

    pub fn update(&mut self, item: T, mut i: usize) {
        if i < self.n_items {
            self.items[i] = item.clone();
            self.ht.insert(item, i);
        }

        while i > 0 {
            let p = parent(i);
            if (self.cmp)(&self.items[i], &self.items[p]) != Ordering::Less {
                break;
            }
            self.swap_items(i, p);
            i = p;
        }
    }

    pub fn find(&self, item: &T) -> Option<usize> {
        self.ht.get(item).copied()
    }

    fn swap_items(&mut self, i: usize, j: usize) {
        self.items.swap(i, j);
        let left_item = self.items[i].clone();
        let right_item = self.items[j].clone();
        self.ht.insert(left_item, i);
        self.ht.insert(right_item, j);
    }
}
