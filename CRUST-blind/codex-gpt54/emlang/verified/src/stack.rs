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
    fn new(cap: usize, popped_cap: usize) -> Self {
        Self {
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
            self.cap = self.cap.saturating_mul(2).max(1);
            let additional = self.cap.saturating_sub(self.buf.capacity());
            if additional > 0 {
                self.buf.reserve(additional);
            }
        }

        self.buf.push(data);
        self.size = self.buf.len();
    }
    fn pop(&mut self) -> Option<data::Data> {
        let data = self.buf.pop()?;
        self.size = self.buf.len();

        if let data::DataValue::Str(value) = &data.value {
            if self.popped_size >= self.popped_cap {
                self.popped_cap = self.popped_cap.saturating_mul(2).max(1);
                let additional = self.popped_cap.saturating_sub(self.popped.capacity());
                if additional > 0 {
                    self.popped.reserve(additional);
                }
            }

            self.popped.push(Popped {
                str: value.clone(),
                marked: false,
            });
            self.popped_size = self.popped.len();
        }

        Some(data)
    }
    fn dup(&mut self, off: usize) -> i32 {
        if off + 1 > self.size {
            return -1;
        }

        let idx = self.size - off - 1;
        self.push(self.buf[idx].clone());
        0
    }
    fn swap(&mut self, off: usize) -> i32 {
        if off + 1 > self.size {
            return -1;
        }

        let idx = self.size - off - 1;
        self.buf.swap(idx, self.size - 1);
        0
    }
    fn shrink_to(&mut self, size: usize) {
        if size == self.size {
            return;
        }

        assert!(size < self.size);

        for data in &self.buf[size..self.size] {
            if let data::DataValue::Str(value) = &data.value {
                if self.popped_size >= self.popped_cap {
                    self.popped_cap = self.popped_cap.saturating_mul(2).max(1);
                    let additional = self.popped_cap.saturating_sub(self.popped.capacity());
                    if additional > 0 {
                        self.popped.reserve(additional);
                    }
                }

                self.popped.push(Popped {
                    str: value.clone(),
                    marked: false,
                });
                self.popped_size = self.popped.len();
            }
        }

        self.buf.truncate(size);
        self.size = self.buf.len();
    }
    fn clear(&mut self) {
        self.shrink_to(0);
        self.gc();
    }
    fn gc(&mut self) {
        for popped in &mut self.popped {
            popped.marked = false;
        }

        self.popped.clear();
        self.popped_size = 0;
    }
}
