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
    // Mirrors C: a sentinel "bottom" node is allocated. Both top and bottom
    // point to that sentinel when the stack is empty.
    Stack {
        top: Some(Box::new(ListNode { data: 0, next: None })),
        bottom: None,
    }
}
pub fn push(&mut self, x: i32) {
    let old_top = self.top.take();
    self.top = Some(Box::new(ListNode { data: x, next: old_top }));
}
pub fn is_empty(&self) -> bool {
    // Empty when only the sentinel node remains, i.e. top has no `next`.
    match &self.top {
        Some(node) => node.next.is_none(),
        None => true,
    }
}
pub fn pop(&mut self) -> Option<i32> {
    if self.is_empty() {
        return None;
    }
    let mut top = self.top.take()?;
    let x = top.data;
    self.top = top.next.take();
    Some(x)
}
pub fn peek(&self, pos: usize) -> Option<i32> {
    let mut current = self.top.as_deref();
    for _ in 0..pos {
        current = current?.next.as_deref();
    }
    current.map(|n| n.data)
}
pub fn print(&self) {
    print!("|");
    let mut current = self.top.as_deref();
    while let Some(node) = current {
        if node.next.is_none() {
            // Reached sentinel/bottom
            break;
        }
        print!("{} ", node.data);
        current = node.next.as_deref();
    }
    println!();
}
}
impl Drop for Stack {
fn drop(&mut self) {
    // Iteratively unlink the list to avoid recursive Drop overflowing the
    // stack on very large stacks.
    let mut current = self.top.take();
    while let Some(mut node) = current {
        current = node.next.take();
    }
}
}
