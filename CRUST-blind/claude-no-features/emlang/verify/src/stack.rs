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
    pub(crate) fn new(cap: usize, popped_cap: usize) -> Self {
        Stack {
            buf: Vec::with_capacity(cap),
            cap,
            size: 0,
            popped: Vec::with_capacity(popped_cap),
            popped_cap,
            popped_size: 0,
        }
    }
    pub(crate) fn push(&mut self, data: data::Data) {
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
    pub(crate) fn pop(&mut self) -> Option<data::Data> {
        if self.size == 0 {
            return None;
        }
        self.size -= 1;
        let data = self.buf[self.size].clone();
        if let data::DataValue::Str(s) = &data.value {
            self.add_popped(s.clone());
        }
        Some(data)
    }
    pub(crate) fn dup(&mut self, off: usize) -> i32 {
        if off + 1 > self.size {
            return -1;
        }
        let item = self.buf[self.size - off - 1].clone();
        self.push(item);
        0
    }
    pub(crate) fn swap(&mut self, off: usize) -> i32 {
        if off + 1 > self.size {
            return -1;
        }
        let i1 = self.size - off - 1;
        let i2 = self.size - 1;
        self.buf.swap(i1, i2);
        0
    }
    pub(crate) fn shrink_to(&mut self, size: usize) {
        if size == self.size {
            return;
        }
        assert!(size < self.size);
        for i in size..self.size {
            if let data::DataValue::Str(s) = &self.buf[i].value {
                let s = s.clone();
                self.add_popped(s);
            }
        }
        self.size = size;
    }
    pub(crate) fn clear(&mut self) {
        self.shrink_to(0);
        self.gc();
    }
    pub(crate) fn gc(&mut self) {
        if self.popped_size == 0 {
            return;
        }
        // Translate the C behavior: mark popped strings that match strings still on the stack.
        // In Rust we don't have raw pointers/identity, so we treat the popped buffer as
        // shared ownership and just clear it (since data was cloned on pop).
        // Simply drop everything that is not still referenced (in our model, everything).
        for i in 0..self.popped_size {
            self.popped[i].marked = false;
        }
        for i in 0..self.size {
            if let data::DataValue::Str(s) = &self.buf[i].value {
                for j in 0..self.popped_size {
                    if self.popped[j].str == *s {
                        self.popped[j].marked = true;
                    }
                }
            }
        }
        // In C, unmarked popped strings get freed; in Rust we just clear the popped buffer.
        self.popped.clear();
        self.popped_size = 0;
    }
}
impl Stack {
    fn add_popped(&mut self, s: String) {
        if self.popped_size >= self.popped_cap {
            self.popped_cap *= 2;
        }
        if self.popped.len() > self.popped_size {
            self.popped[self.popped_size] = Popped { str: s, marked: false };
        } else {
            self.popped.push(Popped { str: s, marked: false });
        }
        self.popped_size += 1;
    }
}
