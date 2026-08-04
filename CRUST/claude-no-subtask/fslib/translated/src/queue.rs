use std::collections::VecDeque;
pub struct Queue<T> {
    pub items: VecDeque<T>,
}
impl<T> Queue<T> {
    pub fn new() -> Self {
        Queue {
            items: VecDeque::new(),
        }
    }
    pub fn remove(self) {
        // drop self
    }
    pub fn empty(&mut self) {
        self.items.clear();
    }
    pub fn enqueue(&mut self, item: T) {
        self.items.push_back(item);
    }
    pub fn dequeue(&mut self) -> Option<T> {
        self.items.pop_front()
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
}
