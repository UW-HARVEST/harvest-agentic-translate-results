use std::vec::Vec;
pub struct CircularBuffer {
    size: usize,
    data_size: usize,
    tail_offset: usize,
    head_offset: usize,
    buffer: Vec<u8>,
}
impl CircularBuffer {
    pub fn new(size: usize) -> Self {
        CircularBuffer {
            size,
            data_size: 0,
            tail_offset: 0,
            head_offset: 0,
            buffer: vec![0; size],
        }
    }
    pub fn pop(&mut self, length: usize, data_out: &mut [u8]) -> usize {
        self.inter_read(length, data_out, true)
    }
    pub fn read(&self, length: usize, data_out: &mut [u8]) -> usize {
        if self.data_size == 0 || length == 0 || self.size == 0 {
            return 0;
        }

        let rd_len = length.min(self.data_size);

        for i in 0..rd_len {
            let idx = (self.head_offset + i) % self.size;
            if i < data_out.len() {
                data_out[i] = self.buffer[idx];
            }
        }

        rd_len
    }
    pub fn get_size(&self) -> usize {
        self.size
    }
    pub fn print(&self, hex: bool) {
        let mut s = String::new();
        for i in 0..self.size {
            let in_data = if self.data_size == 0 {
                false
            } else if self.data_size == self.size {
                true
            } else if self.tail_offset < self.head_offset {
                // wrapped: data is in [head_offset..size) U [0..=tail_offset]
                i <= self.tail_offset || i >= self.head_offset
            } else {
                // contiguous: data is in [head_offset..=tail_offset]
                i >= self.head_offset && i <= self.tail_offset
            };

            let c = if !in_data { b'_' } else { self.buffer[i] };
            if hex {
                s.push_str(&format!("{:02X}|", c));
            } else {
                s.push(c as char);
                s.push('|');
            }
        }
        println!(
            "CircularBuffer: {} <size {} dataSize:{}>",
            s, self.size, self.data_size
        );
    }
    pub fn free(self) {
        // Rust will drop the buffer automatically when self goes out of scope.
    }
    pub fn reset(&mut self) {
        self.head_offset = 0;
        self.tail_offset = 0;
        self.data_size = 0;
    }
    pub fn get_capacity(&self) -> usize {
        self.size
    }
    pub fn push(&mut self, src: &[u8], length: usize) {
        if length == 0 || self.size == 0 {
            return;
        }

        let mut writable_len = length;
        let mut src_offset = 0usize;

        // If incoming data is larger than capacity, only the last `size` bytes matter.
        if writable_len > self.size {
            let overflow = writable_len - self.size;
            writable_len = self.size;
            src_offset = overflow;
        }

        // Determine where to start writing.
        let write_start = if self.data_size == 0 {
            0
        } else {
            (self.tail_offset + 1) % self.size
        };

        // Write bytes (wrapping around).
        for i in 0..writable_len {
            let idx = (write_start + i) % self.size;
            self.buffer[idx] = src[src_offset + i];
        }

        if self.data_size == 0 {
            self.head_offset = 0;
        }
        self.tail_offset = (write_start + writable_len - 1) % self.size;

        let new_data_size = self.data_size + writable_len;
        if new_data_size > self.size {
            // Buffer is now full, head must move to just past tail.
            self.head_offset = (self.tail_offset + 1) % self.size;
            self.data_size = self.size;
        } else {
            self.data_size = new_data_size;
        }
    }
    pub fn inter_read(&mut self, length: usize, data_out: &mut [u8], reset_head: bool) -> usize {
        if self.data_size == 0 || length == 0 || self.size == 0 {
            return 0;
        }

        let rd_len = length.min(self.data_size);

        for i in 0..rd_len {
            let idx = (self.head_offset + i) % self.size;
            if i < data_out.len() {
                data_out[i] = self.buffer[idx];
            }
        }

        if reset_head {
            self.data_size -= rd_len;
            if self.data_size == 0 {
                self.head_offset = 0;
                self.tail_offset = 0;
            } else {
                self.head_offset = (self.head_offset + rd_len) % self.size;
            }
        }

        rd_len
    }
    pub fn get_data_size(&self) -> usize {
        self.data_size
    }
}
