pub struct Vector<T>{
    pub capacity: usize,
    pub size: usize,
    pub data: Vec<T>,
}
impl<T> Vector<T>{
    pub fn new(capacity: usize) -> Self {
        Vector {
            capacity,
            size: 0,
            data: Vec::with_capacity(capacity),
        }
    }
    pub fn release(&mut self) {
        self.data.clear();
        self.size = 0;
    }
    pub fn size(&self) -> usize {
        self.size
    }
    pub fn push_back(&mut self, value: T) {
        self.insert_at_index(value, self.size);
    }
    pub fn get_element_at(&self, index: usize) -> Option<&T> {
        if index < self.size { Some(&self.data[index]) } else { None }
    }
    pub fn pop_back(&mut self) -> Option<T> {
        if self.size == 0 { return None; }
        self.remove_at_index(self.size - 1)
    }
    pub fn push_front(&mut self, value: T) {
        self.insert_at_index(value, 0);
    }
    pub fn pop_front(&mut self) -> Option<T> {
        if self.size == 0 { return None; }
        self.remove_at_index(0)
    }
    pub fn top_front(&self) -> Option<&T> {
        if self.size == 0 { None } else { Some(&self.data[0]) }
    }
    pub fn top_back(&self) -> Option<&T> {
        if self.size == 0 { None } else { Some(&self.data[self.size - 1]) }
    }
    pub fn insert_at_index(&mut self, value: T, index: usize) {
        assert!(index <= self.size);
        if self.size == self.capacity {
            self.capacity *= 2;
        }
        self.data.insert(index, value);
        self.size += 1;
    }
    pub fn remove_at_index(&mut self, index: usize) -> Option<T> {
        if index >= self.size { return None; }
        let elem = self.data.remove(index);
        self.size -= 1;
        Some(elem)
    }
    pub fn index_of(&self, value: &T) -> Option<usize> {
        self.index_of_with_start(value, 0)
    }
    pub fn index_of_with_start(&self, value: &T, start: usize) -> Option<usize> {
        // Pointer-based comparison to match C behavior (comparing addresses)
        // Fall back: since Rust doesn't do pointer comparison on references the same way,
        // we use std::ptr::eq for the generic case. But the signature takes &T,
        // so we compare by pointer identity.
        for i in start..self.size {
            if std::ptr::eq(&self.data[i], value) {
                return Some(i);
            }
        }
        None
    }
}
