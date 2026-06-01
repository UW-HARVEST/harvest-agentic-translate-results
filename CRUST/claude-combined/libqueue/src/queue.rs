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
        let new_node = Box::new(QueueNode { value, next: None });

        if self.is_empty() {
            self.head = Some(new_node);
            // Update tail to point to the head node
            let head_ref = self.head.as_mut().unwrap();
            self.tail = Some(head_ref.as_mut() as *mut QueueNode<T>);
        } else {
            // Safety: `tail` points to a node owned by `self.head`'s chain.
            // The node is alive as long as the queue is, since we never
            // remove the tail while size > 1 except via pop, which would
            // also update the tail.
            unsafe {
                let tail_ptr = self.tail.unwrap();
                let new_raw = Box::into_raw(new_node);
                (*tail_ptr).next = Some(Box::from_raw(new_raw));
                self.tail = Some(new_raw);
            }
        }

        self.size += 1;
    }
    /// Removes and returns the value at the front of the queue.
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let mut head = self.head.take()?;
        self.head = head.next.take();

        if self.size == 1 {
            self.tail = None;
        }

        self.size -= 1;

        Some(head.value)
    }
    /// Returns a reference to the value at the front of the queue.
    pub fn front(&self) -> Option<&T> {
        self.head.as_ref().map(|node| &node.value)
    }
    /// Returns a reference to the value at the back of the queue.
    pub fn back(&self) -> Option<&T> {
        if self.is_empty() {
            return None;
        }
        // Safety: `tail` points into the owned chain rooted at `self.head`.
        unsafe { self.tail.map(|ptr| &(*ptr).value) }
    }
    /// Frees all nodes in the queue.
    pub fn free(&mut self) {
        // Iteratively drop all nodes to avoid recursive Drop blowing the stack.
        while self.pop().is_some() {}
    }
}
