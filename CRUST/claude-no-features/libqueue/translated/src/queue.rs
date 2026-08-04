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
        // Get a stable raw pointer to the heap-allocated node before moving the Box.
        let new_tail: *mut QueueNode<T> = &mut *new_node;

        if self.is_empty() {
            self.head = Some(new_node);
        } else {
            // SAFETY: `tail` always points to a live node owned by the queue
            // (reachable from `head`) when the queue is non-empty.
            let tail_ptr = self.tail.expect("non-empty queue must have a tail");
            unsafe {
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
        self.size -= 1;
        if self.size == 0 {
            self.tail = None;
        }
        Some(head_node.value)
    }
    /// Returns a reference to the value at the front of the queue.
    pub fn front(&self) -> Option<&T> {
        self.head.as_ref().map(|node| &node.value)
    }
    /// Returns a reference to the value at the back of the queue.
    pub fn back(&self) -> Option<&T> {
        // SAFETY: When the queue is non-empty, `tail` points to a live node
        // owned by the queue (reachable from `head`).
        self.tail.map(|tail_ptr| unsafe { &(*tail_ptr).value })
    }
    /// Frees all nodes in the queue.
    pub fn free(&mut self) {
        // Iteratively drop nodes to avoid recursive destructor stack overflow
        // on long queues. Mirrors the C implementation's loop.
        let mut current = self.head.take();
        while let Some(mut node) = current {
            current = node.next.take();
        }
        self.tail = None;
        self.size = 0;
    }
}
