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
            // contiguous region
            let copy_len = rd_len.min(data_out.len());
            data_out[..copy_len]
                .copy_from_slice(&self.buffer[self.head_offset..self.head_offset + copy_len]);
        } else {
            // wraps around
            if self.head_offset + rd_len <= self.size {
                let copy_len = rd_len.min(data_out.len());
                data_out[..copy_len]
                    .copy_from_slice(&self.buffer[self.head_offset..self.head_offset + copy_len]);
            } else {
                let frg1_len = self.size - self.head_offset;
                let copy1 = frg1_len.min(data_out.len());
                data_out[..copy1]
                    .copy_from_slice(&self.buffer[self.head_offset..self.head_offset + copy1]);

                if data_out.len() > frg1_len {
                    let frg2_len = rd_len - frg1_len;
                    let copy2 = frg2_len.min(data_out.len() - frg1_len);
                    data_out[frg1_len..frg1_len + copy2]
                        .copy_from_slice(&self.buffer[..copy2]);
                }
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
            let c: u8 = if self.data_size == 0 {
                b'_'
            } else if self.tail_offset < self.head_offset {
                if i > self.tail_offset && i < self.head_offset {
                    b'_'
                } else {
                    self.buffer[i]
                }
            } else {
                if i > self.tail_offset || i < self.head_offset {
                    b'_'
                } else {
                    self.buffer[i]
                }
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
        // Rust drops automatically; nothing to do.
        drop(self);
    }
    pub fn reset(&mut self) {
        // C uses -1 sentinel for empty; we use data_size == 0 to indicate empty
        // and zero out the offsets for predictability.
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
        let mut src_start: usize = 0;

        // If incoming data is larger than capacity, only the trailing `size` bytes are kept.
        if writable_len > self.size {
            let overflow_len = writable_len - self.size;
            writable_len = self.size;
            src_start = overflow_len;
        }

        // Determine where to start writing.
        let write_start = if self.data_size == 0 {
            0
        } else {
            (self.tail_offset + 1) % self.size
        };

        // Write writable_len bytes starting at write_start with wrap-around.
        for i in 0..writable_len {
            let pos = (write_start + i) % self.size;
            self.buffer[pos] = src[src_start + i];
        }

        let new_tail = (write_start + writable_len - 1) % self.size;

        let prev_data_size = self.data_size;
        let new_data_size = if prev_data_size + writable_len > self.size {
            self.size
        } else {
            prev_data_size + writable_len
        };

        // Adjust head_offset
        if prev_data_size == 0 {
            // First data being written; head starts at 0
            self.head_offset = 0;
        } else if prev_data_size + writable_len > self.size {
            // We overwrote some data; head moves to just after new tail.
            self.head_offset = (new_tail + 1) % self.size;
        }
        // else: head stays unchanged

        self.tail_offset = new_tail;
        self.data_size = new_data_size;
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
            // contiguous region from head_offset .. tail_offset (inclusive)
            let copy_len = rd_len.min(data_out.len());
            data_out[..copy_len]
                .copy_from_slice(&self.buffer[self.head_offset..self.head_offset + copy_len]);

            if reset_head {
                self.head_offset += rd_len;
                if self.head_offset > self.tail_offset {
                    // emptied
                    self.head_offset = 0;
                    self.tail_offset = 0;
                }
            }
        } else {
            // wraps around
            if self.head_offset + rd_len <= self.size {
                let copy_len = rd_len.min(data_out.len());
                data_out[..copy_len]
                    .copy_from_slice(&self.buffer[self.head_offset..self.head_offset + copy_len]);

                if reset_head {
                    self.head_offset += rd_len;
                    if self.head_offset == self.size {
                        self.head_offset = 0;
                    }
                }
            } else {
                let frg1_len = self.size - self.head_offset;
                let copy1 = frg1_len.min(data_out.len());
                data_out[..copy1]
                    .copy_from_slice(&self.buffer[self.head_offset..self.head_offset + copy1]);

                if data_out.len() > frg1_len {
                    let frg2_len = rd_len - frg1_len;
                    let copy2 = frg2_len.min(data_out.len() - frg1_len);
                    data_out[frg1_len..frg1_len + copy2]
                        .copy_from_slice(&self.buffer[..copy2]);
                }

                if reset_head {
                    let frg2_len = rd_len - frg1_len;
                    self.head_offset = frg2_len;
                    if self.head_offset > self.tail_offset {
                        // emptied
                        self.head_offset = 0;
                        self.tail_offset = 0;
                    }
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
