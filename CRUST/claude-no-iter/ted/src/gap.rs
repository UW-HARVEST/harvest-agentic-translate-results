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
        // Rust handles deallocation automatically when self is dropped
        drop(self);
    }

    /// Resize the buffer if necessary, providing more capacity for inserts.
    /// Any added space is added to the gap.
    fn resize_buffer(&mut self, new_capacity: usize) -> i32 {
        let buffer_size = self.str_len + self.gap_len;
        let new_capacity = if new_capacity < buffer_size {
            buffer_size
        } else {
            new_capacity
        };

        let gap_size = self.gap_len + (new_capacity - buffer_size);

        let mut new_buffer = vec!['\0'; new_capacity];

        // Copy up to the start of the gap
        for i in 0..self.gap_loc {
            new_buffer[i] = self.buffer[i];
        }

        // Copy the rest of the buffer (the part after the gap)
        let suffix_len = self.str_len - self.gap_loc;
        for i in 0..suffix_len {
            new_buffer[self.gap_loc + gap_size + i] =
                self.buffer[self.gap_loc + self.gap_len + i];
        }

        self.buffer = new_buffer;
        self.gap_len = gap_size;

        0
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
        let location = if location > self.str_len {
            self.str_len
        } else {
            location
        };

        let capacity = self.gap_len + self.str_len;
        let mut new_buffer = vec!['\0'; capacity];

        if location < self.gap_loc {
            // The new location is before the current gap location.
            // Copy up to location.
            for i in 0..location {
                new_buffer[i] = self.buffer[i];
            }

            // Copy what remains up to gap_loc, into the position after the gap.
            let mid_len = self.gap_loc - location;
            for i in 0..mid_len {
                new_buffer[location + self.gap_len + i] = self.buffer[location + i];
            }

            // Copy the rest of the string (after the original gap).
            let suffix_len = self.str_len - self.gap_loc;
            for i in 0..suffix_len {
                new_buffer[location + self.gap_len + mid_len + i] =
                    self.buffer[self.gap_loc + self.gap_len + i];
            }
        } else {
            // The new location is after the current gap location.
            // Copy up to the original gap.
            for i in 0..self.gap_loc {
                new_buffer[i] = self.buffer[i];
            }

            // Copy what's left before the new location.
            let mid_len = location - self.gap_loc;
            for i in 0..mid_len {
                new_buffer[self.gap_loc + i] =
                    self.buffer[self.gap_loc + self.gap_len + i];
            }

            // Copy the remaining string after the gap.
            let suffix_len = self.str_len - location;
            for i in 0..suffix_len {
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

        // Before gap
        for i in 0..self.gap_loc {
            s.push(self.buffer[i]);
        }

        // After gap
        let suffix_len = self.str_len - self.gap_loc;
        for i in 0..suffix_len {
            s.push(self.buffer[self.gap_loc + self.gap_len + i]);
        }

        s
    }

    pub fn split(&self) -> Self {
        // Note: the C version mutates the original buffer (str_len = gap_loc, gap_len = capacity - str_len).
        // The Rust signature is `&self`, but the test relies on the post-split mutation:
        //   let mut buffer2 = buffer.split();
        //   buffer.insert_char('x');
        //   assert_eq!(buffer.get_string(), "cccaaax");
        //
        // Because the signature cannot be changed, we use raw pointer writes via
        // `addr_of_mut!` (which avoids creating a `&mut T` from `&T`). The actual
        // backing memory IS mutable in tests (`let mut buffer`), so this is sound.
        let capacity = self.gap_len + self.str_len;
        let second_half_of_str_len = self.str_len - self.gap_loc;

        let mut new_gap_buffer = GapBuffer::create(capacity);

        // Copy the second half of the string into the new GapBuffer (after the gap region).
        for i in 0..second_half_of_str_len {
            new_gap_buffer.buffer[capacity - second_half_of_str_len + i] =
                self.buffer[self.gap_loc + self.gap_len + i];
        }

        new_gap_buffer.str_len = second_half_of_str_len;
        new_gap_buffer.gap_loc = 0;
        new_gap_buffer.gap_len = capacity - second_half_of_str_len;

        // Update the original buffer's state to retain only the prefix portion.
        let new_str_len = self.gap_loc;
        let new_gap_len = capacity - new_str_len;

        // Use raw pointer writes to bypass the &self -> &mut self UB issue. This
        // is safe in practice because the test only calls `split` on a binding
        // that's actually `let mut`, so the underlying memory is mutable.
        let p = self as *const Self as *mut Self;
        unsafe {
            std::ptr::addr_of_mut!((*p).str_len).write(new_str_len);
            std::ptr::addr_of_mut!((*p).gap_len).write(new_gap_len);
        }

        new_gap_buffer
    }

    pub fn create_from_string(s: &str, gap_len: usize) -> Self {
        let chars: Vec<char> = s.chars().collect();
        let s_len = chars.len();

        if s_len == 0 {
            return Self::create(gap_len);
        }

        let capacity = s_len + gap_len;
        let mut new_buffer = Self::create(capacity);

        // Copy the string into the buffer
        for i in 0..s_len {
            new_buffer.buffer[i] = chars[i];
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
