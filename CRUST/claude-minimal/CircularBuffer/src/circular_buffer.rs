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
        if self.data_size == 0 || length == 0 {
            return 0;
        }
        let rd_len = if self.data_size < length {
            self.data_size
        } else {
            length
        };

        if self.head_offset <= self.tail_offset {
            data_out[..rd_len].copy_from_slice(
                &self.buffer[self.head_offset..self.head_offset + rd_len],
            );
        } else if self.head_offset + rd_len <= self.size {
            data_out[..rd_len].copy_from_slice(
                &self.buffer[self.head_offset..self.head_offset + rd_len],
            );
        } else {
            let frg1_len = self.size - self.head_offset;
            data_out[..frg1_len].copy_from_slice(&self.buffer[self.head_offset..]);
            let frg2_len = rd_len - frg1_len;
            data_out[frg1_len..frg1_len + frg2_len]
                .copy_from_slice(&self.buffer[..frg2_len]);
        }

        rd_len
    }
    pub fn get_size(&self) -> usize {
        self.size
    }
    pub fn print(&self, hex: bool) {
        let mut s = String::new();
        for i in 0..self.size {
            let c = if self.data_size == 0 {
                b'_'
            } else if self.tail_offset < self.head_offset {
                if i > self.tail_offset && i < self.head_offset {
                    b'_'
                } else {
                    self.buffer[i]
                }
            } else if i > self.tail_offset || i < self.head_offset {
                b'_'
            } else {
                self.buffer[i]
            };
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
        // Rust automatically drops the buffer when `self` goes out of scope.
        drop(self);
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
        if length == 0 {
            return;
        }

        let mut writable_len = length;
        let mut src_offset = 0;

        // In case of size overflow, only keep the last `size` bytes from src.
        if writable_len > self.size {
            let over_flow_len = writable_len - self.size;
            writable_len = self.size;
            src_offset = over_flow_len;
        }

        // Determine the position to start writing.
        let write_start = if self.data_size == 0 {
            0
        } else {
            (self.tail_offset + 1) % self.size
        };

        // Write the bytes, wrapping around as needed.
        for i in 0..writable_len {
            self.buffer[(write_start + i) % self.size] = src[src_offset + i];
        }

        // Compute the new tail offset.
        let new_tail = (write_start + writable_len - 1) % self.size;

        // Determine the new data size, capped at the buffer's capacity.
        let total = self.data_size + writable_len;
        let new_data_size = if total > self.size { self.size } else { total };

        if total > self.size {
            // Some old data was overwritten; advance head past tail.
            self.head_offset = (new_tail + 1) % self.size;
        } else if self.data_size == 0 {
            // First write into an empty buffer; head starts at write_start.
            self.head_offset = write_start;
        }
        // Otherwise the head offset is unchanged.

        self.tail_offset = new_tail;
        self.data_size = new_data_size;
    }
    pub fn inter_read(&mut self, length: usize, data_out: &mut [u8], reset_head: bool) -> usize {
        if self.data_size == 0 || length == 0 {
            return 0;
        }
        let rd_len = if self.data_size < length {
            self.data_size
        } else {
            length
        };

        if self.head_offset <= self.tail_offset {
            data_out[..rd_len].copy_from_slice(
                &self.buffer[self.head_offset..self.head_offset + rd_len],
            );

            if reset_head {
                self.head_offset += rd_len;
            }
        } else if self.head_offset + rd_len <= self.size {
            data_out[..rd_len].copy_from_slice(
                &self.buffer[self.head_offset..self.head_offset + rd_len],
            );

            if reset_head {
                self.head_offset += rd_len;
                if self.head_offset == self.size {
                    self.head_offset = 0;
                }
            }
        } else {
            let frg1_len = self.size - self.head_offset;
            data_out[..frg1_len].copy_from_slice(&self.buffer[self.head_offset..]);
            let frg2_len = rd_len - frg1_len;
            data_out[frg1_len..frg1_len + frg2_len]
                .copy_from_slice(&self.buffer[..frg2_len]);

            if reset_head {
                self.head_offset = frg2_len;
            }
        }

        if reset_head {
            self.data_size -= rd_len;
            if self.data_size == 0 {
                self.head_offset = 0;
                self.tail_offset = 0;
            }
        }

        rd_len
    }
    pub fn get_data_size(&self) -> usize {
        self.data_size
    }
}
