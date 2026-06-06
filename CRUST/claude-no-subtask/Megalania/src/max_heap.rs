pub struct MaxHeap {
    capacity: usize,
    data: Vec<u32>,
    comparator: Box<dyn Fn(u32, u32) -> i32>,
}
impl MaxHeap {
    pub fn new(capacity: usize, comparator: Box<dyn Fn(u32, u32) -> i32>) -> Self {
        Self {
            capacity,
            data: Vec::with_capacity(capacity),
            comparator,
        }
    }
    pub fn count(&self) -> usize {
        self.data.len()
    }
    pub fn clear(&mut self) {
        self.data.clear();
    }
    fn heapify_up(&mut self, mut idx: usize) {
        while idx > 0 {
            let parent = (idx - 1) / 2;
            if (self.comparator)(self.data[idx], self.data[parent]) > 0 {
                self.data.swap(idx, parent);
                idx = parent;
            } else {
                break;
            }
        }
    }
    fn heapify_down(&mut self, mut idx: usize) {
        let len = self.data.len();
        loop {
            let left = idx * 2 + 1;
            if left >= len {
                break;
            }
            let right = idx * 2 + 2;
            let mut largest_child = left;
            if right < len {
                if (self.comparator)(self.data[right], self.data[left]) > 0 {
                    largest_child = right;
                }
            }
            if (self.comparator)(self.data[largest_child], self.data[idx]) > 0 {
                self.data.swap(largest_child, idx);
                idx = largest_child;
            } else {
                break;
            }
        }
    }
    pub fn insert(&mut self, value: u32) -> bool {
        if self.data.len() == self.capacity {
            return false;
        }
        let new_node = self.data.len();
        self.data.push(value);
        self.heapify_up(new_node);
        true
    }
    pub fn maximum(&self) -> Option<u32> {
        self.data.first().copied()
    }
    pub fn remove_maximum(&mut self) -> bool {
        if self.data.is_empty() {
            return false;
        }
        let last = self.data.pop().unwrap();
        if !self.data.is_empty() {
            self.data[0] = last;
            self.heapify_down(0);
        }
        true
    }
    /// Assumes that the element at index 0 may have changed.
    pub fn update_maximum(&mut self) -> bool {
        if self.data.is_empty() {
            return false;
        }
        self.heapify_down(0);
        true
    }
}
