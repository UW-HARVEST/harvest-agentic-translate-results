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
    let mut current = &self.top;
    for _ in 0..pos {
        match current {
            Some(node) => current = &node.next,
            None => return None,
        }
    }
    current.as_ref().map(|node| node.data)
}
pub fn print(&self) {
    print!("|");
    let mut current = &self.top;
    while let Some(node) = current {
        print!("{} ", node.data);
        current = &node.next;
    }
    println!();
}
}
impl Drop for Stack {
fn drop(&mut self) {
    while self.pop().is_some() {}
}
}
