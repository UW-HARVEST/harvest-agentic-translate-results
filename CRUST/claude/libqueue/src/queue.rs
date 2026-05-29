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
        let new_tail_ptr: *mut QueueNode<T> = &mut *new_node;

        if self.is_empty() {
            // Empty queue: head and tail both point to the new node
            self.head = Some(new_node);
        } else {
            // Append to the current tail's next pointer
            // SAFETY: When the queue is non-empty, `tail` was set during a
            // previous push to the address of a Box that is still owned by
            // the queue (either via `head` or transitively via `next`
            // chains). The Box owning that node is alive for the duration
            // of this dereference and the boxed node's address is stable.
            unsafe {
                let tail_ptr = self.tail.expect("non-empty queue must have tail");
                (*tail_ptr).next = Some(new_node);
            }
        }

        self.tail = Some(new_tail_ptr);
        self.size += 1;
    }
    /// Removes and returns the value at the front of the queue.
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let head_box = self.head.take()?;
        let head_node = *head_box;
        self.head = head_node.next;

        if self.size == 1 {
            // The node we just removed was also the tail; clear the dangling
            // pointer.
            self.tail = None;
        }

        self.size -= 1;
        Some(head_node.value)
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
        // SAFETY: When the queue is non-empty, `tail` points to a node still
        // owned by the queue (rooted in `head`). The borrow returned is tied
        // to `&self`, so the queue cannot be mutated while the reference
        // lives, keeping the boxed node alive at a stable address.
        self.tail.map(|tail_ptr| unsafe { &(*tail_ptr).value })
    }
    /// Frees all nodes in the queue.
    pub fn free(&mut self) {
        // Iteratively drop each node to avoid recursive Drop blowing the
        // stack on long queues.
        let mut current = self.head.take();
        while let Some(mut node) = current {
            current = node.next.take();
        }
        self.tail = None;
        self.size = 0;
    }
}
