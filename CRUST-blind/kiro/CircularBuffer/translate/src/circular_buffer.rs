use std::vec::Vec;
pub struct CircularBuffer {
    size: usize,
    data_size: usize,
    tail_offset: usize,
    head_offset: usize,
    buffer: Vec<u8>,
}

const INVALID: usize = usize::MAX;

impl CircularBuffer {
    pub fn new(size: usize) -> Self {
        CircularBuffer {
            size,
            data_size: 0,
            tail_offset: INVALID,
            head_offset: INVALID,
            buffer: vec![0; size],
        }
    }

    pub fn reset(&mut self) {
        self.head_offset = INVALID;
        self.tail_offset = INVALID;
        self.data_size = 0;
    }

    pub fn free(self) {
        // Rust drops automatically; nothing needed.
    }

    pub fn get_capacity(&self) -> usize {
        self.size
    }

    pub fn get_size(&self) -> usize {
        self.size
    }

    pub fn get_data_size(&self) -> usize {
        self.data_size
    }

    pub fn push(&mut self, src: &[u8], length: usize) {
        if length == 0 {
            return;
        }

        let mut writable_len = length;
        let mut p_src_offset = 0usize;

        if writable_len > self.size {
            let overflow = writable_len - self.size;
            writable_len = self.size;
            p_src_offset = overflow;
        }

        let mut reset_head = false;

        // The C code uses tailOffset+1 as the write position.
        // When tailOffset == INVALID (i.e. -1 in C's size_t), wrapping_add(1) gives 0,
        // and wrapping_add(writableLen) gives writableLen. This mirrors the C unsigned arithmetic.
        if self.tail_offset.wrapping_add(writable_len) < self.size {
            let start = self.tail_offset.wrapping_add(1);
            self.buffer[start..start + writable_len]
                .copy_from_slice(&src[p_src_offset..p_src_offset + writable_len]);

            if self.tail_offset < self.head_offset
                && self.tail_offset.wrapping_add(writable_len) >= self.head_offset
            {
                reset_head = true;
            }

            self.tail_offset = self.tail_offset.wrapping_add(writable_len);
        } else {
            let remain_size = self.size - self.tail_offset.wrapping_add(1);
            let start = self.tail_offset.wrapping_add(1);
            self.buffer[start..start + remain_size]
                .copy_from_slice(&src[p_src_offset..p_src_offset + remain_size]);

            let cover_size = writable_len - remain_size;
            self.buffer[..cover_size]
                .copy_from_slice(&src[p_src_offset + remain_size..p_src_offset + remain_size + cover_size]);

            if self.tail_offset < self.head_offset {
                reset_head = true;
            } else if cover_size > self.head_offset {
                reset_head = true;
            }

            self.tail_offset = cover_size.wrapping_sub(1);
        }

        if self.head_offset == INVALID {
            self.head_offset = 0;
        }

        if reset_head {
            if self.tail_offset + 1 < self.size {
                self.head_offset = self.tail_offset + 1;
            } else {
                self.head_offset = 0;
            }
            self.data_size = self.size;
        } else if self.tail_offset >= self.head_offset {
            self.data_size = self.tail_offset - self.head_offset + 1;
        } else {
            self.data_size = self.size - (self.head_offset - self.tail_offset - 1);
        }
    }

    pub fn inter_read(&mut self, length: usize, data_out: &mut [u8], reset_head: bool) -> usize {
        if self.data_size == 0 || length == 0 {
            return 0;
        }

        let rd_len = length.min(self.data_size);

        if self.head_offset <= self.tail_offset {
            data_out[..rd_len]
                .copy_from_slice(&self.buffer[self.head_offset..self.head_offset + rd_len]);

            if reset_head {
                self.head_offset += rd_len;
                if self.head_offset > self.tail_offset {
                    self.head_offset = INVALID;
                    self.tail_offset = INVALID;
                }
            }
        } else if self.head_offset + rd_len <= self.size {
            data_out[..rd_len]
                .copy_from_slice(&self.buffer[self.head_offset..self.head_offset + rd_len]);

            if reset_head {
                self.head_offset += rd_len;
                if self.head_offset == self.size {
                    self.head_offset = 0;
                }
            }
        } else {
            let frg1 = self.size - self.head_offset;
            data_out[..frg1]
                .copy_from_slice(&self.buffer[self.head_offset..self.head_offset + frg1]);

            let frg2 = rd_len - frg1;
            data_out[frg1..frg1 + frg2].copy_from_slice(&self.buffer[..frg2]);

            if reset_head {
                self.head_offset = frg2;
                if self.head_offset > self.tail_offset {
                    self.head_offset = INVALID;
                    self.tail_offset = INVALID;
                }
            }
        }

        if reset_head {
            self.data_size -= rd_len;
        }

        rd_len
    }

    pub fn pop(&mut self, length: usize, data_out: &mut [u8]) -> usize {
        self.inter_read(length, data_out, true)
    }

    pub fn read(&self, length: usize, data_out: &mut [u8]) -> usize {
        // read is non-mutating in the C code, but inter_read takes &mut self.
        // We need a small workaround since the signature is &self.
        if self.data_size == 0 || length == 0 {
            return 0;
        }

        let rd_len = length.min(self.data_size);

        if self.head_offset <= self.tail_offset {
            data_out[..rd_len]
                .copy_from_slice(&self.buffer[self.head_offset..self.head_offset + rd_len]);
        } else if self.head_offset + rd_len <= self.size {
            data_out[..rd_len]
                .copy_from_slice(&self.buffer[self.head_offset..self.head_offset + rd_len]);
        } else {
            let frg1 = self.size - self.head_offset;
            data_out[..frg1]
                .copy_from_slice(&self.buffer[self.head_offset..self.head_offset + frg1]);
            let frg2 = rd_len - frg1;
            data_out[frg1..frg1 + frg2].copy_from_slice(&self.buffer[..frg2]);
        }

        rd_len
    }

    pub fn print(&self, hex: bool) {
        let c_size = self.get_size();
        let mut str_buf = String::new();

        for i in 0..c_size {
            let c: char;
            if self.get_data_size() == 0 {
                c = '_';
            } else if self.tail_offset < self.head_offset {
                if i > self.tail_offset && i < self.head_offset {
                    c = '_';
                } else {
                    c = self.buffer[i] as char;
                }
            } else {
                if i > self.tail_offset || i < self.head_offset {
                    c = '_';
                } else {
                    c = self.buffer[i] as char;
                }
            }
            if hex {
                str_buf.push_str(&format!("{:02X}|", c as u8));
            } else {
                str_buf.push_str(&format!("{}|", c));
            }
        }

        println!(
            "CircularBuffer: {} <size {} dataSize:{}>",
            str_buf,
            self.get_size(),
            self.get_data_size()
        );
    }
}
