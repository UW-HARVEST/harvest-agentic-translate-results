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
    // Resources are dropped automatically when self goes out of scope.
}
pub fn insert_char(&mut self, ch: char) -> i32 {
    // If the gap is about to close, resize it.
    if self.gap_len <= 1 {
        let current_cap = self.gap_len + self.str_len;
        let new_capacity = if current_cap == 0 { 2 } else { current_cap * 2 };
        let err = self.resize_buffer(new_capacity);
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
        // The new location is before the current gap location.
        // Copy up to location.
        for i in 0..location {
            new_buffer[i] = self.buffer[i];
        }

        // Copy what remains up to gap_loc, to the position after the gap in the new buffer.
        let remaining = self.gap_loc - location;
        for i in 0..remaining {
            new_buffer[location + self.gap_len + i] = self.buffer[location + i];
        }

        // Copy the rest of the string after the original gap.
        let tail = self.str_len - self.gap_loc;
        for i in 0..tail {
            new_buffer[location + self.gap_len + remaining + i] =
                self.buffer[self.gap_loc + self.gap_len + i];
        }
    } else {
        // The new location is at or after the current gap location.
        // Copy up to original gap location.
        for i in 0..self.gap_loc {
            new_buffer[i] = self.buffer[i];
        }

        // Copy what's left before the new location.
        let between = location - self.gap_loc;
        for i in 0..between {
            new_buffer[self.gap_loc + i] = self.buffer[self.gap_loc + self.gap_len + i];
        }

        // Now copy the remaining string after the new gap location.
        let tail = self.str_len - location;
        for i in 0..tail {
            new_buffer[location + self.gap_len + i] =
                self.buffer[self.gap_loc + self.gap_len + between + i];
        }
    }

    self.buffer = new_buffer;
    self.gap_loc = location;
    0
}
pub fn get_string(&self) -> String {
    let mut s = String::with_capacity(self.str_len);
    // Before gap
    for i in 0..self.gap_loc {
        s.push(self.buffer[i]);
    }
    // After gap
    let tail = self.str_len - self.gap_loc;
    for i in 0..tail {
        s.push(self.buffer[self.gap_loc + self.gap_len + i]);
    }
    s
}
pub fn split(&mut self) -> Self {
    let capacity = self.gap_len + self.str_len;
    let second_half_of_str_len = self.str_len - self.gap_loc;

    let mut new_gap_buffer = GapBuffer::create(capacity);

    // Copy the second half of the string to the new GapBuffer at the end of the gap.
    for i in 0..second_half_of_str_len {
        new_gap_buffer.buffer[capacity - second_half_of_str_len + i] =
            self.buffer[self.gap_loc + self.gap_len + i];
    }

    new_gap_buffer.str_len = second_half_of_str_len;
    new_gap_buffer.gap_loc = 0;
    new_gap_buffer.gap_len = capacity - second_half_of_str_len;

    // Mutate the original buffer to reflect the first half (matching C semantics).
    self.str_len = self.gap_loc;
    self.gap_len = capacity - self.str_len;

    new_gap_buffer
}
pub fn create_from_string(s: &str, gap_len: usize) -> Self {
    let chars: Vec<char> = s.chars().collect();
    let s_len = chars.len();

    if s_len == 0 {
        return GapBuffer::create(gap_len);
    }

    let capacity = s_len + gap_len;
    let mut new_buffer = GapBuffer::create(capacity);

    // Copy the string to the buffer
    for (i, c) in chars.iter().enumerate() {
        new_buffer.buffer[i] = *c;
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

        // If we increase the capacity, all the new space should go to the gap.
        let gap_size = self.gap_len + (new_capacity - buffer_size);

        let mut new_buffer: Vec<char> = vec!['\0'; new_capacity];

        // Copy up to the start of the gap
        for i in 0..self.gap_loc {
            new_buffer[i] = self.buffer[i];
        }

        // Copy the rest of the buffer, starting from the suffix of the gap
        let tail = self.str_len - self.gap_loc;
        for i in 0..tail {
            new_buffer[self.gap_loc + gap_size + i] =
                self.buffer[self.gap_loc + self.gap_len + i];
        }

        self.buffer = new_buffer;
        self.gap_len = gap_size;
        0
    }
}
