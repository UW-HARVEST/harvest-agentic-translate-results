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
        } else {
            // SAFETY: `tail` is a valid pointer as long as the queue is
            // non-empty; the node it points to is owned by the linked list
            // hanging off `head` and is not aliased mutably elsewhere.
            unsafe {
                if let Some(tail_ptr) = self.tail {
                    (*tail_ptr).next = Some(new_node);
                }
            }
        }

        self.tail = Some(raw);
        self.size += 1;
    }
    /// Removes and returns the value at the front of the queue.
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let mut old_head = self.head.take()?;
        self.head = old_head.next.take();

        if self.size == 1 {
            self.tail = None;
        }

        self.size -= 1;
        Some(old_head.value)
    }
    /// Returns a reference to the value at the front of the queue.
    pub fn front(&self) -> Option<&T> {
        self.head.as_ref().map(|n| &n.value)
    }
    /// Returns a reference to the value at the back of the queue.
    pub fn back(&self) -> Option<&T> {
        // SAFETY: `tail` points into the boxed node owned by the linked list
        // reachable from `head`. While `&self` is held, no mutable aliasing
        // can occur.
        self.tail.map(|t| unsafe { &(*t).value })
    }
    /// Frees all nodes in the queue.
    pub fn free(&mut self) {
        // Iteratively drop nodes to avoid a recursive Drop for long lists.
        let mut cur = self.head.take();
        while let Some(mut node) = cur {
            cur = node.next.take();
        }
        self.tail = None;
        self.size = 0;
    }
}