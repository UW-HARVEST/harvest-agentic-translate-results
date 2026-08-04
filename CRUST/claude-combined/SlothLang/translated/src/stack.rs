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
        let old_top = self.top.take();
        let node = Box::new(ListNode {
            data: x,
            next: old_top,
        });
        self.top = Some(node);
    }

    pub fn is_empty(&self) -> bool {
        self.top.is_none()
    }

    pub fn pop(&mut self) -> Option<i32> {
        match self.top.take() {
            Some(node) => {
                let ListNode { data, next } = *node;
                self.top = next;
                Some(data)
            }
            None => None,
        }
    }

    pub fn peek(&self, pos: usize) -> Option<i32> {
        let mut cur = self.top.as_deref();
        let mut remaining = pos;
        while remaining > 0 {
            cur = cur?.next.as_deref();
            remaining -= 1;
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
        // Iteratively drop nodes to avoid stack overflow on deep stacks.
        let mut cur = self.top.take();
        while let Some(mut node) = cur {
            cur = node.next.take();
        }
        let mut cur = self.bottom.take();
        while let Some(mut node) = cur {
            cur = node.next.take();
        }
    }
}
