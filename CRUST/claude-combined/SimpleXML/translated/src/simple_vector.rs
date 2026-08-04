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
        self.capacity = 0;
    }
    pub fn size(&self) -> usize {
        self.size
    }
    pub fn push_back(&mut self, value: T) {
        self.data.push(value);
        self.size += 1;
        if self.size > self.capacity {
            self.capacity = self.data.capacity();
        }
    }
    pub fn get_element_at(&self, index: usize) -> Option<&T> {
        self.data.get(index)
    }
    pub fn pop_back(&mut self) -> Option<T> {
        if self.size == 0 {
            return None;
        }
        let v = self.data.pop();
        if v.is_some() {
            self.size -= 1;
        }
        v
    }
    pub fn push_front(&mut self, value: T) {
        self.data.insert(0, value);
        self.size += 1;
        if self.size > self.capacity {
            self.capacity = self.data.capacity();
        }
    }
    pub fn pop_front(&mut self) -> Option<T> {
        if self.size == 0 {
            return None;
        }
        let v = self.data.remove(0);
        self.size -= 1;
        Some(v)
    }
    pub fn top_front(&self) -> Option<&T> {
        self.data.first()
    }
    pub fn top_back(&self) -> Option<&T> {
        self.data.last()
    }
    pub fn insert_at_index(&mut self, value: T, index: usize) {
        self.data.insert(index, value);
        self.size += 1;
        if self.size > self.capacity {
            self.capacity = self.data.capacity();
        }
    }
    pub fn remove_at_index(&mut self, index: usize) -> Option<T> {
        if index >= self.size {
            return None;
        }
        let v = self.data.remove(index);
        self.size -= 1;
        Some(v)
    }
    pub fn index_of(&self, value: &T) -> Option<usize> {
        self.index_of_with_start(value, 0)
    }
    pub fn index_of_with_start(&self, value: &T, start: usize) -> Option<usize> {
        for i in start..self.size {
            if std::ptr::eq(&self.data[i], value) {
                return Some(i);
            }
        }
        None
    }
}
