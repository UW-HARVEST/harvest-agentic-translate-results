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
        let mut buffer = CircularBuffer {
            size,
            data_size: 0,
            tail_offset: usize::MAX,
            head_offset: usize::MAX,
            buffer: vec![0; size],
        };
        buffer.reset();
        buffer
    }
    pub fn pop(&mut self, length: usize, data_out: &mut [u8]) -> usize {
        self.inter_read(length, data_out, true)
    }
    pub fn read(&self, length: usize, data_out: &mut [u8]) -> usize {
        if self.data_size == 0 || length == 0 {
            return 0;
        }

        let rd_len = length.min(self.data_size);
        let copy_len = rd_len.min(data_out.len());

        if copy_len == 0 {
            return rd_len;
        }

        if self.head_offset <= self.tail_offset {
            data_out[..copy_len]
                .copy_from_slice(&self.buffer[self.head_offset..self.head_offset + copy_len]);
        } else {
            let first_len = (self.size - self.head_offset).min(copy_len);
            data_out[..first_len]
                .copy_from_slice(&self.buffer[self.head_offset..self.head_offset + first_len]);

            let second_len = copy_len - first_len;
            if second_len > 0 {
                data_out[first_len..first_len + second_len]
                    .copy_from_slice(&self.buffer[..second_len]);
            }
        }

        rd_len
    }
    pub fn get_size(&self) -> usize {
        self.size
    }
    pub fn print(&self, hex: bool) {
        let mut rendered = String::new();

        for i in 0..self.get_size() {
            let c = if self.get_data_size() == 0 {
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
                rendered.push_str(&format!("{c:02X}|"));
            } else {
                rendered.push(c as char);
                rendered.push('|');
            }
        }

        println!(
            "CircularBuffer: {} <size {} dataSize:{}>",
            rendered,
            self.get_size(),
            self.get_data_size()
        );
    }
    pub fn free(self) {
        drop(self);
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
        if length == 0 || self.size == 0 || src.is_empty() {
            return;
        }

        let requested = length.min(src.len());
        if requested == 0 {
            return;
        }

        let (p_src, writable_len) = if requested > self.size {
            (&src[requested - self.size..requested], self.size)
        } else {
            (&src[..requested], requested)
        };

        let tail = self.tail_offset;

        let mut reset_head = false;

        if tail.wrapping_add(writable_len) < self.size {
            let start = tail.wrapping_add(1);
            self.buffer[start..start + writable_len].copy_from_slice(p_src);

            if tail < self.head_offset && tail.wrapping_add(writable_len) >= self.head_offset {
                reset_head = true;
            }

            self.tail_offset = tail.wrapping_add(writable_len);
        } else {
            let remain_size = self.size.wrapping_sub(tail).wrapping_sub(1);
            let split = remain_size.min(p_src.len());
            let start = tail.wrapping_add(1);
            self.buffer[start..start + split].copy_from_slice(&p_src[..split]);

            let cover_size = writable_len - remain_size;
            if cover_size > 0 {
                self.buffer[..cover_size].copy_from_slice(&p_src[remain_size..remain_size + cover_size]);
            }

            if tail < self.head_offset || cover_size > self.head_offset {
                reset_head = true;
            }

            self.tail_offset = cover_size - 1;
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
        let copy_len = rd_len.min(data_out.len());

        if self.head_offset <= self.tail_offset {
            if copy_len > 0 {
                data_out[..copy_len]
                    .copy_from_slice(&self.buffer[self.head_offset..self.head_offset + copy_len]);
            }

            if reset_head {
                self.head_offset += rd_len;
                if self.head_offset > self.tail_offset {
                    self.head_offset = usize::MAX;
                    self.tail_offset = usize::MAX;
                }
            }
        } else if self.head_offset + rd_len <= self.size {
            if copy_len > 0 {
                data_out[..copy_len]
                    .copy_from_slice(&self.buffer[self.head_offset..self.head_offset + copy_len]);
            }

            if reset_head {
                self.head_offset += rd_len;
                if self.head_offset == self.size {
                    self.head_offset = 0;
                }
            }
        } else {
            let first_len = self.size - self.head_offset;
            let first_copy = first_len.min(copy_len);
            if first_copy > 0 {
                data_out[..first_copy]
                    .copy_from_slice(&self.buffer[self.head_offset..self.head_offset + first_copy]);
            }

            let second_len = rd_len - first_len;
            let second_copy = copy_len.saturating_sub(first_len).min(second_len);
            if second_copy > 0 {
                data_out[first_len..first_len + second_copy]
                    .copy_from_slice(&self.buffer[..second_copy]);
            }

            if reset_head {
                self.head_offset = second_len;
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
