/// A custom binary heap that stores elements in a Vec and uses a custom comparator.
pub struct Heap<T> {
    data: Vec<T>,
    comparator: Box<dyn Fn(&T, &T) -> bool>,
}
impl<T> Heap<T> {
    /// Creates a new heap with the given initial capacity and comparator.
    pub fn new(init_size: usize, comparator: impl Fn(&T, &T) -> bool + 'static) -> Self {
        Self {
            data: Vec::with_capacity(init_size),
            comparator: Box::new(comparator),
        }
    }
    /// Returns true if the heap is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    /// Returns a reference to the minimum element (if any).
    pub fn find_min(&self) -> Option<&T> {
        self.data.first()
    }
    /// Inserts an element into the heap.
    pub fn insert(&mut self, element: T) {
        self.data.push(element);
        // Percolate up
        let mut son = self.data.len() - 1;
        while son > 0 {
            let parent = (son - 1) / 2;
            if (self.comparator)(&self.data[son], &self.data[parent]) {
                self.data.swap(son, parent);
                son = parent;
            } else {
                break;
            }
        }
    }
    /// Removes and returns the minimum element, or None if empty.
    pub fn delete_min(&mut self) -> Option<T> {
        if self.data.is_empty() {
            return None;
        }
        let last_idx = self.data.len() - 1;
        self.data.swap(0, last_idx);
        let min = self.data.pop();
        // Percolate down
        let len = self.data.len();
        if len > 0 {
            let mut parent = 0usize;
            loop {
                let son1 = 2 * parent + 1;
                let son2 = 2 * parent + 2;
                let son = if son1 < len {
                    if son2 < len {
                        if (self.comparator)(&self.data[son1], &self.data[son2]) {
                            son1
                        } else {
                            son2
                        }
                    } else {
                        son1
                    }
                } else {
                    break;
                };
                if (self.comparator)(&self.data[son], &self.data[parent]) {
                    self.data.swap(parent, son);
                    parent = son;
                } else {
                    break;
                }
            }
        }
        min
    }
}
