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
        // Mirrors `sstack_new()`: in C, both `top` and `bottom` are set to a
        // freshly allocated sentinel node so that `top == bottom` indicates an
        // empty stack. In safe Rust we cannot share ownership of a single node,
        // so we model the empty stack as `top: None, bottom: None` (which are
        // equal under value equality, satisfying `is_empty`).
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
        let mut node = self.top.as_deref()?;
        for _ in 0..pos {
            node = node.next.as_deref()?;
        }
        Some(node.data)
    }
    pub fn print(&self) {
        print!("|");
        let mut node = self.top.as_deref();
        while let Some(n) = node {
            print!("{} ", n.data);
            node = n.next.as_deref();
        }
        println!();
    }
}
impl Drop for Stack {
    fn drop(&mut self) {
        // Iteratively drop the linked list to avoid the recursion that the
        // default Drop implementation would produce, which could overflow the
        // stack for deeply nested boxes.
        let mut current = self.top.take();
        while let Some(mut boxed) = current {
            current = boxed.next.take();
        }
        let mut current = self.bottom.take();
        while let Some(mut boxed) = current {
            current = boxed.next.take();
        }
    }
}
