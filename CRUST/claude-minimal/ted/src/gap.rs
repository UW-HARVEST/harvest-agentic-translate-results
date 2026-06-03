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
    // In Rust, dropping the value deallocates it automatically.
    drop(self);
}
pub fn insert_char(&mut self, ch: char) -> i32 {
    // If the gap is about to close, resize it
    if self.gap_len <= 1 {
        let current_cap = self.gap_len + self.str_len;
        let err = self.resize_buffer(current_cap * 2);
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
    let mut location = location;
    if location > self.str_len {
        location = self.str_len;
    }

    let capacity = self.gap_len + self.str_len;
    let mut new_buffer: Vec<char> = vec!['\0'; capacity];

    if location < self.gap_loc {
        // new location is before the current gap location
        // copy up to location
        for i in 0..location {
            new_buffer[i] = self.buffer[i];
        }

        // copy what remains up to gap_loc to position after the gap in new buffer
        for i in 0..(self.gap_loc - location) {
            new_buffer[location + self.gap_len + i] = self.buffer[location + i];
        }

        // copy the rest of the string
        let remaining = self.str_len - self.gap_loc;
        for i in 0..remaining {
            new_buffer[location + self.gap_len + (self.gap_loc - location) + i] =
                self.buffer[self.gap_loc + self.gap_len + i];
        }
    } else {
        // new location is after the current gap location
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
            new_buffer[location + self.gap_len + i] =
                self.buffer[self.gap_loc + self.gap_len + (location - self.gap_loc) + i];
        }
    }

    self.buffer = new_buffer;
    self.gap_loc = location;
    0
}
pub fn get_string(&self) -> String {
    let mut result = String::with_capacity(self.str_len);

    // Copy before gap
    for i in 0..self.gap_loc {
        result.push(self.buffer[i]);
    }

    // Copy after gap
    let remaining = self.str_len - self.gap_loc;
    for i in 0..remaining {
        result.push(self.buffer[self.gap_loc + self.gap_len + i]);
    }

    result
}
pub fn split(&mut self) -> Self {
    let capacity = self.gap_len + self.str_len;
    let second_half_of_str_len = self.str_len - self.gap_loc;

    let mut new_gap_buffer = GapBuffer::create(capacity);

    // Copy the second half of the string to the new GapBuffer.
    // Place it at the end of the new buffer.
    for i in 0..second_half_of_str_len {
        new_gap_buffer.buffer[capacity - second_half_of_str_len + i] =
            self.buffer[self.gap_loc + self.gap_len + i];
    }

    new_gap_buffer.str_len = second_half_of_str_len;
    new_gap_buffer.gap_loc = 0;
    new_gap_buffer.gap_len = capacity - second_half_of_str_len;

    // update the original buffer to contain only the first half
    self.str_len = self.gap_loc;
    self.gap_len = capacity - self.str_len;

    new_gap_buffer
}
pub fn create_from_string(s: &str, gap_len: usize) -> Self {
    let s_len = s.chars().count();

    if s.is_empty() || s_len == 0 {
        return GapBuffer::create(gap_len);
    }

    let capacity = s_len + gap_len;
    let mut new_buffer = GapBuffer::create(capacity);

    // Copy the string to the buffer
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

        // If we increase the capacity, all the new space should go to the gap
        let gap_size = self.gap_len + (new_capacity - buffer_size);

        let mut new_buffer: Vec<char> = vec!['\0'; new_capacity];

        // First, copy up to the start of the gap
        for i in 0..self.gap_loc {
            new_buffer[i] = self.buffer[i];
        }

        // Copy the rest of the buffer, starting from the suffix of the gap
        let remaining = self.str_len - self.gap_loc;
        for i in 0..remaining {
            new_buffer[self.gap_loc + gap_size + i] =
                self.buffer[self.gap_loc + self.gap_len + i];
        }

        self.buffer = new_buffer;
        self.gap_len = gap_size;

        0
    }
}
