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
    // Rust drops automatically; nothing to do
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
        // Copy up to location
        new_buffer[..location].copy_from_slice(&self.buffer[..location]);
        // Copy gap_loc - location chars after the gap
        let count = self.gap_loc - location;
        for i in 0..count {
            new_buffer[location + self.gap_len + i] = self.buffer[location + i];
        }
        // Copy rest of string after original gap
        let remaining = self.str_len - self.gap_loc;
        for i in 0..remaining {
            new_buffer[location + self.gap_len + count + i] = self.buffer[self.gap_loc + self.gap_len + i];
        }
    } else {
        // Copy up to original gap
        new_buffer[..self.gap_loc].copy_from_slice(&self.buffer[..self.gap_loc]);
        // Copy what's between old gap and new location
        let count = location - self.gap_loc;
        for i in 0..count {
            new_buffer[self.gap_loc + i] = self.buffer[self.gap_loc + self.gap_len + i];
        }
        // Copy remaining string after new gap position
        let remaining = self.str_len - location;
        for i in 0..remaining {
            new_buffer[location + self.gap_len + i] = self.buffer[self.gap_loc + self.gap_len + count + i];
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
    for i in (self.gap_loc + self.gap_len)..(self.gap_loc + self.gap_len + self.str_len - self.gap_loc) {
        s.push(self.buffer[i]);
    }
    s
}
pub fn split(&self) -> Self {
    let capacity = self.gap_len + self.str_len;
    let second_half_len = self.str_len - self.gap_loc;
    let mut new_gb = GapBuffer::create(capacity);

    // Copy second half to end of new buffer
    for i in 0..second_half_len {
        new_gb.buffer[capacity - second_half_len + i] = self.buffer[self.gap_loc + self.gap_len + i];
    }
    new_gb.str_len = second_half_len;
    new_gb.gap_loc = 0;
    new_gb.gap_len = capacity - second_half_len;
    new_gb
}
pub fn create_from_string(s: &str, gap_len: usize) -> Self {
    if s.is_empty() {
        return GapBuffer::create(gap_len);
    }
    let chars: Vec<char> = s.chars().collect();
    let s_len = chars.len();
    let capacity = s_len + gap_len;
    let mut gb = GapBuffer::create(capacity);
    for i in 0..s_len {
        gb.buffer[i] = chars[i];
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

    // Copy up to gap
    for i in 0..self.gap_loc {
        new_buffer[i] = self.buffer[i];
    }
    // Copy after gap
    let after_gap_start = self.gap_loc + self.gap_len;
    let remaining = self.str_len - self.gap_loc;
    for i in 0..remaining {
        new_buffer[self.gap_loc + gap_size + i] = self.buffer[after_gap_start + i];
    }

    self.buffer = new_buffer;
    self.gap_len = gap_size;
}
}
