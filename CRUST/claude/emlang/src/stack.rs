use crate::data;
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
        if self.size >= self.cap {
            self.cap *= 2;
        }
        self.buf.push(data);
        self.size += 1;
    }
    pub fn pop(&mut self) -> Option<data::Data> {
        if self.size == 0 {
            return None;
        }
        self.size -= 1;
        let data = self.buf.pop()?;
        if let data::DataValue::Str(ref s) = data.value {
            self.add_popped(s.clone());
        }
        Some(data)
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
    pub fn dup(&mut self, off: usize) -> i32 {
        if off + 1 > self.size {
            return -1;
        }
        let val = self.buf[self.size - off - 1].clone();
        self.push(val);
        0
    }
    pub fn swap(&mut self, off: usize) -> i32 {
        if off + 1 > self.size {
            return -1;
        }
        let last = self.size - 1;
        let other = self.size - off - 1;
        self.buf.swap(last, other);
        0
    }
    pub fn shrink_to(&mut self, size: usize) {
        if size == self.size {
            return;
        }
        assert!(size < self.size);
        // collect popped strings before truncating
        let mut to_pop: Vec<String> = Vec::new();
        for i in size..self.size {
            if let data::DataValue::Str(ref s) = self.buf[i].value {
                to_pop.push(s.clone());
            }
        }
        for s in to_pop {
            self.add_popped(s);
        }
        self.buf.truncate(size);
        self.size = size;
    }
    pub fn clear(&mut self) {
        self.shrink_to(0);
        self.gc();
    }
    pub fn gc(&mut self) {
        if self.popped_size == 0 {
            return;
        }
        // In Rust, we don't manually free strings -- they're owned. The popped
        // entries are dropped when we clear them.
        self.popped.clear();
        self.popped_size = 0;
    }
}
