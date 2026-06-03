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
    self.top = Some(Box::new(ListNode {
        data: x,
        next: old_top,
    }));
}
pub fn is_empty(&self) -> bool {
    self.top.is_none()
}
pub fn pop(&mut self) -> Option<i32> {
    let top = self.top.take()?;
    let ListNode { data, next } = *top;
    self.top = next;
    Some(data)
}
pub fn peek(&self, pos: usize) -> Option<i32> {
    let mut curr = self.top.as_deref()?;
    for _ in 0..pos {
        curr = curr.next.as_deref()?;
    }
    Some(curr.data)
}
pub fn print(&self) {
    print!("|");
    let mut curr = self.top.as_deref();
    while let Some(node) = curr {
        print!("{} ", node.data);
        curr = node.next.as_deref();
    }
    println!();
}
}
impl Drop for Stack {
fn drop(&mut self) {
    // Iterative drop to avoid stack overflow on long chains.
    let mut curr = self.top.take();
    while let Some(mut node) = curr {
        curr = node.next.take();
    }
}
}
