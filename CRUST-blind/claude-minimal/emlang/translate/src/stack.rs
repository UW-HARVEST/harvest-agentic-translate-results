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

    pub fn push(&mut self, d: data::Data) {
        if self.size >= self.cap {
            self.cap *= 2;
        }
        self.buf.push(d);
        self.size += 1;
    }

    pub fn pop(&mut self) -> Option<data::Data> {
        if self.size == 0 {
            return None;
        }
        self.size -= 1;
        let d = self.buf.pop().unwrap();
        if let data::DataValue::Str(ref s) = d.value {
            let s_clone = s.clone();
            self.add_popped(s_clone);
        }
        Some(d)
    }

    pub fn dup(&mut self, off: usize) -> i32 {
        if off + 1 > self.size {
            return -1;
        }
        let item = self.buf[self.size - off - 1].clone();
        self.push(item);
        0
    }

    pub fn swap(&mut self, off: usize) -> i32 {
        if off + 1 > self.size {
            return -1;
        }
        let a = self.size - off - 1;
        let b = self.size - 1;
        self.buf.swap(a, b);
        0
    }

    pub fn shrink_to(&mut self, size: usize) {
        if size == self.size {
            return;
        }
        assert!(size < self.size);

        // Collect string data for popped tracking
        let mut to_add: Vec<String> = Vec::new();
        for i in size..self.size {
            if let data::DataValue::Str(ref s) = self.buf[i].value {
                to_add.push(s.clone());
            }
        }
        for s in to_add {
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
        // In Rust, `popped` strings are owned; just drop them all.
        self.popped.clear();
        self.popped_size = 0;
    }
}
