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
        let mut new_node = Box::new(QueueNode {
            value,
            next: None,
        });
        let new_tail: *mut QueueNode<T> = &mut *new_node;

        if self.is_empty() {
            self.head = Some(new_node);
        } else {
            // Append to current tail's `next`. Tail must be valid.
            unsafe {
                let tail_ptr = self.tail.expect("tail must exist when not empty");
                (*tail_ptr).next = Some(new_node);
            }
        }
        self.tail = Some(new_tail);
        self.size += 1;
    }
    /// Removes and returns the value at the front of the queue.
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        let mut head_node = self.head.take()?;
        self.head = head_node.next.take();

        if self.size == 1 {
            self.tail = None;
        }
        self.size -= 1;
        Some(head_node.value)
    }
    /// Returns a reference to the value at the front of the queue.
    pub fn front(&self) -> Option<&T> {
        if self.is_empty() {
            return None;
        }
        self.head.as_ref().map(|node| &node.value)
    }
    /// Returns a reference to the value at the back of the queue.
    pub fn back(&self) -> Option<&T> {
        if self.is_empty() {
            return None;
        }
        match self.tail {
            Some(tail_ptr) => unsafe { Some(&(*tail_ptr).value) },
            None => None,
        }
    }
    /// Frees all nodes in the queue.
    pub fn free(&mut self) {
        // Iteratively drop nodes to avoid recursive deep stack drops.
        let mut current = self.head.take();
        while let Some(mut node) = current {
            current = node.next.take();
        }
        self.tail = None;
        self.size = 0;
    }
}
