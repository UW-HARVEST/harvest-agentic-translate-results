use std::io::{self, Read, Write};

fn main() {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();
    let mut scanner = Scanner::new(input);

    let flags = match scanner.scan_u32() {
        Some(v) => v,
        None => return exit_error("Error reading flags\n"),
    };

    let param1 = match scanner.scan_i32() {
        Some(v) => v,
        None => return exit_error("Error reading param1\n"),
    };

    let param2 = match scanner.scan_i32() {
        Some(v) => v,
        None => return exit_error("Error reading param2\n"),
    };

    let length = match scanner.scan_usize() {
        Some(v) => v,
        None => return exit_error("Error reading length\n"),
    };

    if length > 256 {
        return exit_error(&format!("Error: length {} exceeds maximum 256\n", length));
    }

    let mut buffer = [0_u8; 256];
    for i in 0..length {
        let byte = match scanner.scan_u32() {
            Some(v) => v,
            None => return exit_error(&format!("Error reading byte {}\n", i)),
        };
        buffer[i] = byte as u8;
    }

    let new_length = process_buffer(&mut buffer, length, flags, param1, param2);

    print!("{}", new_length);
    for byte in buffer.iter().take(new_length) {
        print!(" {}", byte);
    }
    println!();
}

fn exit_error(message: &str) {
    let _ = io::stderr().write_all(message.as_bytes());
    std::process::exit(1);
}

struct Scanner {
    bytes: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn new(bytes: Vec<u8>) -> Self {
        Self { bytes, pos: 0 }
    }

    fn scan_u32(&mut self) -> Option<u32> {
        self.scan_unsigned_wrapping(32).map(|v| v as u32)
    }

    fn scan_usize(&mut self) -> Option<usize> {
        self.scan_unsigned_wrapping(usize::BITS).map(|v| v as usize)
    }

    fn scan_i32(&mut self) -> Option<i32> {
        self.scan_unsigned_wrapping(32).map(|v| v as u32 as i32)
    }

    fn scan_unsigned_wrapping(&mut self, bits: u32) -> Option<u128> {
        self.skip_whitespace();
        if self.pos >= self.bytes.len() {
            return None;
        }

        let mut negative = false;
        if self.bytes[self.pos] == b'+' || self.bytes[self.pos] == b'-' {
            negative = self.bytes[self.pos] == b'-';
            self.pos += 1;
        }

        let start_digits = self.pos;
        let mask = if bits == 128 {
            u128::MAX
        } else {
            (1_u128 << bits) - 1
        };
        let mut value = 0_u128;

        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
            value = (value
                .wrapping_mul(10)
                .wrapping_add((self.bytes[self.pos] - b'0') as u128))
                & mask;
            self.pos += 1;
        }

        if self.pos == start_digits {
            return None;
        }

        if negative {
            value = ((!value).wrapping_add(1)) & mask;
        }
        Some(value)
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }
}

fn process_buffer(
    buffer: &mut [u8; 256],
    length: usize,
    flags: u32,
    param1: i32,
    param2: i32,
) -> usize {
    let mut new_len = length;

    if length == 0 {
        return 0;
    }

    if flags & 0x01 != 0 {
        let offset = c_rem_i32(param1, length as i32);
        if offset != 0 {
            rotate_buffer(buffer, length, offset);
        }
    }

    if flags & 0x02 != 0 {
        let threshold = if param1 > 0 && param1 <= 255 {
            param1 as u8
        } else {
            3
        };
        new_len = compact_runs(buffer, new_len, threshold);
    }

    if flags & 0x04 != 0 {
        let preserve = param2 != 0;
        new_len = remove_duplicates(buffer, new_len, preserve);
    }

    if flags & 0x08 != 0 && new_len >= 2 {
        interleave_halves(buffer, new_len);
    }

    if flags & 0x10 != 0 && new_len >= 4 {
        let seg_size = if param1 > 0 { param1 as usize } else { 4 };
        if seg_size <= new_len {
            reverse_segments(buffer, new_len, seg_size);
        }
    }

    new_len
}

fn c_rem_i32(a: i32, b: i32) -> i32 {
    a - (a / b) * b
}

fn rotate_buffer(buf: &mut [u8; 256], len: usize, mut offset: i32) {
    if len <= 1 {
        return;
    }

    offset = c_rem_i32(offset, len as i32);
    if offset < 0 {
        offset += len as i32;
    }
    if offset == 0 {
        return;
    }

    let mut temp = [0_u8; 256];
    let offset = offset as usize;
    let chunk = if offset < 256 { offset } else { 256 };

    if offset < len / 2 {
        let mut i = 0;
        while i < offset {
            let copy_len = if offset - i < chunk {
                offset - i
            } else {
                chunk
            };
            temp[..copy_len].copy_from_slice(&buf[i..i + copy_len]);
            buf.copy_within(offset..len, i);
            buf[len - offset..len - offset + copy_len].copy_from_slice(&temp[..copy_len]);
            i += chunk;
        }
    } else {
        let shift = len - offset;
        temp[..shift].copy_from_slice(&buf[..shift]);
        buf.copy_within(shift..shift + offset, 0);
        buf[offset..offset + shift].copy_from_slice(&temp[..shift]);
    }
}

fn compact_runs(buf: &mut [u8; 256], mut len: usize, threshold: u8) -> usize {
    let mut read = 0;
    let mut write = 0;

    while read < len {
        let current = buf[read];
        let mut run_len = 1;

        while read + run_len < len && buf[read + run_len] == current {
            run_len += 1;
        }

        if run_len >= threshold as usize {
            if run_len > 255 {
                run_len = 255;
            }

            buf[write] = current;
            write += 1;
            buf[write] = run_len as u8;
            write += 1;

            if read + run_len < len {
                let remaining = len - (read + run_len);
                buf.copy_within(read + run_len..read + run_len + remaining, write);
            }
            len = write + (len - (read + run_len));
            read = write;
        } else {
            if write != read {
                buf.copy_within(read..read + run_len, write);
            }
            write += run_len;
            read += run_len;
        }
    }

    len
}

fn remove_duplicates(buf: &mut [u8; 256], len: usize, preserve_order: bool) -> usize {
    if len <= 1 {
        return len;
    }

    if preserve_order {
        let mut write = 1;
        for i in 1..len {
            let mut j = 0;
            while j < write {
                if buf[i] == buf[j] {
                    break;
                }
                j += 1;
            }
            if j == write {
                if write != i {
                    buf[write] = buf[i];
                }
                write += 1;
            }
        }
        write
    } else {
        let mut seen = [0_u8; 256];
        let mut write = 0;

        for i in 0..len {
            if seen[buf[i] as usize] == 0 {
                seen[buf[i] as usize] = 1;
                if write != i {
                    let temp = buf[write];
                    buf[write] = buf[i];
                    buf[i] = temp;
                }
                write += 1;
            }
        }
        write
    }
}

fn interleave_halves(buf: &mut [u8; 256], len: usize) {
    if len < 2 {
        return;
    }

    let half = len / 2;
    let odd = len % 2;
    let mut temp = [0_u8; 512];

    if half <= 256 {
        temp[..half].copy_from_slice(&buf[..half]);

        for i in 0..half {
            buf.copy_within(half + i..half + i + 1, i * 2 + 1);
            buf[i * 2] = temp[i];
        }
        if odd != 0 {
            buf[len - 1] = buf[half];
        }
    } else {
        for i in 0..half {
            let src = half + i;
            let dst = i * 2 + 1;
            if dst < src {
                let val = buf[src];
                buf.copy_within(dst..src, dst + 1);
                buf[dst] = val;
            }
        }
    }
}

fn reverse_segments(buf: &mut [u8; 256], len: usize, seg_size: usize) {
    if seg_size <= 1 || len < seg_size {
        return;
    }

    let num_segments = len / seg_size;
    let remainder = len % seg_size;

    for seg in 0..num_segments {
        let base = seg * seg_size;
        for i in 0..seg_size / 2 {
            let left = base + i;
            let right = base + seg_size - 1 - i;
            let temp = buf[left];
            buf.copy_within(right..right + 1, left);
            buf[right] = temp;
        }
    }

    if remainder > 1 {
        let base = num_segments * seg_size;
        for i in 0..remainder / 2 {
            let temp = buf[base + i];
            buf[base + i] = buf[base + remainder - 1 - i];
            buf[base + remainder - 1 - i] = temp;
        }
    }
}
