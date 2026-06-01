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
        // In the C version a sentinel node is allocated and both top and bottom
        // point to it. In safe Rust we represent the sentinel as `None` for both
        // top and bottom. An empty stack is `top.is_none()`, equivalent to
        // `top == bottom` in C.
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
        // top == bottom in C; both are None when empty in our representation.
        match (&self.top, &self.bottom) {
            (None, None) => true,
            _ => false,
        }
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
        let mut cur = self.top.as_deref();
        let mut remaining = pos;
        while remaining > 0 {
            match cur {
                Some(node) => {
                    cur = node.next.as_deref();
                    remaining -= 1;
                }
                None => return None,
            }
        }
        cur.map(|n| n.data)
    }

    pub fn print(&self) {
        print!("|");
        let mut cur = self.top.as_deref();
        while let Some(node) = cur {
            print!("{} ", node.data);
            cur = node.next.as_deref();
        }
        println!();
    }
}

impl Drop for Stack {
    fn drop(&mut self) {
        // Iteratively drop the chain to avoid potential recursion overflow for
        // very deep stacks (matches the iterative free loop in the C code).
        let mut cur = self.top.take();
        while let Some(mut node) = cur {
            cur = node.next.take();
        }
        // bottom is logically a sentinel; nothing more to free.
        let _ = self.bottom.take();
    }
}
