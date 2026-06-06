use crate::{parser, throw, slothvm};

#[allow(dead_code)]
fn _unused_imports() {
    let _ = parser::parse;
    let _ = throw::math_err;
    let _ = slothvm::execute;
}

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
        let node = Box::new(ListNode {
            data: x,
            next: self.top.take(),
        });
        self.top = Some(node);
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
        // Iteratively drop nodes to avoid stack overflow on long stacks.
        let mut cur = self.top.take();
        while let Some(mut node) = cur {
            cur = node.next.take();
        }
    }
}
