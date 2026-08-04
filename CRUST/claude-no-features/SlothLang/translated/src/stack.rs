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
        match self.top.take() {
            Some(mut node) => {
                self.top = node.next.take();
                Some(node.data)
            }
            None => None,
        }
    }
    pub fn peek(&self, pos: usize) -> Option<i32> {
        let mut current = self.top.as_ref();
        for _ in 0..pos {
            current = current.and_then(|n| n.next.as_ref());
        }
        current.map(|n| n.data)
    }
    pub fn print(&self) {
        print!("|");
        let mut current = self.top.as_ref();
        while let Some(node) = current {
            print!("{} ", node.data);
            current = node.next.as_ref();
        }
        println!();
    }
}
impl Drop for Stack {
    fn drop(&mut self) {
        // Iteratively drop nodes to avoid recursive Box drop overflow on large stacks.
        let mut current = self.top.take();
        while let Some(mut node) = current {
            current = node.next.take();
        }
    }
}
