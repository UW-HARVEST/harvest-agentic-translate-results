use std::fmt::Write as _;
use std::io::{self, Read};
use std::process::ExitCode;

const INPUT_CAPACITY: usize = 256;
const BUFFER_CAPACITY: usize = INPUT_CAPACITY * 2;

struct Scanner {
    input: Vec<u8>,
    position: usize,
}

impl Scanner {
    fn from_stdin() -> Self {
        let mut input = Vec::new();
        let _ = io::stdin().read_to_end(&mut input);
        Self { input, position: 0 }
    }

    fn skip_whitespace(&mut self) {
        while self.position < self.input.len()
            && matches!(
                self.input[self.position],
                b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r'
            )
        {
            self.position += 1;
        }
    }

    fn sign_and_digits(&mut self) -> Option<(bool, u64, bool)> {
        self.skip_whitespace();

        let negative = match self.input.get(self.position) {
            Some(b'+') => {
                self.position += 1;
                false
            }
            Some(b'-') => {
                self.position += 1;
                true
            }
            _ => false,
        };

        let start = self.position;
        let mut value = 0_u64;
        let mut overflowed = false;
        while let Some(&byte) = self.input.get(self.position) {
            if !byte.is_ascii_digit() {
                break;
            }
            let digit = u64::from(byte - b'0');
            match value.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(next) if !overflowed => value = next,
                _ => {
                    value = u64::MAX;
                    overflowed = true;
                }
            }
            self.position += 1;
        }

        (self.position != start).then_some((negative, value, overflowed))
    }

    fn scan_unsigned(&mut self) -> Option<u64> {
        let (negative, magnitude, overflowed) = self.sign_and_digits()?;
        if overflowed {
            Some(u64::MAX)
        } else if negative {
            Some(0_u64.wrapping_sub(magnitude))
        } else {
            Some(magnitude)
        }
    }

    fn scan_u32(&mut self) -> Option<u32> {
        self.scan_unsigned().map(|value| value as u32)
    }

    fn scan_usize(&mut self) -> Option<usize> {
        self.scan_unsigned().map(|value| value as usize)
    }

    fn scan_i32(&mut self) -> Option<i32> {
        let (negative, magnitude, _) = self.sign_and_digits()?;
        let value = if negative {
            if magnitude >= (1_u64 << 63) {
                i64::MIN
            } else {
                -(magnitude as i64)
            }
        } else if magnitude > i64::MAX as u64 {
            i64::MAX
        } else {
            magnitude as i64
        };
        Some(value as i32)
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
    let mut temp = [0_u8; INPUT_CAPACITY];
    let chunk = offset.min(INPUT_CAPACITY);

    if offset < len / 2 {
        let mut i = 0;
        while i < offset {
            let copy_len = (offset - i).min(chunk);
            temp[..copy_len].copy_from_slice(&buf[i..i + copy_len]);
            buf.copy_within(offset..len, i);
            buf[len - offset..len].copy_from_slice(&temp[..copy_len]);
            i += chunk;
        }
    } else {
        let shift = len - offset;
        temp[..shift].copy_from_slice(&buf[..shift]);
        buf.copy_within(shift..len, 0);
        buf[offset..offset + shift].copy_from_slice(&temp[..shift]);
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
            if run_len > u8::MAX as usize {
                run_len = u8::MAX as usize;
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
        let mut seen = [0_u8; 256];
        let mut write = 0;

        for i in 0..len {
            let value = buf[i] as usize;
            if seen[value] == 0 {
                seen[value] = 1;
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
    let mut temp = [0_u8; BUFFER_CAPACITY];

    if half <= INPUT_CAPACITY {
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
            let left = base + i;
            let right = base + seg_size - 1 - i;
            buf.swap(left, right);
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

fn run() -> Result<(), String> {
    let mut scanner = Scanner::from_stdin();

    let flags = scanner
        .scan_u32()
        .ok_or_else(|| "Error reading flags".to_owned())?;
    let param1 = scanner
        .scan_i32()
        .ok_or_else(|| "Error reading param1".to_owned())?;
    let param2 = scanner
        .scan_i32()
        .ok_or_else(|| "Error reading param2".to_owned())?;
    let length = scanner
        .scan_usize()
        .ok_or_else(|| "Error reading length".to_owned())?;

    if length > INPUT_CAPACITY {
        return Err(format!(
            "Error: length {length} exceeds maximum {INPUT_CAPACITY}"
        ));
    }

    let mut buffer = [0_u8; BUFFER_CAPACITY];
    for (i, byte) in buffer.iter_mut().take(length).enumerate() {
        let value = scanner
            .scan_u32()
            .ok_or_else(|| format!("Error reading byte {i}"))?;
        *byte = value as u8;
    }

    let new_length = process_buffer(&mut buffer, length, flags, param1, param2);
    let mut output = new_length.to_string();
    for byte in &buffer[..new_length] {
        write!(output, " {byte}").unwrap();
    }
    output.push('\n');
    print!("{output}");

    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
