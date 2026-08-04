struct QueueNode<T> {
    value: T,
    next: Option<Box<QueueNode<T>>>,
}
pub struct Queue<T> {
    pub head: Option<Box<QueueNode<T>>>,
    pub tail: Option<*mut QueueNode<T>>, // Tail for efficient appending
    pub size: usize,
}
impl<T> Queue<T> {
    /// Creates a new, empty queue.
    pub fn new() -> Self {
        Queue {
            head: None,
            tail: None,
            size: 0,
        }
    }
    /// Checks if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
    /// Adds a value to the back of the queue.
    pub fn push(&mut self, value: T) {
        let mut new_node = Box::new(QueueNode { value, next: None });
        let raw: *mut QueueNode<T> = &mut *new_node;
        if self.is_empty() {
            self.head = Some(new_node);
            self.tail = Some(raw);
        } else {
            unsafe { (*self.tail.unwrap()).next = Some(new_node); }
            self.tail = Some(raw);
        }
        self.size += 1;
    }
    /// Removes and returns the value at the front of the queue.
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() { return None; }
        let old_head = self.head.take()?;
        self.head = old_head.next;
        self.size -= 1;
        if self.size == 0 { self.tail = None; }
        Some(old_head.value)
    }
    /// Returns a reference to the value at the front of the queue.
    pub fn front(&self) -> Option<&T> {
        self.head.as_ref().map(|n| &n.value)
    }
    /// Returns a reference to the value at the back of the queue.
    pub fn back(&self) -> Option<&T> {
        self.tail.map(|p| unsafe { &(*p).value })
    }
    /// Frees all nodes in the queue.
    pub fn free(&mut self) {
        while self.pop().is_some() {}
    }
}