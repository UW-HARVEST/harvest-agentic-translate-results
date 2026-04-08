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
        if self.popped_size >= self.popped.len() {
            self.popped.push(Popped { str: s, marked: false });
        } else {
            self.popped[self.popped_size] = Popped { str: s, marked: false };
        }
        self.popped_size += 1;
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
        if let data::DataValue::Str(ref s) = data.value {
            self.add_popped(s.clone());
        }
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
        for i in size..self.size {
            if let data::DataValue::Str(ref s) = self.buf[i].value {
                self.add_popped(s.clone());
            }
        }
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
        // The C code has a bug where inner loop uses same variable name 'i' as outer loop,
        // causing it to compare buf[i] with popped[i] using the inner index.
        // We replicate that behavior: for each stack item that is a string,
        // iterate popped and compare buf[inner_i].as.str with popped[inner_i].str
        for i in 0..self.size {
            if let data::DataValue::Str(_) = self.buf[i].value {
                for j in 0..self.popped_size {
                    // In C, the inner loop shadows 'i', so buf[i] uses inner 'i' = j
                    if let data::DataValue::Str(ref buf_s) = self.buf[j].value {
                        if j < self.popped_size && buf_s == &self.popped[j].str {
                            self.popped[j].marked = true;
                        }
                    }
                }
            }
        }
        // In Rust we don't actually need to free strings, but we clear the popped list
        self.popped_size = 0;
    }
}
