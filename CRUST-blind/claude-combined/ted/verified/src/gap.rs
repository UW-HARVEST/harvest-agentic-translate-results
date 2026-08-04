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
        // Rust will drop the buffer automatically when self goes out of scope.
        // This method is a no-op (matches the C "free everything" semantics).
        drop(self);
    }

    pub fn insert_char(&mut self, ch: char) -> i32 {
        // If the gap is about to close, resize it.
        if self.gap_len <= 1 {
            let current_cap = self.gap_len + self.str_len;
            let new_capacity = current_cap * 2;
            let err = self.resize_buffer(new_capacity);
            if err != 0 {
                return err;
            }
        }

        if self.gap_loc >= self.buffer.len() {
            // safety: should not happen given the resize above, but be defensive
            self.buffer.resize(self.gap_loc + 1, '\0');
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
        // location was a usize, but in C it's int and clamped to [0, str_len].
        // In Rust we receive usize so the lower bound (>= 0) is automatic.
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

            // copy what remains up to gap_loc, to the position after the gap
            for i in 0..(self.gap_loc - location) {
                new_buffer[location + self.gap_len + i] = self.buffer[location + i];
            }

            // copy the rest of the string
            // C copies sizeof(char) * instance->str_len - gap_loc bytes
            // (operator precedence: that's str_len - gap_loc)
            let tail_len = self.str_len - self.gap_loc;
            for i in 0..tail_len {
                new_buffer[location + self.gap_len + (self.gap_loc - location) + i] =
                    self.buffer[self.gap_loc + self.gap_len + i];
            }
        } else {
            // new location is after current gap location
            // copy up to gap_loc
            for i in 0..self.gap_loc {
                new_buffer[i] = self.buffer[i];
            }

            // copy what's left before the new location
            let mid_len = location - self.gap_loc;
            for i in 0..mid_len {
                new_buffer[self.gap_loc + i] = self.buffer[self.gap_loc + self.gap_len + i];
            }

            // copy the remaining string after the new gap
            let tail_len = self.str_len - location;
            for i in 0..tail_len {
                new_buffer[location + self.gap_len + i] =
                    self.buffer[self.gap_loc + self.gap_len + mid_len + i];
            }
        }

        self.buffer = new_buffer;
        self.gap_loc = location;
        0
    }

    pub fn get_string(&self) -> String {
        let mut s = String::with_capacity(self.str_len);
        for i in 0..self.gap_loc {
            s.push(self.buffer[i]);
        }
        let tail_len = self.str_len - self.gap_loc;
        for i in 0..tail_len {
            s.push(self.buffer[self.gap_loc + self.gap_len + i]);
        }
        s
    }

    pub fn split(&self) -> Self {
        // Note: C version mutates the original and returns a new GapBuffer.
        // The Rust signature only returns a new buffer; callers must mutate the
        // original separately. Here we still match the semantics by making the
        // returned buffer hold the second half.
        let capacity = self.gap_len + self.str_len;
        let second_half_of_str_len = self.str_len - self.gap_loc;

        let mut new_gap_buffer = GapBuffer::create(capacity);

        // copy the second half of the string to the new GapBuffer at the end
        for i in 0..second_half_of_str_len {
            new_gap_buffer.buffer[capacity - second_half_of_str_len + i] =
                self.buffer[self.gap_loc + self.gap_len + i];
        }

        new_gap_buffer.str_len = second_half_of_str_len;
        new_gap_buffer.gap_loc = 0;
        new_gap_buffer.gap_len = capacity - second_half_of_str_len;

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

        // copy the string to the buffer
        for (i, ch) in chars.iter().enumerate() {
            new_buffer.buffer[i] = *ch;
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
    /// Internal helper: resizes the buffer (and gap) to a new capacity.
    /// If new_capacity <= current size, the same capacity is used.
    fn resize_buffer(&mut self, new_capacity: usize) -> i32 {
        let buffer_size = self.str_len + self.gap_len;
        let new_capacity = if new_capacity < buffer_size {
            buffer_size
        } else {
            new_capacity
        };

        let gap_size = self.gap_len + (new_capacity - buffer_size);

        let mut new_buffer: Vec<char> = vec!['\0'; new_capacity];

        // copy up to the start of the gap
        for i in 0..self.gap_loc {
            new_buffer[i] = self.buffer[i];
        }

        // copy the suffix after the gap
        let tail_len = self.str_len - self.gap_loc;
        for i in 0..tail_len {
            new_buffer[self.gap_loc + gap_size + i] =
                self.buffer[self.gap_loc + self.gap_len + i];
        }

        self.buffer = new_buffer;
        self.gap_len = gap_size;

        0
    }
}
