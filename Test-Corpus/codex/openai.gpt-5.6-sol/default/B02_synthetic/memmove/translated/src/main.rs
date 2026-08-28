use std::io::{self, Read, Write};

struct Scanner {
    input: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn from_stdin() -> io::Result<Self> {
        let mut input = Vec::new();
        io::stdin().read_to_end(&mut input)?;
        Ok(Self { input, pos: 0 })
    }

    fn decimal(&mut self) -> Option<(bool, u128)> {
        while self
            .input
            .get(self.pos)
            .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r'))
        {
            self.pos += 1;
        }

        let negative = match self.input.get(self.pos) {
            Some(b'+') => {
                self.pos += 1;
                false
            }
            Some(b'-') => {
                self.pos += 1;
                true
            }
            _ => false,
        };

        let start = self.pos;
        let mut value = 0_u128;
        while let Some(byte @ b'0'..=b'9') = self.input.get(self.pos) {
            value = value
                .saturating_mul(10)
                .saturating_add(u128::from(*byte - b'0'));
            self.pos += 1;
        }

        (self.pos != start).then_some((negative, value))
    }

    fn unsigned_long(&mut self) -> Option<u64> {
        let (negative, magnitude) = self.decimal()?;
        if magnitude > u128::from(u64::MAX) {
            return Some(u64::MAX);
        }

        let value = magnitude as u64;
        Some(if negative {
            value.wrapping_neg()
        } else {
            value
        })
    }

    fn unsigned_int(&mut self) -> Option<u32> {
        self.unsigned_long().map(|value| value as u32)
    }

    fn signed_int(&mut self) -> Option<i32> {
        let (negative, magnitude) = self.decimal()?;
        let value = if negative {
            if magnitude >= (1_u128 << 63) {
                i64::MIN
            } else {
                -(magnitude as i64)
            }
        } else if magnitude > i64::MAX as u128 {
            i64::MAX
        } else {
            magnitude as i64
        };
        Some(value as i32)
    }

    fn size(&mut self) -> Option<usize> {
        self.unsigned_long().map(|value| value as usize)
    }
}

fn rotate_buffer(buf: &mut [u8], len: usize, mut offset: i32) {
    if len <= 1 {
        return;
    }

    offset %= len as i32;
    if offset < 0 {
        offset += len as i32;
    }
    if offset == 0 {
        return;
    }

    let offset = offset as usize;
    let mut temp = [0_u8; 256];
    if offset < len / 2 {
        temp[..offset].copy_from_slice(&buf[..offset]);
        buf.copy_within(offset..len, 0);
        buf[len - offset..len].copy_from_slice(&temp[..offset]);
    } else {
        let shift = len - offset;
        temp[..shift].copy_from_slice(&buf[..shift]);
        buf.copy_within(shift..len, 0);
        buf[offset..len].copy_from_slice(&temp[..shift]);
    }
}

fn compact_runs(buf: &mut [u8], mut len: usize, threshold: u8) -> usize {
    let mut read = 0;
    let mut write = 0;

    while read < len {
        let current = buf[read];
        let mut run_len = 1;
        while read + run_len < len && buf[read + run_len] == current {
            run_len += 1;
        }

        if run_len >= usize::from(threshold) {
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

fn remove_duplicates(buf: &mut [u8], len: usize, preserve_order: bool) -> usize {
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
        let mut seen = [false; 256];
        let mut write = 0;

        for i in 0..len {
            let value = usize::from(buf[i]);
            if !seen[value] {
                seen[value] = true;
                if write != i {
                    buf.swap(write, i);
                }
                write += 1;
            }
        }
        write
    }
}

fn interleave_halves(buf: &mut [u8], len: usize) {
    if len < 2 {
        return;
    }

    let half = len / 2;
    let odd = len % 2;
    let mut temp = [0_u8; 512];

    if half <= 256 {
        temp[..half].copy_from_slice(&buf[..half]);
        for i in 0..half {
            buf[i * 2 + 1] = buf[half + i];
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
                let value = buf[src];
                buf.copy_within(dst..src, dst + 1);
                buf[dst] = value;
            }
        }
    }
}

fn reverse_segments(buf: &mut [u8], len: usize, seg_size: usize) {
    if seg_size <= 1 || len < seg_size {
        return;
    }

    let num_segments = len / seg_size;
    let remainder = len % seg_size;

    for seg in 0..num_segments {
        let base = seg * seg_size;
        for i in 0..seg_size / 2 {
            buf.swap(base + i, base + seg_size - 1 - i);
        }
    }

    if remainder > 1 {
        let base = num_segments * seg_size;
        for i in 0..remainder / 2 {
            buf.swap(base + i, base + remainder - 1 - i);
        }
    }
}

fn process_buffer(buffer: &mut [u8], length: usize, flags: u32, param1: i32, param2: i32) -> usize {
    let mut new_len = length;
    if length == 0 {
        return 0;
    }

    if flags & 0x01 != 0 {
        let offset = param1 % length as i32;
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
        new_len = remove_duplicates(buffer, new_len, param2 != 0);
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

fn fail(message: &str) -> ! {
    let _ = io::stderr().write_all(message.as_bytes());
    std::process::exit(1);
}

fn main() {
    let mut scanner = Scanner::from_stdin().unwrap_or_else(|_| fail("Error reading flags\n"));

    let flags = scanner
        .unsigned_int()
        .unwrap_or_else(|| fail("Error reading flags\n"));
    let param1 = scanner
        .signed_int()
        .unwrap_or_else(|| fail("Error reading param1\n"));
    let param2 = scanner
        .signed_int()
        .unwrap_or_else(|| fail("Error reading param2\n"));
    let length = scanner
        .size()
        .unwrap_or_else(|| fail("Error reading length\n"));

    if length > 256 {
        fail(&format!("Error: length {length} exceeds maximum 256\n"));
    }

    let mut buffer = vec![0_u8; 512];
    for i in 0..length {
        let byte = scanner
            .unsigned_int()
            .unwrap_or_else(|| fail(&format!("Error reading byte {i}\n")));
        buffer[i] = byte as u8;
    }

    let new_length = process_buffer(&mut buffer, length, flags, param1, param2);
    let mut output = new_length.to_string();
    for byte in &buffer[..new_length] {
        output.push(' ');
        output.push_str(&byte.to_string());
    }
    output.push('\n');
    let _ = io::stdout().write_all(output.as_bytes());
}
