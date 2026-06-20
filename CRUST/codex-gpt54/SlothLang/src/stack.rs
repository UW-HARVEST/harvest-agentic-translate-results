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
Self {
top: None,
bottom: None,
}
}
pub fn push(&mut self, x: i32) {
let new_top = Box::new(ListNode {
data: x,
next: self.top.take(),
});
self.top = Some(new_top);
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
        current = current?.next.as_ref();
    }
    current.map(|node| node.data)
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
    while self.pop().is_some() {}
}
}
