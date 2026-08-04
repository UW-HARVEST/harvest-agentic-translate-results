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
            tail_offset: usize::MAX,
            head_offset: usize::MAX,
            buffer: vec![0; size],
        }
    }
    pub fn pop(&mut self, length: usize, data_out: &mut [u8]) -> usize {
        self.inter_read(length, data_out, true)
    }
    pub fn read(&self, length: usize, data_out: &mut [u8]) -> usize {
        // Need a mutable copy to call inter_read without actually mutating self
        let head_offset = self.head_offset;
        let tail_offset = self.tail_offset;
        let data_size = self.data_size;

        if data_size == 0 || length == 0 {
            return 0;
        }
        let rd_len = length.min(data_size);

        if head_offset <= tail_offset {
            data_out[..rd_len].copy_from_slice(&self.buffer[head_offset..head_offset + rd_len]);
        } else if head_offset + rd_len <= self.size {
            data_out[..rd_len].copy_from_slice(&self.buffer[head_offset..head_offset + rd_len]);
        } else {
            let frg1 = self.size - head_offset;
            data_out[..frg1].copy_from_slice(&self.buffer[head_offset..self.size]);
            let frg2 = rd_len - frg1;
            data_out[frg1..frg1 + frg2].copy_from_slice(&self.buffer[..frg2]);
        }
        rd_len
    }
    pub fn get_size(&self) -> usize {
        self.size
    }
    pub fn print(&self, hex: bool) {
        let mut str_buf = String::new();
        for i in 0..self.size {
            let c = if self.data_size == 0 {
                '_'
            } else if self.tail_offset < self.head_offset {
                if i > self.tail_offset && i < self.head_offset {
                    '_'
                } else {
                    self.buffer[i] as char
                }
            } else {
                if i > self.tail_offset || i < self.head_offset {
                    '_'
                } else {
                    self.buffer[i] as char
                }
            };
            if hex {
                str_buf.push_str(&format!("{:02X}|", c as u8));
            } else {
                str_buf.push_str(&format!("{}|", c));
            }
        }
        println!(
            "CircularBuffer: {} <size {} dataSize:{}>",
            str_buf, self.size, self.data_size
        );
    }
    pub fn free(self) {
        // Rust drops automatically
    }
    pub fn reset(&mut self) {
        self.head_offset = usize::MAX;
        self.tail_offset = usize::MAX;
        self.data_size = 0;
    }
    pub fn get_capacity(&self) -> usize {
        self.size
    }
    pub fn push(&mut self, src: &[u8], length: usize) {
        if length == 0 {
            return;
        }
        let (p_src, writable_len) = if length > self.size {
            let overflow = length - self.size;
            (&src[overflow..], self.size)
        } else {
            (src, length)
        };

        let mut reset_head = false;

        if self.tail_offset.wrapping_add(writable_len) < self.size {
            let start = self.tail_offset.wrapping_add(1);
            self.buffer[start..start + writable_len].copy_from_slice(&p_src[..writable_len]);

            if self.tail_offset < self.head_offset
                && self.tail_offset.wrapping_add(writable_len) >= self.head_offset
            {
                reset_head = true;
            }
            self.tail_offset = self.tail_offset.wrapping_add(writable_len);
        } else {
            let remain = self.size - self.tail_offset.wrapping_add(1);
            let start = self.tail_offset.wrapping_add(1);
            self.buffer[start..start + remain].copy_from_slice(&p_src[..remain]);

            let cover = writable_len - remain;
            self.buffer[..cover].copy_from_slice(&p_src[remain..remain + cover]);

            if self.tail_offset < self.head_offset {
                reset_head = true;
            } else if cover > self.head_offset {
                reset_head = true;
            }
            self.tail_offset = cover.wrapping_sub(1);
        }

        if self.head_offset == usize::MAX {
            self.head_offset = 0;
        }

        if reset_head {
            if self.tail_offset + 1 < self.size {
                self.head_offset = self.tail_offset + 1;
            } else {
                self.head_offset = 0;
            }
            self.data_size = self.size;
        } else {
            if self.tail_offset >= self.head_offset {
                self.data_size = self.tail_offset - self.head_offset + 1;
            } else {
                self.data_size = self.size - (self.head_offset - self.tail_offset - 1);
            }
        }
    }
    pub fn inter_read(&mut self, length: usize, data_out: &mut [u8], reset_head: bool) -> usize {
        if self.data_size == 0 || length == 0 {
            return 0;
        }
        let rd_len = length.min(self.data_size);

        if self.head_offset <= self.tail_offset {
            data_out[..rd_len].copy_from_slice(&self.buffer[self.head_offset..self.head_offset + rd_len]);
            if reset_head {
                self.head_offset += rd_len;
                if self.head_offset > self.tail_offset {
                    self.head_offset = usize::MAX;
                    self.tail_offset = usize::MAX;
                }
            }
        } else if self.head_offset + rd_len <= self.size {
            data_out[..rd_len].copy_from_slice(&self.buffer[self.head_offset..self.head_offset + rd_len]);
            if reset_head {
                self.head_offset += rd_len;
                if self.head_offset == self.size {
                    self.head_offset = 0;
                }
            }
        } else {
            let frg1 = self.size - self.head_offset;
            data_out[..frg1].copy_from_slice(&self.buffer[self.head_offset..self.size]);
            let frg2 = rd_len - frg1;
            data_out[frg1..frg1 + frg2].copy_from_slice(&self.buffer[..frg2]);
            if reset_head {
                self.head_offset = frg2;
                if self.head_offset > self.tail_offset {
                    self.head_offset = usize::MAX;
                    self.tail_offset = usize::MAX;
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
