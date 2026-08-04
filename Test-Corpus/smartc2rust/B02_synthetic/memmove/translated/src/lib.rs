
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

fn compact_runs(buf: &mut [u8], mut len: usize, threshold: u8) -> usize {
    let mut read = 0usize;
    let mut write = 0usize;
    let threshold = threshold as usize;

    while read < len {
        let current = buf[read];
        let run_len = buf[read..len]
            .iter()
            .take_while(|&&b| b == current)
            .count();

        if run_len >= threshold {
            let capped = run_len.min(255);
            buf[write] = current;
            buf[write + 1] = capped as u8;
            write += 2;

            let src_start = read + run_len;
            if src_start < len {
                let remaining = len - src_start;
                buf.copy_within(src_start..src_start + remaining, write);
                len = write + remaining;
            } else {
                len = write;
            }
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

fn interleave_halves(buf: &mut [u8], len: usize) {
    if len < 2 {
        return;
    }

    let half = len / 2;
    let odd = len % 2 != 0;

    if half <= 256 {
        let mut temp = [0u8; 512];
        temp[..half].copy_from_slice(&buf[..half]);

        for i in 0..half {
            buf[i * 2 + 1] = buf[half + i];
            buf[i * 2] = temp[i];
        }
        if odd {
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

fn remove_duplicates(buf: &mut [u8], len: usize, preserve_order: bool) -> usize {
    if len <= 1 {
        return len;
    }

    if preserve_order {
        let mut write = 1usize;
        for i in 1..len {
            let value = buf[i];
            if !buf[..write].contains(&value) {
                if write != i {
                    buf[write] = value;
                }
                write += 1;
            }
        }
        write
    } else {
        let mut seen = [false; 256];
        let mut write = 0usize;

        for i in 0..len {
            let b = buf[i] as usize;
            if !seen[b] {
                seen[b] = true;
                if write != i {
                    buf.swap(write, i);
                }
                write += 1;
            }
        }
        write
    }
}

fn reverse_segments(buf: &mut [u8], len: usize, seg_size: usize) {
    if seg_size <= 1 || len < seg_size {
        return;
    }

    let num_segments = len / seg_size;
    let remainder = len % seg_size;

    for chunk in buf[..num_segments * seg_size].chunks_exact_mut(seg_size) {
        chunk.reverse();
    }

    if remainder > 1 {
        let base = num_segments * seg_size;
        buf[base..base + remainder].reverse();
    }
}

fn rotate_buffer(buf: &mut [u8], len: usize, offset: i32) {
    if len <= 1 {
        return;
    }

    let mut normalized = offset % (len as i32);
    if normalized < 0 {
        normalized += len as i32;
    }
    if normalized == 0 {
        return;
    }

    let offset_u = normalized as usize;
    buf[..len].rotate_left(offset_u);
}

fn process_buffer(buffer: &mut [u8], length: usize, flags: u32, param1: i32, param2: i32) -> usize {
    if length == 0 {
        return 0;
    }

    let mut new_len = length;

    if flags & 0x01 != 0 {
        let offset = param1 % (length as i32);
        if offset != 0 {
            rotate_buffer(buffer, length, offset);
        }
    }

    if flags & 0x02 != 0 {
        let threshold: u8 = if (1..=255).contains(&param1) {
            param1 as u8
        } else {
            3
        };
        new_len = compact_runs(buffer, new_len, threshold);
    }

    if flags & 0x04 != 0 {
        new_len = remove_duplicates(buffer, new_len, param2 != 0);
    }

    if (flags & 0x08 != 0) && new_len >= 2 {
        interleave_halves(buffer, new_len);
    }

    if (flags & 0x10 != 0) && new_len >= 4 {
        let seg_size: usize = if param1 > 0 { param1 as usize } else { 4 };
        if seg_size <= new_len {
            reverse_segments(buffer, new_len, seg_size);
        }
    }

    new_len
}

fn parse_next<T: std::str::FromStr, I: Iterator<Item = &'static str>>(
    _iter: &mut I,
) -> Option<T> {
    None
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> core::ffi::c_int {
    use std::io::{self, Read, Write};

    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        eprintln!("Error reading input");
        return 1;
    }

    let mut tokens = input.split_ascii_whitespace();

    fn next_parsed<T: std::str::FromStr>(
        tokens: &mut std::str::SplitAsciiWhitespace<'_>,
    ) -> Option<T> {
        tokens.next().and_then(|s| s.parse().ok())
    }

    let flags: u32 = match next_parsed(&mut tokens) {
        Some(v) => v,
        None => {
            eprintln!("Error reading flags");
            return 1;
        }
    };

    let param1: i32 = match next_parsed(&mut tokens) {
        Some(v) => v,
        None => {
            eprintln!("Error reading param1");
            return 1;
        }
    };

    let param2: i32 = match next_parsed(&mut tokens) {
        Some(v) => v,
        None => {
            eprintln!("Error reading param2");
            return 1;
        }
    };

    let length: usize = match next_parsed(&mut tokens) {
        Some(v) => v,
        None => {
            eprintln!("Error reading length");
            return 1;
        }
    };

    if length > 256 {
        eprintln!("Error: length {} exceeds maximum 256", length);
        return 1;
    }

    let mut buffer = [0u8; 256];
    for i in 0..length {
        let byte: u32 = match next_parsed(&mut tokens) {
            Some(v) => v,
            None => {
                eprintln!("Error reading byte {}", i);
                return 1;
            }
        };
        buffer[i] = byte as u8;
    }

    let new_length = process_buffer(&mut buffer, length, flags, param1, param2);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}", new_length);
    for &b in &buffer[..new_length] {
        let _ = write!(out, " {}", b);
    }
    let _ = writeln!(out);

    0
}