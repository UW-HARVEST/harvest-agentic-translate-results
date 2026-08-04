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

        let mut rd_len = length;
        if self.data_size < rd_len {
            rd_len = self.data_size;
        }

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
            data_out[..frg1_len]
                .copy_from_slice(&self.buffer[self.head_offset..self.size]);
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
        let c_size = self.size;
        let mut s = String::with_capacity(3 * c_size + 1);

        for i in 0..c_size {
            let c: u8 = if self.data_size == 0 {
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
        // Rust automatically frees memory when `self` is dropped
        // at the end of this scope. Nothing to do explicitly.
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
        if length == 0 || self.size == 0 {
            return;
        }

        let mut writable_len = length;
        let mut p_src_offset: usize = 0;

        if writable_len > self.size {
            let overflow_len = writable_len - self.size;
            writable_len = self.size;
            p_src_offset = overflow_len;
        }

        // Determine where to start writing.
        // If buffer is empty, start at index 0.
        // Otherwise, start at (tail_offset + 1) wrapped around.
        let write_start = if self.data_size == 0 {
            0
        } else if self.tail_offset + 1 >= self.size {
            0
        } else {
            self.tail_offset + 1
        };

        // Write the data, possibly in two segments if it wraps.
        let space_until_end = self.size - write_start;
        let first_chunk = space_until_end.min(writable_len);
        self.buffer[write_start..write_start + first_chunk]
            .copy_from_slice(&src[p_src_offset..p_src_offset + first_chunk]);

        if first_chunk < writable_len {
            let second_chunk = writable_len - first_chunk;
            self.buffer[..second_chunk].copy_from_slice(
                &src[p_src_offset + first_chunk..p_src_offset + writable_len],
            );
        }

        // Compute new tail offset.
        let new_tail = if write_start + writable_len - 1 >= self.size {
            (write_start + writable_len - 1) - self.size
        } else {
            write_start + writable_len - 1
        };

        // Update state.
        if self.data_size == 0 {
            self.head_offset = 0;
            self.tail_offset = new_tail;
            self.data_size = writable_len;
        } else {
            let new_data_size = self.data_size + writable_len;
            if new_data_size >= self.size {
                // Buffer became (or was) full; head moves to just after new tail.
                self.head_offset = if new_tail + 1 >= self.size {
                    0
                } else {
                    new_tail + 1
                };
                self.data_size = self.size;
            } else {
                self.data_size = new_data_size;
            }
            self.tail_offset = new_tail;
        }
    }
    pub fn inter_read(&mut self, length: usize, data_out: &mut [u8], reset_head: bool) -> usize {
        if self.data_size == 0 || length == 0 {
            return 0;
        }

        let mut rd_len = length;
        if self.data_size < rd_len {
            rd_len = self.data_size;
        }

        if self.head_offset <= self.tail_offset {
            data_out[..rd_len].copy_from_slice(
                &self.buffer[self.head_offset..self.head_offset + rd_len],
            );

            if reset_head {
                self.head_offset += rd_len;
                if self.head_offset > self.tail_offset {
                    // Buffer becomes empty; reset offsets.
                    self.head_offset = 0;
                    self.tail_offset = 0;
                }
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
            data_out[..frg1_len]
                .copy_from_slice(&self.buffer[self.head_offset..self.size]);
            let frg2_len = rd_len - frg1_len;
            data_out[frg1_len..frg1_len + frg2_len]
                .copy_from_slice(&self.buffer[..frg2_len]);

            if reset_head {
                self.head_offset = frg2_len;
                if self.head_offset > self.tail_offset {
                    // Buffer becomes empty; reset offsets.
                    self.head_offset = 0;
                    self.tail_offset = 0;
                }
            }
        }

        if reset_head {
            self.data_size -= rd_len;
        }

        rd_len
    }
    pub fn get_data_size(&self) -> usize {
        self.data_size
    }
}
