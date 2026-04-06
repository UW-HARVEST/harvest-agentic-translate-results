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
    Stack { top: None, bottom: None }
}
pub fn push(&mut self, x: i32) {
    let node = Box::new(ListNode { data: x, next: self.top.take() });
    self.top = Some(node);
}
pub fn is_empty(&self) -> bool {
    self.top.is_none()
}
pub fn pop(&mut self) -> Option<i32> {
    self.top.take().map(|node| {
        self.top = node.next;
        node.data
    })
}
pub fn peek(&self, pos: usize) -> Option<i32> {
    let mut cur = &self.top;
    for _ in 0..pos {
        cur = match cur {
            Some(node) => &node.next,
            None => return None,
        };
    }
    cur.as_ref().map(|node| node.data)
}
pub fn print(&self) {
    print!("|");
    let mut cur = &self.top;
    while let Some(node) = cur {
        print!("{} ", node.data);
        cur = &node.next;
    }
    println!();
}
}
impl Drop for Stack {
fn drop(&mut self) {
    let mut cur = self.top.take();
    while let Some(mut node) = cur {
        cur = node.next.take();
    }
    // bottom is a separate sentinel in C, but we don't allocate one
}
}
