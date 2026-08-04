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
    fn new(cap: usize, popped_cap: usize) -> Self {
        Stack {
            buf: Vec::with_capacity(cap),
            cap,
            size: 0,
            popped: Vec::with_capacity(popped_cap),
            popped_cap,
            popped_size: 0,
        }
    }
    fn push(&mut self, data: data::Data) {
        if self.size >= self.cap {
            self.cap *= 2;
        }
        if self.buf.len() > self.size {
            self.buf[self.size] = data;
        } else {
            self.buf.push(data);
        }
        self.size += 1;
    }
    fn pop(&mut self) -> Option<data::Data> {
        if self.size == 0 {
            return None;
        }
        self.size -= 1;
        let d = self.buf[self.size].clone();
        if let data::DataValue::Str(ref s) = d.value {
            self.add_popped(s.clone());
        }
        Some(d)
    }
    fn dup(&mut self, off: usize) -> i32 {
        if off + 1 > self.size {
            return -1;
        }
        let item = self.buf[self.size - off - 1].clone();
        self.push(item);
        0
    }
    fn swap(&mut self, off: usize) -> i32 {
        if off + 1 > self.size {
            return -1;
        }
        let i = self.size - off - 1;
        let j = self.size - 1;
        self.buf.swap(i, j);
        0
    }
    fn shrink_to(&mut self, size: usize) {
        if size == self.size {
            return;
        }
        assert!(size < self.size);
        for i in size..self.size {
            if let data::DataValue::Str(ref s) = self.buf[i].value {
                let s = s.clone();
                self.add_popped(s);
            }
        }
        self.size = size;
    }
    fn clear(&mut self) {
        self.shrink_to(0);
        self.gc();
    }
    fn gc(&mut self) {
        if self.popped_size == 0 {
            return;
        }
        // In Rust, since strings are owned (cloned), we don't need to free pointers.
        // The popped strings are dropped automatically when we clear them.
        self.popped.clear();
        self.popped_size = 0;
    }

    fn add_popped(&mut self, s: String) {
        if self.popped_size >= self.popped_cap {
            self.popped_cap *= 2;
        }
        self.popped.push(Popped {
            str: s,
            marked: false,
        });
        self.popped_size += 1;
    }
}

// Public accessors so other modules in the crate can manipulate the stack.
impl Stack {
    pub fn make(cap: usize, popped_cap: usize) -> Self {
        Self::new(cap, popped_cap)
    }
    pub fn do_push(&mut self, data: data::Data) {
        self.push(data)
    }
    pub fn do_pop(&mut self) -> Option<data::Data> {
        self.pop()
    }
    pub fn do_dup(&mut self, off: usize) -> i32 {
        self.dup(off)
    }
    pub fn do_swap(&mut self, off: usize) -> i32 {
        self.swap(off)
    }
    pub fn do_shrink_to(&mut self, size: usize) {
        self.shrink_to(size)
    }
    pub fn do_clear(&mut self) {
        self.clear()
    }
    pub fn do_gc(&mut self) {
        self.gc()
    }
}

#[allow(dead_code)]
fn _use_utils() {
    let _ = utils::strcpy_to_heap("");
}
