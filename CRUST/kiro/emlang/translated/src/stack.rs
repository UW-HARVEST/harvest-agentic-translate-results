use crate::{data, utils};
pub const DEFAULT_POPPED_CAP: usize = 32;
pub const DEFAULT_STACK_CAP: usize = 1024;
#[derive(Debug)]
pub struct Popped {
    pub str: String,
    pub marked: bool,
}
#[derive(Debug)]
pub struct Stack {
    pub buf: Vec<data::Data>,
    pub cap: usize,
    pub size: usize,
    pub popped: Vec<Popped>,
    pub popped_cap: usize,
    pub popped_size: usize,
}
impl Stack {
    pub fn new(cap: usize, popped_cap: usize) -> Self {
        Stack {
            buf: Vec::with_capacity(cap),
            cap,
            size: 0,
            popped: Vec::with_capacity(popped_cap),
            popped_cap,
            popped_size: 0,
        }
    }
    pub fn push(&mut self, data: data::Data) {
        if self.size >= self.buf.len() {
            self.buf.push(data);
        } else {
            self.buf[self.size] = data;
        }
        self.size += 1;
    }
    pub fn pop(&mut self) -> Option<data::Data> {
        if self.size == 0 {
            return None;
        }
        self.size -= 1;
        let data = self.buf[self.size].clone();
        Some(data)
    }
    pub fn dup(&mut self, off: usize) -> i32 {
        if off + 1 > self.size {
            return -1;
        }
        let d = self.buf[self.size - off - 1].clone();
        self.push(d);
        0
    }
    pub fn swap(&mut self, off: usize) -> i32 {
        if off + 1 > self.size {
            return -1;
        }
        let idx = self.size - off - 1;
        let top = self.size - 1;
        self.buf.swap(idx, top);
        0
    }
    pub fn shrink_to(&mut self, size: usize) {
        if size == self.size {
            return;
        }
        self.size = size;
    }
    pub fn clear(&mut self) {
        self.shrink_to(0);
        self.gc();
    }
    pub fn gc(&mut self) {
        // In Rust with owned strings, GC is a no-op since Drop handles cleanup
    }
}
