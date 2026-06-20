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
Self { top: None, bottom: None }
}
pub fn push(&mut self, x: i32) {
let next = self.top.take();
self.top = Some(Box::new(ListNode { data: x, next }));
self.refresh_bottom();
}
pub fn is_empty(&self) -> bool {
    self.top.is_none()
}
pub fn pop(&mut self) -> Option<i32> {
let mut top = self.top.take()?;
self.top = top.next.take();
let data = top.data;
self.refresh_bottom();
Some(data)
}
pub fn peek(&self, pos: usize) -> Option<i32> {
    let mut current = self.top.as_deref();
    for _ in 0..pos {
        current = current?.next.as_deref();
    }
    current.map(|node| node.data)
}
pub fn print(&self) {
    print!("|");
    let mut current = self.top.as_deref();
    while let Some(node) = current {
        print!("{} ", node.data);
        current = node.next.as_deref();
    }
    println!();
}

fn refresh_bottom(&mut self) {
    let mut current = self.top.as_deref();
    let mut last = None;
    while let Some(node) = current {
        last = Some(node.data);
        current = node.next.as_deref();
    }
    self.bottom = last.map(|data| Box::new(ListNode { data, next: None }));
}
}
impl Drop for Stack {
fn drop(&mut self) {
    while let Some(mut node) = self.top.take() {
        self.top = node.next.take();
    }
    self.bottom = None;
}
}
