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
            self.cap *= 2;
            self.buf.reserve(self.cap.saturating_sub(self.buf.capacity()));
        }

        self.buf.push(data);
        self.size = self.buf.len();
    }
    fn pop(&mut self) -> Option<data::Data> {
        let data = self.buf.pop()?;
        self.size = self.buf.len();

        if let data::DataValue::Str(value) = &data.value {
            self.add_popped(value.clone());
        }

        Some(data)
    }
    fn dup(&mut self, off: usize) -> i32 {
        if off + 1 > self.size {
            return -1;
        }

        let index = self.size - off - 1;
        let data = self.buf[index].clone();
        self.push(data);
        0
    }
    fn swap(&mut self, off: usize) -> i32 {
        if off + 1 > self.size {
            return -1;
        }

        let index = self.size - off - 1;
        self.buf.swap(index, self.size - 1);
        0
    }
    fn shrink_to(&mut self, size: usize) {
        if size == self.size {
            return;
        }

        assert!(size < self.size);

        let popped: Vec<String> = self.buf[size..self.size]
            .iter()
            .filter_map(|data| match &data.value {
                data::DataValue::Str(value) => Some(value.clone()),
                _ => None,
            })
            .collect();

        for value in popped {
            self.add_popped(value);
        }

        self.buf.truncate(size);
        self.size = self.buf.len();
    }
    fn clear(&mut self) {
        if self.size > 0 {
            self.shrink_to(0);
        }
        self.gc();
    }
    fn gc(&mut self) {
        self.popped.clear();
        self.popped_size = 0;
    }

    fn add_popped(&mut self, value: String) {
        if self.popped_size >= self.popped_cap {
            self.popped_cap *= 2;
            self.popped
                .reserve(self.popped_cap.saturating_sub(self.popped.capacity()));
        }

        self.popped.push(Popped {
            str: value,
            marked: false,
        });
        self.popped_size = self.popped.len();
    }
}

pub(crate) fn stack_new(cap: usize, popped_cap: usize) -> Stack {
    Stack::new(cap, popped_cap)
}

pub(crate) fn stack_push(stack: &mut Stack, data: data::Data) {
    stack.push(data);
}

pub(crate) fn stack_pop(stack: &mut Stack) -> Option<data::Data> {
    stack.pop()
}

pub(crate) fn stack_dup(stack: &mut Stack, off: usize) -> i32 {
    stack.dup(off)
}

pub(crate) fn stack_swap(stack: &mut Stack, off: usize) -> i32 {
    stack.swap(off)
}

pub(crate) fn stack_shrink_to(stack: &mut Stack, size: usize) {
    stack.shrink_to(size);
}

pub(crate) fn stack_clear(stack: &mut Stack) {
    stack.clear();
}

pub(crate) fn stack_gc(stack: &mut Stack) {
    stack.gc();
}
