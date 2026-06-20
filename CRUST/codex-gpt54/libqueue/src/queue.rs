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

        let mut link = &mut self.head;
        while let Some(node) = link {
            link = &mut node.next;
        }
        *link = Some(new_node);
        if let Some(last) = link.as_mut() {
            self.tail = Some(last.as_mut() as *mut QueueNode<T>);
        }

        self.size += 1;
    }
    /// Removes and returns the value at the front of the queue.
    pub fn pop(&mut self) -> Option<T> {
        let mut head = self.head.take()?;
        self.head = head.next.take();
        self.size -= 1;

        if self.size == 0 {
            self.tail = None;
        } else if let Some(current) = self.head.as_deref() {
            let mut last = current;
            while let Some(next) = last.next.as_deref() {
                last = next;
            }
            self.tail = Some((last as *const QueueNode<T>) as *mut QueueNode<T>);
        }

        Some(head.value)
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
        self.head = None;
        self.tail = None;
        self.size = 0;
    }
}
