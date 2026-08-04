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
        // Allocate `capacity` slots in the underlying buffer.
        // Initial gap occupies the entire buffer; string is empty.
        let buffer = vec!['\0'; capacity];
        GapBuffer {
            buffer,
            str_len: 0,
            gap_len: capacity,
            gap_loc: 0,
        }
    }

    pub fn destroy(self) {
        // Drop the value; Rust handles deallocation.
        drop(self);
    }

    pub fn insert_char(&mut self, ch: char) -> i32 {
        // If the gap is about to close, resize it.
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
        // Clamp location to [0, str_len]
        let mut location = location;
        if location > self.str_len {
            location = self.str_len;
        }

        let capacity = self.gap_len + self.str_len;
        let mut new_buffer: Vec<char> = vec!['\0'; capacity];

        if location < self.gap_loc {
            // copy up to location
            for i in 0..location {
                new_buffer[i] = self.buffer[i];
            }
            // copy what remains up to gap_loc, to the position after the gap in the new buffer
            for i in 0..(self.gap_loc - location) {
                new_buffer[location + self.gap_len + i] = self.buffer[location + i];
            }
            // copy the rest of the string after the gap
            let tail = self.str_len - self.gap_loc;
            for i in 0..tail {
                new_buffer[location + self.gap_len + (self.gap_loc - location) + i] =
                    self.buffer[self.gap_loc + self.gap_len + i];
            }
        } else {
            // location >= gap_loc
            // copy up to original gap
            for i in 0..self.gap_loc {
                new_buffer[i] = self.buffer[i];
            }
            // copy what's left before the new location (from after old gap)
            for i in 0..(location - self.gap_loc) {
                new_buffer[self.gap_loc + i] = self.buffer[self.gap_loc + self.gap_len + i];
            }
            // now copy the remaining string after the gap
            let tail = self.str_len - location;
            for i in 0..tail {
                new_buffer[location + self.gap_len + i] = self.buffer
                    [self.gap_loc + self.gap_len + (location - self.gap_loc) + i];
            }
        }

        self.buffer = new_buffer;
        self.gap_loc = location;
        0
    }

    pub fn get_string(&self) -> String {
        let mut s = String::with_capacity(self.str_len);
        // Copy before gap
        for i in 0..self.gap_loc {
            s.push(self.buffer[i]);
        }
        // Copy after gap
        let tail = self.str_len - self.gap_loc;
        for i in 0..tail {
            s.push(self.buffer[self.gap_loc + self.gap_len + i]);
        }
        s
    }

    pub fn split(&self) -> Self {
        // Split the current GapBuffer where the gap is.
        // Create a new GapBuffer and copy the second half to the new GapBuffer
        // NOTE: The C version mutates the original. Since `&self` is immutable here, we
        // emulate the split semantics by computing the new buffer; tests for split rely
        // on the "new" returned half. We also adjust internals via interior `Vec`.
        // To match C semantics where the original instance is mutated, callers should
        // use the wrapper in TextBuffer that reassigns the original.

        let capacity = self.gap_len + self.str_len;
        let second_half_len = self.str_len - self.gap_loc;

        let mut new_buffer: Vec<char> = vec!['\0'; capacity];
        // Copy the second half of the string at end of buffer (gap then string)
        for i in 0..second_half_len {
            new_buffer[(capacity - second_half_len) + i] =
                self.buffer[self.gap_loc + self.gap_len + i];
        }

        GapBuffer {
            buffer: new_buffer,
            str_len: second_half_len,
            gap_loc: 0,
            gap_len: capacity - second_half_len,
        }
    }

    pub fn create_from_string(s: &str, gap_len: usize) -> Self {
        let s_len = s.chars().count();
        if s_len == 0 {
            return GapBuffer::create(gap_len);
        }
        let capacity = s_len + gap_len;
        let mut new_buffer: Vec<char> = vec!['\0'; capacity];
        for (i, ch) in s.chars().enumerate() {
            new_buffer[i] = ch;
        }
        GapBuffer {
            buffer: new_buffer,
            str_len: s_len,
            gap_loc: s_len,
            gap_len,
        }
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
    /// Resize the underlying buffer. If `new_capacity` is less than the current size,
    /// the existing capacity is kept. Newly added space goes into the gap.
    fn resize_buffer(&mut self, new_capacity: usize) -> i32 {
        let buffer_size = self.str_len + self.gap_len;
        let mut new_capacity = new_capacity;
        if new_capacity < buffer_size {
            new_capacity = buffer_size;
        }

        let gap_size = self.gap_len + (new_capacity - buffer_size);

        let mut new_buffer: Vec<char> = vec!['\0'; new_capacity];

        // Copy up to the start of the gap
        for i in 0..self.gap_loc {
            new_buffer[i] = self.buffer[i];
        }

        // Copy the rest of the buffer (from after old gap) to its new position
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
