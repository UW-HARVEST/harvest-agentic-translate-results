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
    fn refresh_tail(&mut self) {
        let mut current = self.head.as_deref_mut();
        let mut tail = None;

        while let Some(node) = current {
            tail = Some(node as *mut QueueNode<T>);
            current = node.next.as_deref_mut();
        }

        self.tail = tail;
    }

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

        match self.head.as_mut() {
            None => {
                self.head = Some(new_node);
            }
            Some(mut current) => {
                while current.next.is_some() {
                    current = current.next.as_mut().expect("next exists");
                }
                current.next = Some(new_node);
            }
        }

        self.size += 1;
        self.refresh_tail();
    }
    /// Removes and returns the value at the front of the queue.
    pub fn pop(&mut self) -> Option<T> {
        let mut head = self.head.take()?;
        self.head = head.next.take();
        self.size -= 1;

        if self.size == 0 {
            self.tail = None;
        } else {
            self.refresh_tail();
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
