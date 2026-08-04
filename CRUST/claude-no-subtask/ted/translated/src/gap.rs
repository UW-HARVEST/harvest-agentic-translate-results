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
        // Drop will handle deallocation
        drop(self);
    }

    fn resize_buffer(&mut self, new_capacity: usize) -> i32 {
        let buffer_size = self.str_len + self.gap_len;
        let new_capacity = if new_capacity < buffer_size {
            buffer_size
        } else {
            new_capacity
        };

        let gap_size = self.gap_len + (new_capacity - buffer_size);

        let mut new_buffer = vec!['\0'; new_capacity];

        // Copy up to start of gap
        for i in 0..self.gap_loc {
            new_buffer[i] = self.buffer[i];
        }

        // Copy after gap (offset by new gap size)
        let suffix_len = self.str_len - self.gap_loc;
        for i in 0..suffix_len {
            new_buffer[self.gap_loc + gap_size + i] = self.buffer[self.gap_loc + self.gap_len + i];
        }

        self.buffer = new_buffer;
        self.gap_len = gap_size;
        0
    }

    pub fn insert_char(&mut self, ch: char) -> i32 {
        // If gap is about to close, resize it
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
        // location is usize, so it's always >= 0
        let location = if location > self.str_len {
            self.str_len
        } else {
            location
        };

        let capacity = self.gap_len + self.str_len;
        let mut new_buffer = vec!['\0'; capacity];

        if location < self.gap_loc {
            // copy up to location
            for i in 0..location {
                new_buffer[i] = self.buffer[i];
            }

            // Copy what remains up to instance.gap_loc, after the gap
            let mid_len = self.gap_loc - location;
            for i in 0..mid_len {
                new_buffer[location + self.gap_len + i] = self.buffer[location + i];
            }

            // Copy the rest of the string
            let rest_len = self.str_len - self.gap_loc;
            for i in 0..rest_len {
                new_buffer[location + self.gap_len + mid_len + i] =
                    self.buffer[self.gap_loc + self.gap_len + i];
            }
        } else {
            // copy up to original gap
            for i in 0..self.gap_loc {
                new_buffer[i] = self.buffer[i];
            }

            // copy what's left before the new location
            let mid_len = location - self.gap_loc;
            for i in 0..mid_len {
                new_buffer[self.gap_loc + i] = self.buffer[self.gap_loc + self.gap_len + i];
            }

            // now copy the remaining string after the gap
            let rest_len = self.str_len - location;
            for i in 0..rest_len {
                new_buffer[location + self.gap_len + i] =
                    self.buffer[self.gap_loc + self.gap_len + mid_len + i];
            }
        }

        self.buffer = new_buffer;
        self.gap_loc = location;
        0
    }

    pub fn get_string(&self) -> String {
        let mut result = String::with_capacity(self.str_len);

        // Before gap
        for i in 0..self.gap_loc {
            result.push(self.buffer[i]);
        }

        // After gap
        let suffix_len = self.str_len - self.gap_loc;
        for i in 0..suffix_len {
            result.push(self.buffer[self.gap_loc + self.gap_len + i]);
        }

        result
    }

    pub fn split(&mut self) -> Self {
        let capacity = self.gap_len + self.str_len;
        let second_half_of_str_len = self.str_len - self.gap_loc;

        let mut new_buf = GapBuffer::create(capacity);

        // Copy second half of string into new buffer at the position after the gap
        for i in 0..second_half_of_str_len {
            new_buf.buffer[capacity - second_half_of_str_len + i] =
                self.buffer[self.gap_loc + self.gap_len + i];
        }

        new_buf.str_len = second_half_of_str_len;
        new_buf.gap_loc = 0;
        new_buf.gap_len = capacity - second_half_of_str_len;

        // Update old buffer to contain only the first half (gap fills the rest)
        self.str_len = self.gap_loc;
        self.gap_len = capacity - self.str_len;

        new_buf
    }

    pub fn create_from_string(s: &str, gap_len: usize) -> Self {
        let s_len = s.chars().count();

        if s_len == 0 {
            return GapBuffer::create(gap_len);
        }

        let capacity = s_len + gap_len;
        let mut new_buffer = GapBuffer::create(capacity);

        // Copy string into the buffer
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
