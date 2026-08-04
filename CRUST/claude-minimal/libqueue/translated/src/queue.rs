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
            // SAFETY: tail is non-null when the queue is not empty, and points
            // to a node still owned by the chain rooted at `head`.
            unsafe {
                (*self.tail.unwrap()).next = Some(new_node);
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

        let old_head = self.head.take()?;
        let node = *old_head;
        self.head = node.next;

        self.size -= 1;
        if self.size == 0 {
            self.tail = None;
        }

        Some(node.value)
    }
    /// Returns a reference to the value at the front of the queue.
    pub fn front(&self) -> Option<&T> {
        self.head.as_ref().map(|node| &node.value)
    }
    /// Returns a reference to the value at the back of the queue.
    pub fn back(&self) -> Option<&T> {
        // SAFETY: tail is non-null when the queue is not empty, and points
        // to a node still owned by the chain rooted at `head`.
        self.tail.map(|tail| unsafe { &(*tail).value })
    }
    /// Frees all nodes in the queue.
    pub fn free(&mut self) {
        while self.pop().is_some() {}
    }
}
