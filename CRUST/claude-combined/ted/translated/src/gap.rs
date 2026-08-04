pub const MEM_ERROR: i32 = 128;

#[derive(Clone)]
pub struct GapBuffer {
    pub buffer: Vec<char>,
    pub str_len: usize,
    pub gap_len: usize,
    pub gap_loc: usize,
}

impl GapBuffer {
    /// Creates and initializes a new gap buffer with initial gap of size `capacity`.
    pub fn create(capacity: usize) -> Self {
        // Allocate a buffer of size `capacity` filled with null chars
        let buffer = vec!['\0'; capacity];
        GapBuffer {
            buffer,
            str_len: 0,
            gap_len: capacity,
            gap_loc: 0,
        }
    }

    /// Safely deallocates a gap buffer. In Rust this is a no-op since drop handles it,
    /// but we provide it for API parity.
    pub fn destroy(self) {
        // Drop happens at end of scope
        drop(self);
    }

    /// Helper: resize the underlying buffer. If `new_capacity` is less than the current
    /// total used space, it remains unchanged. New space is added to the gap.
    fn resize_buffer(&mut self, new_capacity: usize) -> i32 {
        let buffer_size = self.str_len + self.gap_len;
        let new_capacity = if new_capacity < buffer_size {
            buffer_size
        } else {
            new_capacity
        };

        // The added space goes into the gap.
        let gap_size = self.gap_len + (new_capacity - buffer_size);

        let mut new_buffer = vec!['\0'; new_capacity];

        // Copy [0..gap_loc) (the prefix) verbatim
        for i in 0..self.gap_loc {
            new_buffer[i] = self.buffer[i];
        }

        // Copy the suffix [gap_loc + gap_len..gap_loc + gap_len + (str_len - gap_loc))
        // to its new location after the (potentially larger) gap.
        let suffix_len = self.str_len - self.gap_loc;
        for i in 0..suffix_len {
            new_buffer[self.gap_loc + gap_size + i] = self.buffer[self.gap_loc + self.gap_len + i];
        }

        self.buffer = new_buffer;
        self.gap_len = gap_size;
        0
    }

    /// Inserts a character into the gap buffer. Resizes if the gap is about to close.
    pub fn insert_char(&mut self, ch: char) -> i32 {
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

    /// Backspace deletes a character from the prefix of the gap, extending the gap length.
    pub fn backspace(&mut self) {
        if self.gap_loc > 0 {
            self.gap_loc -= 1;
            self.gap_len += 1;
            self.str_len -= 1;
        }
    }

    /// Moves the gap to a new location in the string.
    pub fn move_gap(&mut self, location: usize) -> i32 {
        // location is usize so cannot be negative; clamp to str_len above
        let location = if location > self.str_len {
            self.str_len
        } else {
            location
        };

        let capacity = self.gap_len + self.str_len;
        let mut new_buffer = vec!['\0'; capacity];

        if location < self.gap_loc {
            // The new location is before the current gap location
            // Copy up to location
            for i in 0..location {
                new_buffer[i] = self.buffer[i];
            }
            // Then copy from location up to old gap_loc, into position after the gap in new buffer
            let count = self.gap_loc - location;
            for i in 0..count {
                new_buffer[location + self.gap_len + i] = self.buffer[location + i];
            }
            // Copy the rest of the string (from after the old gap)
            let suffix_len = self.str_len - self.gap_loc;
            for i in 0..suffix_len {
                new_buffer[location + self.gap_len + count + i] =
                    self.buffer[self.gap_loc + self.gap_len + i];
            }
        } else {
            // The new location is at or after the current gap location
            // Copy up to original gap
            for i in 0..self.gap_loc {
                new_buffer[i] = self.buffer[i];
            }
            // Copy what's left before the new location
            let mid_count = location - self.gap_loc;
            for i in 0..mid_count {
                new_buffer[self.gap_loc + i] = self.buffer[self.gap_loc + self.gap_len + i];
            }
            // Now copy the remaining string after the gap
            let remaining = self.str_len - location;
            for i in 0..remaining {
                new_buffer[location + self.gap_len + i] =
                    self.buffer[self.gap_loc + self.gap_len + mid_count + i];
            }
        }

        self.buffer = new_buffer;
        self.gap_loc = location;
        0
    }

    /// Returns the current string contents of the gap buffer (excluding the gap).
    pub fn get_string(&self) -> String {
        let mut s = String::with_capacity(self.str_len);
        // Copy before gap
        for i in 0..self.gap_loc {
            s.push(self.buffer[i]);
        }
        // Copy after gap
        let suffix_len = self.str_len - self.gap_loc;
        for i in 0..suffix_len {
            s.push(self.buffer[self.gap_loc + self.gap_len + i]);
        }
        s
    }

    /// Splits the buffer at the cursor (gap) location, returning a new buffer containing the
    /// second half of the string. The original buffer keeps the first half.
    pub fn split(&mut self) -> Self {
        let capacity = self.gap_len + self.str_len;
        let second_half_len = self.str_len - self.gap_loc;

        let mut new_gap_buffer = GapBuffer::create(capacity);

        // Copy the second half of the string into the new buffer at position
        // (capacity - second_half_len) — i.e., right after the gap.
        for i in 0..second_half_len {
            new_gap_buffer.buffer[capacity - second_half_len + i] =
                self.buffer[self.gap_loc + self.gap_len + i];
        }

        new_gap_buffer.str_len = second_half_len;
        new_gap_buffer.gap_loc = 0;
        new_gap_buffer.gap_len = capacity - second_half_len;

        // Update old buffer
        self.str_len = self.gap_loc;
        self.gap_len = capacity - self.str_len;

        new_gap_buffer
    }

    /// Create a gap buffer containing the contents of `s`. Gap is at end of string with `gap_len`.
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

    /// Returns the character at the logical index `i` (ignoring the gap).
    /// Returns `'\0'` if i is out of bounds or the buffer is empty.
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
