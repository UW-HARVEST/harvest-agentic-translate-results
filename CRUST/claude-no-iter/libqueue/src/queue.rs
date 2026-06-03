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

        match self.head {
            None => {
                self.head = Some(new_node);
            }
            Some(ref mut head_node) => {
                let mut current: &mut QueueNode<T> = head_node.as_mut();
                while current.next.is_some() {
                    current = current.next.as_mut().unwrap().as_mut();
                }
                current.next = Some(new_node);
            }
        }

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
        let mut current = self.head.as_deref()?;
        while let Some(next) = current.next.as_deref() {
            current = next;
        }
        Some(&current.value)
    }
    /// Frees all nodes in the queue.
    pub fn free(&mut self) {
        // Iteratively drop all nodes to avoid potential stack overflow
        // from recursive Drop on long lists.
        let mut current = self.head.take();
        while let Some(mut node) = current {
            current = node.next.take();
        }
        self.tail = None;
        self.size = 0;
    }
}
