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
            // Append the new node to the current tail's `next`.
            // Safety: `self.tail` is guaranteed to point to a valid node owned
            // by this queue (reachable via `head`) whenever the queue is
            // non-empty. We hold an exclusive `&mut self` for the duration
            // of this operation, so no other reference exists.
            unsafe {
                if let Some(tail_ptr) = self.tail {
                    (*tail_ptr).next = Some(new_node);
                }
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

        let mut head_box = self.head.take()?;
        self.head = head_box.next.take();

        self.size -= 1;
        if self.size == 0 {
            self.tail = None;
        }

        Some(head_box.value)
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
        // Safety: when the queue is non-empty, `self.tail` points to a node
        // owned by the queue (reachable from `head`). The returned reference
        // is bound to `&self`, so the queue cannot be mutated while it lives.
        unsafe {
            self.tail.map(|tail_ptr| &(*tail_ptr).value)
        }
    }
    /// Frees all nodes in the queue.
    pub fn free(&mut self) {
        // Iteratively drop nodes to avoid recursive Drop blowing the stack
        // on long queues.
        let mut current = self.head.take();
        while let Some(mut node) = current {
            current = node.next.take();
        }
        self.tail = None;
        self.size = 0;
    }
}
