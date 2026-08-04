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

        let rd_len = if self.data_size < length {
            self.data_size
        } else {
            length
        };

        for i in 0..rd_len {
            let pos = (self.head_offset + i) % self.size;
            data_out[i] = self.buffer[pos];
        }

        rd_len
    }
    pub fn get_size(&self) -> usize {
        self.size
    }
    pub fn print(&self, hex: bool) {
        let c_size = self.size;
        let mut out = String::new();

        for i in 0..c_size {
            let c: u8 = if self.data_size == 0 {
                b'_'
            } else if self.data_size == self.size {
                self.buffer[i]
            } else {
                // head_offset is the index of the oldest byte; tail_offset is
                // one past the newest. Determine whether index i is inside
                // the occupied range.
                let head = self.head_offset;
                let tail_inclusive = if self.tail_offset == 0 {
                    self.size - 1
                } else {
                    self.tail_offset - 1
                };

                let occupied = if head <= tail_inclusive {
                    i >= head && i <= tail_inclusive
                } else {
                    i >= head || i <= tail_inclusive
                };

                if occupied {
                    self.buffer[i]
                } else {
                    b'_'
                }
            };

            if hex {
                out.push_str(&format!("{:02X}|", c));
            } else {
                out.push(c as char);
                out.push('|');
            }
        }

        println!(
            "CircularBuffer: {} <size {} dataSize:{}>",
            out, self.size, self.data_size
        );
    }
    pub fn free(self) {
        // Rust's ownership system handles deallocation when `self` is dropped.
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
        let mut start = 0usize;

        // If the input is larger than capacity, drop the front portion so only
        // the trailing `size` bytes will be written (matches the C behavior).
        if writable_len > self.size {
            let overflow = writable_len - self.size;
            writable_len = self.size;
            start = overflow;
        }

        // Write `writable_len` bytes into the buffer starting at tail_offset.
        for i in 0..writable_len {
            let pos = (self.tail_offset + i) % self.size;
            self.buffer[pos] = src[start + i];
        }

        // Advance the tail.
        self.tail_offset = (self.tail_offset + writable_len) % self.size;

        // Update the head and data_size. If we wrote more bytes than free
        // space, the buffer becomes full and the head moves to the new tail.
        if self.data_size + writable_len >= self.size {
            self.data_size = self.size;
            self.head_offset = self.tail_offset;
        } else {
            self.data_size += writable_len;
        }
    }
    pub fn inter_read(&mut self, length: usize, data_out: &mut [u8], reset_head: bool) -> usize {
        if self.data_size == 0 || length == 0 || self.size == 0 {
            return 0;
        }

        let rd_len = if self.data_size < length {
            self.data_size
        } else {
            length
        };

        for i in 0..rd_len {
            let pos = (self.head_offset + i) % self.size;
            data_out[i] = self.buffer[pos];
        }

        if reset_head {
            self.head_offset = (self.head_offset + rd_len) % self.size;
            self.data_size -= rd_len;
        }

        rd_len
    }
    pub fn get_data_size(&self) -> usize {
        self.data_size
    }
}
