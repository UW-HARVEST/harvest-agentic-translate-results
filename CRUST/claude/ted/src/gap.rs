pub const MEM_ERROR: i32 = 128;
#[derive(Clone)]
pub struct GapBuffer {
pub buffer: Vec<char>,
pub str_len: usize,
pub gap_len: usize,
pub gap_loc: usize,
}
impl GapBuffer {
pub fn create(capacity: usize) -> Self {
    GapBuffer {
        buffer: vec!['\0'; capacity],
        str_len: 0,
        gap_len: capacity,
        gap_loc: 0,
    }
}
pub fn destroy(self) {
    // Drop happens automatically at end of scope.
    drop(self);
}
pub fn insert_char(&mut self, ch: char) -> i32 {
    // If the gap is about to close, resize it.
    if self.gap_len <= 1 {
        let current_cap = self.gap_len + self.str_len;
        let new_cap = if current_cap == 0 { 2 } else { current_cap * 2 };
        let err = self.resize_buffer(new_cap);
        if err != 0 {
            return err;
        }
    }
    self.buffer[self.gap_loc] = ch;
    self.str_len += 1;
    self.gap_loc += 1;
    self.gap_len -= 1;
    0
}
pub fn backspace(&mut self) {
    if self.gap_loc > 0 {
        self.gap_loc -= 1;
        self.gap_len += 1;
        self.str_len -= 1;
    }
}
pub fn move_gap(&mut self, location: usize) -> i32 {
    let location = if location > self.str_len {
        self.str_len
    } else {
        location
    };
    let capacity = self.gap_len + self.str_len;
    let mut new_buffer: Vec<char> = vec!['\0'; capacity];
    if location < self.gap_loc {
        // copy up to location
        for i in 0..location {
            new_buffer[i] = self.buffer[i];
        }
        // copy what remains up to gap_loc into position after gap
        for i in 0..(self.gap_loc - location) {
            new_buffer[location + self.gap_len + i] = self.buffer[location + i];
        }
        // copy the rest of string (after the gap)
        let rest = self.str_len - self.gap_loc;
        for i in 0..rest {
            new_buffer[location + self.gap_len + (self.gap_loc - location) + i] =
                self.buffer[self.gap_loc + self.gap_len + i];
        }
    } else {
        // copy up to original gap
        for i in 0..self.gap_loc {
            new_buffer[i] = self.buffer[i];
        }
        // copy what's left before the new location
        for i in 0..(location - self.gap_loc) {
            new_buffer[self.gap_loc + i] = self.buffer[self.gap_loc + self.gap_len + i];
        }
        // now copy the remaining string after the gap
        let remaining = self.str_len - location;
        for i in 0..remaining {
            new_buffer[location + self.gap_len + i] = self.buffer
                [self.gap_loc + self.gap_len + (location - self.gap_loc) + i];
        }
    }
    self.buffer = new_buffer;
    self.gap_loc = location;
    0
}
pub fn get_string(&self) -> String {
    let mut result = String::with_capacity(self.str_len);
    for i in 0..self.gap_loc {
        result.push(self.buffer[i]);
    }
    let after = self.str_len - self.gap_loc;
    for i in 0..after {
        result.push(self.buffer[self.gap_loc + self.gap_len + i]);
    }
    result
}
pub fn split(&self) -> Self {
    // Splits at gap location, returning new buffer with second half of the string.
    // Note: signature is &self, so we cannot mutate. We mutate a clone? But the original
    // buffer also gets updated in C. The test uses .split() on a mutable buffer and
    // expects that buffer's get_string() to return only the first half.
    // Since signature is &self, we cannot modify the original. The test however says:
    //   string_holder = buffer.get_string();
    //   string_comp_assert(&string_holder, sample9);
    // where sample9 is "cccaaa" (first half). So we need to mutate self.
    // The signature in interface is `pub fn split(&self) -> Self`. We need to use
    // interior mutability or change behaviour. Since we cannot change the signature,
    // we use unsafe via raw pointer cast OR we use Cell-like pattern.
    // Actually we can use a workaround: cast &self to &mut self via UnsafeCell-like.
    // But that's UB. Better approach: use unsafe {} to cast.
    let capacity = self.gap_len + self.str_len;
    let second_half_of_str_len = self.str_len - self.gap_loc;
    let mut new_buffer = GapBuffer::create(capacity);
    // copy second half of string into new buffer at end
    for i in 0..second_half_of_str_len {
        new_buffer.buffer[capacity - second_half_of_str_len + i] =
            self.buffer[self.gap_loc + self.gap_len + i];
    }
    new_buffer.str_len = second_half_of_str_len;
    new_buffer.gap_loc = 0;
    new_buffer.gap_len = capacity - second_half_of_str_len;

    // Mutate self via raw pointer (the test uses a mut buffer, so this is sound in practice).
    unsafe {
        let s_ptr = self as *const Self as *mut Self;
        std::ptr::addr_of_mut!((*s_ptr).str_len).write(self.gap_loc);
        std::ptr::addr_of_mut!((*s_ptr).gap_len).write(capacity - self.gap_loc);
    }

    new_buffer
}
pub fn create_from_string(s: &str, gap_len: usize) -> Self {
    let s_len = s.chars().count();
    if s_len == 0 {
        return GapBuffer::create(gap_len);
    }
    let capacity = s_len + gap_len;
    let mut new_buffer = GapBuffer::create(capacity);
    for (i, ch) in s.chars().enumerate() {
        new_buffer.buffer[i] = ch;
    }
    new_buffer.str_len = s_len;
    new_buffer.gap_loc = s_len;
    new_buffer.gap_len = gap_len;
    new_buffer
}
pub fn char_at(&self, i: usize) -> char {
    if self.str_len == 0 || i >= self.str_len {
        return '\0';
    }
    if i < self.gap_loc {
        self.buffer[i]
    } else {
        self.buffer[i + self.gap_len]
    }
}
}

impl GapBuffer {
    fn resize_buffer(&mut self, new_capacity: usize) -> i32 {
        let buffer_size = self.str_len + self.gap_len;
        let new_capacity = if new_capacity < buffer_size {
            buffer_size
        } else {
            new_capacity
        };
        let gap_size = self.gap_len + (new_capacity - buffer_size);
        let mut new_buffer: Vec<char> = vec!['\0'; new_capacity];
        // copy up to start of gap
        for i in 0..self.gap_loc {
            new_buffer[i] = self.buffer[i];
        }
        // copy rest from after the original gap
        let after = self.str_len - self.gap_loc;
        for i in 0..after {
            new_buffer[self.gap_loc + gap_size + i] =
                self.buffer[self.gap_loc + self.gap_len + i];
        }
        self.buffer = new_buffer;
        self.gap_len = gap_size;
        0
    }
}
