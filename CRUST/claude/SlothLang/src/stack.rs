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
            Some(node) => {
                self.top = node.next;
                Some(node.data)
            }
            None => None,
        }
    }

    pub fn peek(&self, pos: usize) -> Option<i32> {
        let mut cur = self.top.as_ref();
        let mut remaining = pos;
        while remaining > 0 {
            match cur {
                Some(node) => {
                    cur = node.next.as_ref();
                    remaining -= 1;
                }
                None => return None,
            }
        }
        cur.map(|n| n.data)
    }

    pub fn print(&self) {
        print!("|");
        let mut cur = self.top.as_ref();
        while let Some(node) = cur {
            print!("{} ", node.data);
            cur = node.next.as_ref();
        }
        println!();
    }
}

impl Drop for Stack {
    fn drop(&mut self) {
        // Iteratively drop to avoid potential stack overflow on recursive Drop
        let mut cur = self.top.take();
        while let Some(mut node) = cur {
            cur = node.next.take();
        }
    }
}
