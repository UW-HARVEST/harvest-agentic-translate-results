#[allow(unused_imports)]
use crate::{parser, throw, slothvm};

pub struct ListNode {
    pub data: i32,
    pub next: Option<Box<ListNode>>,
}

pub struct Stack {
    pub top: Option<Box<ListNode>>,
    pub bottom: Option<Box<ListNode>>,
}

impl Stack {
    pub fn new() -> Self {
        Stack {
            top: None,
            bottom: None,
        }
    }

    pub fn push(&mut self, x: i32) {
        let new_node = Box::new(ListNode {
            data: x,
            next: self.top.take(),
        });
        self.top = Some(new_node);
    }

    pub fn is_empty(&self) -> bool {
        self.top.is_none()
    }

    pub fn pop(&mut self) -> Option<i32> {
        let node = self.top.take()?;
        let ListNode { data, next } = *node;
        self.top = next;
        Some(data)
    }

    pub fn peek(&self, pos: usize) -> Option<i32> {
        let mut current = self.top.as_deref();
        let mut remaining = pos;
        while remaining > 0 {
            current = current?.next.as_deref();
            remaining -= 1;
        }
        current.map(|n| n.data)
    }

    pub fn print(&self) {
        print!("|");
        let mut current = self.top.as_deref();
        while let Some(node) = current {
            print!("{} ", node.data);
            current = node.next.as_deref();
        }
        println!();
    }
}

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Stack {
    fn drop(&mut self) {
        // Iteratively drop the linked list to prevent stack overflow on
        // long stacks (default Box<T> drop would recurse).
        let mut current = self.top.take();
        while let Some(mut node) = current {
            current = node.next.take();
        }
        let mut current = self.bottom.take();
        while let Some(mut node) = current {
            current = node.next.take();
        }
    }
}
