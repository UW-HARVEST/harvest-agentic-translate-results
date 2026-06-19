use libc::{c_char, strtol, strtoul};
use std::io::{self, Read, Write};

fn main() {
    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() {
        return;
    }

    let mut scanner = Scanner::new(input);

    let flags = match scanner.scan_u32() {
        Some(value) => value,
        None => return fail("Error reading flags\n"),
    };

    let param1 = match scanner.scan_i32() {
        Some(value) => value,
        None => return fail("Error reading param1\n"),
    };

    let param2 = match scanner.scan_i32() {
        Some(value) => value,
        None => return fail("Error reading param2\n"),
    };

    let length = match scanner.scan_usize() {
        Some(value) => value,
        None => return fail("Error reading length\n"),
    };

    if length > 256 {
        let _ = writeln!(
            io::stderr(),
            "Error: length {} exceeds maximum 256",
            length
        );
        std::process::exit(1);
    }

    let mut buffer = [0u8; 256];
    for (i, slot) in buffer[..length].iter_mut().enumerate() {
        let byte = match scanner.scan_u32() {
            Some(value) => value,
            None => return fail(&format!("Error reading byte {}\n", i)),
        };
        *slot = byte as u8;
    }

    let new_length = process_buffer(&mut buffer, length, flags, param1, param2);

    print!("{}", new_length);
    for byte in &buffer[..new_length] {
        print!(" {}", *byte as u32);
    }
    println!();
}

fn fail(message: &str) {
    let _ = io::stderr().write_all(message.as_bytes());
    std::process::exit(1);
}

struct Scanner {
    bytes: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn new(mut bytes: Vec<u8>) -> Self {
        bytes.push(0);
        Self { bytes, pos: 0 }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if !b.is_ascii_whitespace() {
                break;
            }
            self.pos += 1;
        }
    }

    fn scan_u32(&mut self) -> Option<u32> {
        self.skip_whitespace();
        let start = self.pos;
        let parsed = parse_unsigned(&self.bytes[start..])?;
        self.pos += parsed.1;
        Some(parsed.0 as u32)
    }

    fn scan_i32(&mut self) -> Option<i32> {
        self.skip_whitespace();
        let start = self.pos;
        let parsed = parse_signed(&self.bytes[start..])?;
        self.pos += parsed.1;
        Some(parsed.0 as i32)
    }

    fn scan_usize(&mut self) -> Option<usize> {
        self.skip_whitespace();
        let start = self.pos;
        let parsed = parse_unsigned(&self.bytes[start..])?;
        self.pos += parsed.1;
        Some(parsed.0 as usize)
    }
}

fn parse_unsigned(bytes: &[u8]) -> Option<(u64, usize)> {
    if bytes.is_empty() || bytes[0] == 0 {
        return None;
    }
    let mut end = std::ptr::null_mut();
    let start = bytes.as_ptr() as *const c_char;
    let value = unsafe { strtoul(start, &mut end, 10) };
    if std::ptr::eq(end as *const c_char, start) {
        return None;
    }
    let consumed = unsafe { end.offset_from(start) as usize };
    Some((value as u64, consumed))
}

fn parse_signed(bytes: &[u8]) -> Option<(i64, usize)> {
    if bytes.is_empty() || bytes[0] == 0 {
        return None;
    }
    let mut end = std::ptr::null_mut::<c_char>();
    let start = bytes.as_ptr() as *const c_char;
    let value = unsafe { strtol(start, &mut end, 10) };
    if std::ptr::eq(end as *const c_char, start) {
        return None;
    }
    let consumed = unsafe { end.offset_from(start) as usize };
    Some((value as i64, consumed))
}

fn process_buffer(
    buffer: &mut [u8],
    length: usize,
    flags: u32,
    param1: i32,
    param2: i32,
) -> usize {
    let mut new_len = length;

    if buffer.is_empty() || length == 0 {
        return 0;
    }

    if (flags & 0x01) != 0 {
        let offset = param1 % (length as i32);
        if offset != 0 {
            rotate_buffer(buffer, length, offset);
        }
    }

    if (flags & 0x02) != 0 {
        let threshold = if param1 > 0 && param1 <= 255 {
            param1 as u8
        } else {
            3
        };
        new_len = compact_runs(buffer, new_len, threshold);
    }

    if (flags & 0x04) != 0 {
        let preserve = param2 != 0;
        new_len = remove_duplicates(buffer, new_len, preserve);
    }

    if (flags & 0x08) != 0 && new_len >= 2 {
        interleave_halves(buffer, new_len);
    }

    if (flags & 0x10) != 0 && new_len >= 4 {
        let seg_size = if param1 > 0 { param1 as usize } else { 4 };
        if seg_size <= new_len {
            reverse_segments(buffer, new_len, seg_size);
        }
    }

    new_len
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

    let mut temp = [0u8; 256];
    let chunk = usize::try_from(offset).unwrap().min(256);

    if (offset as usize) < len / 2 {
        let mut i = 0usize;
        while i < offset as usize {
            let copy_len = ((offset as usize) - i).min(chunk);
            temp[..copy_len].copy_from_slice(&buf[i..i + copy_len]);
            buf.copy_within(offset as usize..len, i);
            buf[len - offset as usize..len - offset as usize + copy_len]
                .copy_from_slice(&temp[..copy_len]);
            i += chunk;
        }
    } else {
        let shift = len - offset as usize;
        temp[..shift].copy_from_slice(&buf[..shift]);
        buf.copy_within(shift..len, 0);
        buf[offset as usize..offset as usize + shift].copy_from_slice(&temp[..shift]);
    }
}

fn compact_runs(buf: &mut [u8], mut len: usize, threshold: u8) -> usize {
    let mut read = 0usize;
    let mut write = 0usize;

    while read < len {
        let current = buf[read];
        let mut run_len = 1usize;

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

fn remove_duplicates(buf: &mut [u8], len: usize, preserve_order: bool) -> usize {
    if len <= 1 {
        return len;
    }

    if preserve_order {
        let mut write = 1usize;
        for i in 1..len {
            let mut j = 0usize;
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
        let mut seen = [0u8; 256];
        let mut write = 0usize;

        for i in 0..len {
            if seen[buf[i] as usize] == 0 {
                seen[buf[i] as usize] = 1;
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
    let mut temp = [0u8; 512];

    if half <= 256 {
        temp[..half].copy_from_slice(&buf[..half]);

        for i in 0..half {
            let value = buf[half + i];
            buf[i * 2 + 1] = value;
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

fn reverse_segments(buf: &mut [u8], len: usize, seg_size: usize) {
    if seg_size <= 1 || len < seg_size {
        return;
    }

    let num_segments = len / seg_size;
    let remainder = len % seg_size;

    for seg in 0..num_segments {
        let base = seg * seg_size;

        for i in 0..(seg_size / 2) {
            let left = base + i;
            let right = base + seg_size - 1 - i;
            let temp = buf[left];
            buf[left] = buf[right];
            buf[right] = temp;
        }
    }

    if remainder > 1 {
        let base = num_segments * seg_size;
        for i in 0..(remainder / 2) {
            buf.swap(base + i, base + remainder - 1 - i);
        }
    }
}
