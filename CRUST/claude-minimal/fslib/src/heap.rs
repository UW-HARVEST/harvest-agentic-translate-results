use std::cmp::Ordering;
const HEAP_INIT_SIZE: usize = 0xff;
const HEAP_RESIZE_FACTOR: usize = 2;
pub type HeapCmp<T> = fn(&T, &T) -> Ordering;
pub struct Heap<T> {
    pub n_items: usize,
    pub n_max: usize,
    pub limit: usize,
    pub cmp: HeapCmp<T>,
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
impl<T: Clone + PartialEq> Heap<T> {
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
            items: Vec::with_capacity(n_max),
        }
    }
    pub fn index(&mut self, _hsh: fn(&T) -> u64, _hcmp: fn(&T, &T) -> bool) {
        // We do not maintain the index in this Rust implementation; find() is linear.
    }
    pub fn remove(&mut self) {
        self.items.clear();
    }
    pub fn pop(&mut self) -> Option<T> {
        if self.n_items == 0 {
            return None;
        }
        let top = self.items[0].clone();
        self.n_items -= 1;
        if self.n_items > 0 {
            self.items[0] = self.items[self.n_items].clone();
        }
        // pop the last
        self.items.truncate(self.n_items);
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
            if l < self.n_items && (self.cmp)(&self.items[l], &self.items[mx]) == Ordering::Less {
                mx = l;
            }
            if r < self.n_items && (self.cmp)(&self.items[r], &self.items[mx]) == Ordering::Less {
                mx = r;
            }
            if mx != i {
                self.items.swap(i, mx);
                i = mx;
            } else {
                break;
            }
        }
    }
    fn delete_max(&mut self) {
        // Find max via linear search
        if self.n_items == 0 {
            return;
        }
        let mut m: usize = 0;
        for i in 1..(self.n_items + 1).min(self.items.len()) {
            if (self.cmp)(&self.items[m], &self.items[i]) == Ordering::Less {
                m = i;
            }
        }
        if m != self.n_items {
            // Replace with last
            self.items[m] = self.items[self.n_items].clone();
            self.update_at(m);
        }
        // Drop last item
        self.items.truncate(self.n_items);
    }
    pub fn insert(&mut self, item: T) {
        if self.limit == 0 && self.n_max == self.n_items {
            self.n_max *= HEAP_RESIZE_FACTOR;
        }
        if self.items.len() <= self.n_items {
            self.items.push(item);
        } else {
            self.items[self.n_items] = item;
        }
        if self.limit == 0 || self.n_items < self.limit {
            let i = self.n_items;
            self.n_items += 1;
            self.update_at(i);
        } else {
            // Above limit; remove max
            self.delete_max();
        }
    }
    pub fn update(&mut self, item: T, i: usize) {
        assert!(i < self.n_items);
        // For min heap, only allow lowering priority value
        // C asserts: cmp(item, items[i])
        // i.e., item < items[i] in min-heap.
        self.items[i] = item;
        self.update_at(i);
    }
    fn update_at(&mut self, mut i: usize) {
        while i > 0 {
            let p = parent(i);
            if (self.cmp)(&self.items[i], &self.items[p]) == Ordering::Less {
                self.items.swap(i, p);
                i = p;
            } else {
                break;
            }
        }
    }
    pub fn find(&self, item: &T) -> Option<usize> {
        for i in 0..self.n_items {
            if &self.items[i] == item {
                return Some(i);
            }
        }
        None
    }
}
