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
        // Re-index existing items.
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
        if self.n_items == 0 {
            return None;
        }
        let top = self.items[0].clone();
        self.n_items -= 1;
        if self.n_items > 0 {
            // Move last element to head
            let last = self.items[self.n_items].clone();
            self.items[0] = last.clone();
            // index
            self.ht.insert(last, 0);
        }
        // Pop the (now-duplicate) tail entry from items so length tracks n_items.
        self.items.pop();
        // Heapify if more than 1
        if self.n_items > 1 {
            self.heapify(0);
        }
        Some(top)
    }

    pub fn heapify(&mut self, mut i: usize) {
        if i >= self.n_items {
            return;
        }
        loop {
            let l = left(i);
            let r = right(i);

            let mut mx = if l < self.n_items && (self.cmp)(&self.items[l], &self.items[i]) == Ordering::Less {
                l
            } else {
                i
            };
            mx = if r < self.n_items && (self.cmp)(&self.items[r], &self.items[mx]) == Ordering::Less {
                r
            } else {
                mx
            };

            if mx != i {
                self.swap_items(i, mx);
                i = mx;
            } else {
                break;
            }
        }
    }

    pub fn insert(&mut self, item: T) {
        // Resize if needed (only if not limited)
        if self.limit == 0 && self.n_max == self.n_items {
            self.n_max *= HEAP_RESIZE_FACTOR;
        }

        // Push to tail
        if self.items.len() <= self.n_items {
            self.items.push(item.clone());
        } else {
            self.items[self.n_items] = item.clone();
        }

        // index
        self.ht.insert(item.clone(), self.n_items);

        if self.limit == 0 || self.n_items < self.limit {
            // Sift up
            let i = self.n_items;
            self.n_items += 1;
            self.sift_up(i);
        } else {
            // Limit reached: remove the max element to keep size at limit
            self.delete_max();
        }
    }

    pub fn update(&mut self, item: T, mut i: usize) {
        // C version: assert(cmp(item, items[i])) — only allows reducing priority
        // for min-heap. We mimic that.
        assert!(i < self.n_items);
        // Replace
        self.items[i] = item.clone();
        self.ht.insert(item, i);
        self.sift_up(i);
        // Allow `mut i` to be used (after the loop above is via sift_up, not local).
        // Suppress the "unused mut" lint by referencing i.
        let _ = i;
    }

    pub fn find(&self, item: &T) -> Option<usize> {
        self.ht.get(item).copied()
    }

    fn swap_items(&mut self, i: usize, j: usize) {
        self.items.swap(i, j);
        // Re-index
        self.ht.insert(self.items[i].clone(), i);
        self.ht.insert(self.items[j].clone(), j);
    }

    fn sift_up(&mut self, mut i: usize) {
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
        // The new item was just pushed at index n_items (n_items not yet bumped).
        // Find the max among items[0..=n_items] (mirrors C: `1..n_items+1`).
        let mut m = 0usize;
        for i in 1..=self.n_items {
            if (self.cmp)(&self.items[m], &self.items[i]) == Ordering::Less {
                m = i;
            }
        }
        if m != self.n_items {
            let old_max = self.items[m].clone();
            self.ht.remove(&old_max);
            let last = self.items[self.n_items].clone();
            self.items[m] = last.clone();
            self.ht.insert(last, m);
            self.items.pop();
            self.sift_up(m);
        } else {
            // The new (just pushed) element is itself the max → drop it.
            let item = self.items.pop().unwrap();
            self.ht.remove(&item);
        }
    }
}
