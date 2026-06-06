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

        let head = self.head_offset;
        let out_cap = data_out.len();

        if head + rd_len <= self.size {
            // contiguous segment
            let to_copy = rd_len.min(out_cap);
            if to_copy > 0 {
                data_out[..to_copy].copy_from_slice(&self.buffer[head..head + to_copy]);
            }
        } else {
            // wrapped: first segment from head..size, second from 0..remainder
            let frg1_len = self.size - head;
            let frg2_len = rd_len - frg1_len;

            let copy1 = frg1_len.min(out_cap);
            if copy1 > 0 {
                data_out[..copy1].copy_from_slice(&self.buffer[head..head + copy1]);
            }
            if out_cap > frg1_len {
                let copy2 = frg2_len.min(out_cap - frg1_len);
                if copy2 > 0 {
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
        let c_size = self.size;
        let mut str_out = String::with_capacity(2 * c_size + 1);

        for i in 0..c_size {
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
                str_out.push_str(&format!("{:02X}|", c));
            } else {
                str_out.push(c as char);
                str_out.push('|');
            }
        }

        println!(
            "CircularBuffer: {} <size {} dataSize:{}>",
            str_out, self.size, self.data_size
        );
    }
    pub fn free(self) {
        // Rust drops the buffer automatically when self is consumed.
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
        let mut src_offset: usize = 0;

        // In case incoming data exceeds capacity, only the last `size` bytes matter.
        if writable_len > self.size {
            let overflow = writable_len - self.size;
            writable_len = self.size;
            src_offset = overflow;
        }

        // Guard against src being shorter than what we plan to read.
        // (The C code does no such bounds check, but in safe Rust we must.)
        if src_offset + writable_len > src.len() {
            // If the caller passed an inconsistent length, clamp to what is available
            // by taking the trailing `writable_len` bytes of src (matching the
            // semantics of "only the last `size` bytes survive").
            if writable_len > src.len() {
                src_offset = 0;
                writable_len = src.len();
            } else {
                src_offset = src.len() - writable_len;
            }
        }

        if writable_len == 0 {
            return;
        }

        // Compute the write start position: directly after the current tail,
        // unless the buffer is empty in which case we start at 0.
        let write_start = if self.data_size == 0 {
            0
        } else {
            (self.tail_offset + 1) % self.size
        };

        let space_to_end = self.size - write_start;
        let new_tail;
        if writable_len <= space_to_end {
            self.buffer[write_start..write_start + writable_len]
                .copy_from_slice(&src[src_offset..src_offset + writable_len]);
            new_tail = write_start + writable_len - 1;
        } else {
            let first_part = space_to_end;
            let second_part = writable_len - first_part;
            self.buffer[write_start..self.size]
                .copy_from_slice(&src[src_offset..src_offset + first_part]);
            self.buffer[..second_part].copy_from_slice(
                &src[src_offset + first_part..src_offset + writable_len],
            );
            new_tail = second_part - 1;
        }

        // Determine new head/data_size.
        if self.data_size == 0 {
            // Buffer was empty; head sits at the first written byte.
            self.head_offset = write_start;
            self.tail_offset = new_tail;
            self.data_size = writable_len;
        } else {
            let new_data_size = self.data_size + writable_len;
            if new_data_size >= self.size {
                // We've overwritten old data; head jumps to one past the new tail.
                self.head_offset = (new_tail + 1) % self.size;
                self.tail_offset = new_tail;
                self.data_size = self.size;
            } else {
                self.tail_offset = new_tail;
                self.data_size = new_data_size;
                // head_offset unchanged
            }
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

        let head = self.head_offset;
        let out_cap = data_out.len();

        if head + rd_len <= self.size {
            // contiguous segment
            let to_copy = rd_len.min(out_cap);
            if to_copy > 0 {
                data_out[..to_copy].copy_from_slice(&self.buffer[head..head + to_copy]);
            }
        } else {
            // wrapped segment
            let frg1_len = self.size - head;
            let frg2_len = rd_len - frg1_len;

            let copy1 = frg1_len.min(out_cap);
            if copy1 > 0 {
                data_out[..copy1].copy_from_slice(&self.buffer[head..head + copy1]);
            }
            if out_cap > frg1_len {
                let copy2 = frg2_len.min(out_cap - frg1_len);
                if copy2 > 0 {
                    data_out[frg1_len..frg1_len + copy2]
                        .copy_from_slice(&self.buffer[..copy2]);
                }
            }
        }

        if reset_head {
            self.data_size -= rd_len;
            if self.data_size == 0 {
                self.head_offset = 0;
                self.tail_offset = 0;
            } else {
                self.head_offset = (head + rd_len) % self.size;
            }
        }

        rd_len
    }
    pub fn get_data_size(&self) -> usize {
        self.data_size
    }
}
