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
        // Drop happens automatically
    }
    pub fn insert_char(&mut self, ch: char) -> i32 {
        if self.gap_len <= 1 {
            let current_cap = self.gap_len + self.str_len;
            self.resize_buffer(current_cap * 2);
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
        let location = if location > self.str_len { self.str_len } else { location };

        let capacity = self.gap_len + self.str_len;
        let mut new_buffer = vec!['\0'; capacity];

        if location < self.gap_loc {
            // Copy up to new location
            new_buffer[..location].copy_from_slice(&self.buffer[..location]);
            // Copy from location to old gap_loc, placing after new gap
            let chunk_len = self.gap_loc - location;
            new_buffer[location + self.gap_len..location + self.gap_len + chunk_len]
                .copy_from_slice(&self.buffer[location..self.gap_loc]);
            // Copy rest of string after old gap
            let rest_len = self.str_len - self.gap_loc;
            let dst_start = location + self.gap_len + chunk_len;
            let src_start = self.gap_loc + self.gap_len;
            new_buffer[dst_start..dst_start + rest_len]
                .copy_from_slice(&self.buffer[src_start..src_start + rest_len]);
        } else {
            // Copy up to original gap
            new_buffer[..self.gap_loc].copy_from_slice(&self.buffer[..self.gap_loc]);
            // Copy what's between old gap and new location
            let chunk_len = location - self.gap_loc;
            let src_start = self.gap_loc + self.gap_len;
            new_buffer[self.gap_loc..self.gap_loc + chunk_len]
                .copy_from_slice(&self.buffer[src_start..src_start + chunk_len]);
            // Copy remaining string after new gap
            let rest_len = self.str_len - location;
            let dst_start = location + self.gap_len;
            let src2_start = src_start + chunk_len;
            new_buffer[dst_start..dst_start + rest_len]
                .copy_from_slice(&self.buffer[src2_start..src2_start + rest_len]);
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
        let after_gap = self.gap_loc + self.gap_len;
        for i in after_gap..after_gap + (self.str_len - self.gap_loc) {
            s.push(self.buffer[i]);
        }
        s
    }
    pub fn split(&self) -> Self {
        let capacity = self.gap_len + self.str_len;
        let second_half_len = self.str_len - self.gap_loc;

        let mut new_buf = GapBuffer::create(capacity);
        // Copy second half to end of new buffer
        let dst_start = capacity - second_half_len;
        let src_start = self.gap_loc + self.gap_len;
        for i in 0..second_half_len {
            new_buf.buffer[dst_start + i] = self.buffer[src_start + i];
        }
        new_buf.str_len = second_half_len;
        new_buf.gap_loc = 0;
        new_buf.gap_len = capacity - second_half_len;
        new_buf
    }
    pub fn create_from_string(s: &str, gap_len: usize) -> Self {
        if s.is_empty() {
            return GapBuffer::create(gap_len);
        }
        let chars: Vec<char> = s.chars().collect();
        let s_len = chars.len();
        let capacity = s_len + gap_len;
        let mut gb = GapBuffer::create(capacity);
        for (i, &c) in chars.iter().enumerate() {
            gb.buffer[i] = c;
        }
        gb.str_len = s_len;
        gb.gap_loc = s_len;
        gb.gap_len = gap_len;
        gb
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

    fn resize_buffer(&mut self, new_capacity: usize) {
        let buffer_size = self.str_len + self.gap_len;
        let new_capacity = if new_capacity < buffer_size { buffer_size } else { new_capacity };
        let gap_size = self.gap_len + (new_capacity - buffer_size);

        let mut new_buffer = vec!['\0'; new_capacity];
        // Copy before gap
        new_buffer[..self.gap_loc].copy_from_slice(&self.buffer[..self.gap_loc]);
        // Copy after gap
        let rest_len = self.str_len - self.gap_loc;
        let dst_start = self.gap_loc + gap_size;
        let src_start = self.gap_loc + self.gap_len;
        new_buffer[dst_start..dst_start + rest_len]
            .copy_from_slice(&self.buffer[src_start..src_start + rest_len]);

        self.buffer = new_buffer;
        self.gap_len = gap_size;
    }
}
