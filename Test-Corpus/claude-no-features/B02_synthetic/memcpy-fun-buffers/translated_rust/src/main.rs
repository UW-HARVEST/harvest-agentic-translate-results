// Translated from c_src/src/main.c
// Goal: byte-identical output

use std::io::{self, Read, Write};
use std::process::ExitCode;

// ==================== Data Structures ====================

#[derive(Clone, Copy)]
struct Buffer {
    data: [u8; 256],
    length: usize,
    checksum: u32,
}

impl Buffer {
    fn new() -> Self {
        Buffer {
            data: [0u8; 256],
            length: 0,
            checksum: 0,
        }
    }
}

struct BufferArray {
    buffers: Vec<Buffer>,
    count: i32,
    capacity: i32,
}

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(i32)]
#[allow(dead_code)]
enum Operation {
    Copy = 0,
    Reverse = 1,
    Merge = 2,
    Split = 3,
    Interleave = 4,
    Rotate = 5,
    Checksum = 6,
}

// ==================== Token-based stdin reader (mimics scanf("%d")) ====================

struct TokenReader {
    data: Vec<u8>,
    pos: usize,
}

impl TokenReader {
    fn new() -> Self {
        let mut buf = Vec::new();
        // Read all of stdin into buffer (mimics scanf which reads across newlines)
        let _ = io::stdin().read_to_end(&mut buf);
        TokenReader { data: buf, pos: 0 }
    }

    // Read next integer token. Returns None if cannot parse (mimics scanf returning != 1).
    fn read_int(&mut self) -> Option<i64> {
        // Skip whitespace
        while self.pos < self.data.len() && is_whitespace(self.data[self.pos]) {
            self.pos += 1;
        }
        if self.pos >= self.data.len() {
            return None;
        }

        let start = self.pos;
        // Optional sign
        if self.data[self.pos] == b'+' || self.data[self.pos] == b'-' {
            self.pos += 1;
        }

        let digits_start = self.pos;
        while self.pos < self.data.len() && self.data[self.pos].is_ascii_digit() {
            self.pos += 1;
        }

        if self.pos == digits_start {
            // No digits found; revert and fail
            self.pos = start;
            return None;
        }

        let s = std::str::from_utf8(&self.data[start..self.pos]).ok()?;
        // C's scanf %d stores into int, which is typically 32-bit. If the value
        // overflows, behavior is undefined. We parse as i64 and let the caller
        // truncate as needed; if not parseable, fail.
        match s.parse::<i64>() {
            Ok(v) => Some(v),
            Err(_) => {
                // Try to handle overflow: scanf would consume digits but result is undefined.
                // For our purposes, attempt to truncate via wrapping.
                None
            }
        }
    }
}

fn is_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
}

// ==================== Helper Functions ====================

fn calculate_checksum(data: &[u8], length: usize) -> u32 {
    let mut sum: u32 = 0;
    for i in 0..length {
        sum = sum.wrapping_shl(3) ^ (data[i] as u32);
    }
    sum
}

fn validate_buffer(buf: &Buffer, stderr: &mut impl Write) -> bool {
    if buf.length > 256 {
        let _ = writeln!(
            stderr,
            "Error: Buffer length {} exceeds maximum 256",
            buf.length
        );
        return false;
    }
    let expected = calculate_checksum(&buf.data, buf.length);
    if buf.checksum != expected {
        let _ = writeln!(
            stderr,
            "Warning: Checksum mismatch. Expected {}, got {}",
            expected, buf.checksum
        );
    }
    true
}

fn init_buffer_array(initial_capacity: i32, stderr: &mut impl Write) -> Option<BufferArray> {
    if initial_capacity <= 0 {
        let _ = writeln!(stderr, "Error: Invalid capacity {}", initial_capacity);
        return None;
    }

    let mut buffers = Vec::with_capacity(initial_capacity as usize);
    for _ in 0..initial_capacity {
        buffers.push(Buffer::new());
    }

    Some(BufferArray {
        buffers,
        count: 0,
        capacity: initial_capacity,
    })
}

// ==================== Core Buffer Operations ====================

fn buffer_copy(src: &Buffer, dst: &mut Buffer, stderr: &mut impl Write) -> i32 {
    if !validate_buffer(src, stderr) {
        return -1;
    }
    // memcpy of src->length bytes from src->data to dst->data
    dst.data[..src.length].copy_from_slice(&src.data[..src.length]);
    dst.length = src.length;
    dst.checksum = calculate_checksum(&dst.data, dst.length);
    0
}

fn buffer_reverse(buf: &mut Buffer, _stderr: &mut impl Write) -> i32 {
    if buf.length == 0 {
        return 0;
    }
    let mut temp = [0u8; 256];
    temp[..buf.length].copy_from_slice(&buf.data[..buf.length]);
    for i in 0..buf.length {
        buf.data[i] = temp[buf.length - 1 - i];
    }
    buf.checksum = calculate_checksum(&buf.data, buf.length);
    0
}

fn buffer_merge(src1: &Buffer, src2: &Buffer, dst: &mut Buffer, stderr: &mut impl Write) -> i32 {
    if src1.length + src2.length > 256 {
        let _ = writeln!(
            stderr,
            "Error: Merged length {} exceeds maximum",
            src1.length + src2.length
        );
        return -1;
    }
    dst.data[..src1.length].copy_from_slice(&src1.data[..src1.length]);
    dst.data[src1.length..src1.length + src2.length].copy_from_slice(&src2.data[..src2.length]);
    dst.length = src1.length + src2.length;
    dst.checksum = calculate_checksum(&dst.data, dst.length);
    0
}

fn buffer_split(
    src: &Buffer,
    split_pos: usize,
    dst1: &mut Buffer,
    dst2: &mut Buffer,
    stderr: &mut impl Write,
) -> i32 {
    if split_pos > src.length {
        let _ = writeln!(
            stderr,
            "Error: Split position {} exceeds length {}",
            split_pos, src.length
        );
        return -1;
    }
    if split_pos > 0 {
        dst1.data[..split_pos].copy_from_slice(&src.data[..split_pos]);
    }
    dst1.length = split_pos;
    dst1.checksum = calculate_checksum(&dst1.data, dst1.length);

    let remaining = src.length - split_pos;
    if remaining > 0 {
        dst2.data[..remaining].copy_from_slice(&src.data[split_pos..split_pos + remaining]);
    }
    dst2.length = remaining;
    dst2.checksum = calculate_checksum(&dst2.data, dst2.length);

    0
}

fn buffer_interleave(
    src1: &Buffer,
    src2: &Buffer,
    dst: &mut Buffer,
    stderr: &mut impl Write,
) -> i32 {
    let max_len = if src1.length > src2.length {
        src1.length
    } else {
        src2.length
    };
    if src1.length + src2.length > 256 {
        let _ = writeln!(stderr, "Error: Interleaved length exceeds maximum");
        return -1;
    }
    let mut dst_pos = 0usize;
    for i in 0..max_len {
        if i < src1.length {
            dst.data[dst_pos] = src1.data[i];
            dst_pos += 1;
        }
        if i < src2.length {
            dst.data[dst_pos] = src2.data[i];
            dst_pos += 1;
        }
    }
    dst.length = dst_pos;
    dst.checksum = calculate_checksum(&dst.data, dst.length);
    0
}

fn buffer_rotate(buf: &mut Buffer, positions: i32, _stderr: &mut impl Write) -> i32 {
    if buf.length == 0 || positions == 0 {
        return 0;
    }
    // positions = positions % (int)buf->length;
    // C truncated modulo for signed int
    let len_i = buf.length as i32;
    let mut p = positions % len_i;
    if p < 0 {
        // C: positions += buf->length;  (size_t). The result is implicitly converted in addition.
        // In C: int + size_t -> size_t. positions stored back into int. With positions in (-len, 0)
        // and len <= 256, this is well-defined for our typical inputs.
        p = p.wrapping_add(buf.length as i32);
    }
    let pu = p as usize;

    let mut temp = [0u8; 256];
    temp[..buf.length].copy_from_slice(&buf.data[..buf.length]);

    // memcpy(buf->data, temp + positions, buf->length - positions);
    let first_len = buf.length - pu;
    buf.data[..first_len].copy_from_slice(&temp[pu..pu + first_len]);
    // memcpy(buf->data + (buf->length - positions), temp, positions);
    buf.data[first_len..first_len + pu].copy_from_slice(&temp[..pu]);

    buf.checksum = calculate_checksum(&buf.data, buf.length);
    0
}

// ==================== Input/Output Functions ====================

fn read_buffer(reader: &mut TokenReader, buf: &mut Buffer, stderr: &mut impl Write) -> i32 {
    let length = match reader.read_int() {
        Some(v) => v as i32, // truncate like scanf to int
        None => {
            let _ = writeln!(stderr, "Error: Failed to read buffer length");
            return -1;
        }
    };

    if length < 0 || length > 256 {
        let _ = writeln!(stderr, "Error: Invalid buffer length {}", length);
        return -1;
    }

    buf.length = length as usize;
    for i in 0..buf.length {
        let byte = match reader.read_int() {
            Some(v) => v as i32,
            None => {
                let _ = writeln!(stderr, "Error: Failed to read byte {}", i);
                return -1;
            }
        };
        buf.data[i] = byte as u8;
    }
    buf.checksum = calculate_checksum(&buf.data, buf.length);
    0
}

fn write_buffer(buf: &Buffer, stdout: &mut impl Write) {
    let _ = write!(stdout, "{}", buf.length);
    for i in 0..buf.length {
        let _ = write!(stdout, " {}", buf.data[i]);
    }
    let _ = writeln!(stdout);
}

// ==================== Main Function ====================

fn run() -> i32 {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let stderr = io::stderr();
    let mut stderr = stderr.lock();

    let mut reader = TokenReader::new();

    let operation = match reader.read_int() {
        Some(v) => v as i32,
        None => {
            let _ = writeln!(stderr, "Error: Failed to read operation");
            return 1;
        }
    };

    let buffer_count = match reader.read_int() {
        Some(v) => v as i32,
        None => {
            let _ = writeln!(stderr, "Error: Failed to read buffer count");
            return 1;
        }
    };

    if buffer_count <= 0 || buffer_count > 100 {
        let _ = writeln!(stderr, "Error: Invalid buffer count {}", buffer_count);
        return 1;
    }

    let mut buffers = match init_buffer_array(buffer_count, &mut stderr) {
        Some(b) => b,
        None => return 1,
    };

    for i in 0..buffer_count {
        if read_buffer(&mut reader, &mut buffers.buffers[i as usize], &mut stderr) != 0 {
            return 1;
        }
        buffers.count += 1;
    }
    let _ = buffers.capacity; // keep field used

    let mut result: i32 = 0;
    match operation {
        x if x == Operation::Copy as i32 => {
            if buffer_count >= 2 {
                let mut temp = Buffer::new();
                let src = buffers.buffers[0];
                result = buffer_copy(&src, &mut temp, &mut stderr);
                if result == 0 {
                    write_buffer(&temp, &mut stdout);
                }
            } else {
                let _ = writeln!(stderr, "Error: Copy needs at least 2 buffers");
                result = -1;
            }
        }
        x if x == Operation::Reverse as i32 => {
            for i in 0..buffer_count {
                result = buffer_reverse(&mut buffers.buffers[i as usize], &mut stderr);
                if result != 0 {
                    break;
                }
                write_buffer(&buffers.buffers[i as usize], &mut stdout);
            }
        }
        x if x == Operation::Merge as i32 => {
            if buffer_count >= 2 {
                let mut merged = Buffer::new();
                let src1 = buffers.buffers[0];
                let src2 = buffers.buffers[1];
                result = buffer_merge(&src1, &src2, &mut merged, &mut stderr);
                if result == 0 {
                    write_buffer(&merged, &mut stdout);
                }
            } else {
                let _ = writeln!(stderr, "Error: Merge needs at least 2 buffers");
                result = -1;
            }
        }
        x if x == Operation::Split as i32 => {
            if buffer_count >= 1 {
                let split_pos_opt = reader.read_int();
                match split_pos_opt {
                    None => {
                        let _ = writeln!(stderr, "Error: Failed to read split position");
                        result = -1;
                    }
                    Some(sp) => {
                        let split_pos = sp as i32;
                        // C uses size_t for split_pos parameter; int -> size_t conversion is
                        // well-defined (modulo 2^N). Negative values become huge, exceeding
                        // src->length and triggering the error path with the "size_t" form.
                        let split_pos_sz: usize = split_pos as usize; // wrapping cast like (size_t)int
                        let mut part1 = Buffer::new();
                        let mut part2 = Buffer::new();
                        let src = buffers.buffers[0];
                        result = buffer_split(&src, split_pos_sz, &mut part1, &mut part2, &mut stderr);
                        if result == 0 {
                            write_buffer(&part1, &mut stdout);
                            write_buffer(&part2, &mut stdout);
                        }
                    }
                }
            }
        }
        x if x == Operation::Interleave as i32 => {
            if buffer_count >= 2 {
                let mut interleaved = Buffer::new();
                let src1 = buffers.buffers[0];
                let src2 = buffers.buffers[1];
                result = buffer_interleave(&src1, &src2, &mut interleaved, &mut stderr);
                if result == 0 {
                    write_buffer(&interleaved, &mut stdout);
                }
            } else {
                let _ = writeln!(stderr, "Error: Interleave needs at least 2 buffers");
                result = -1;
            }
        }
        x if x == Operation::Rotate as i32 => {
            let positions_opt = reader.read_int();
            match positions_opt {
                None => {
                    let _ = writeln!(stderr, "Error: Failed to read rotation amount");
                    result = -1;
                }
                Some(p) => {
                    let positions = p as i32;
                    for i in 0..buffer_count {
                        result = buffer_rotate(
                            &mut buffers.buffers[i as usize],
                            positions,
                            &mut stderr,
                        );
                        if result != 0 {
                            break;
                        }
                        write_buffer(&buffers.buffers[i as usize], &mut stdout);
                    }
                }
            }
        }
        x if x == Operation::Checksum as i32 => {
            for i in 0..buffer_count {
                let _ = writeln!(stdout, "{}", buffers.buffers[i as usize].checksum);
            }
        }
        _ => {
            let _ = writeln!(stderr, "Error: Unknown operation {}", operation);
            result = -1;
        }
    }

    if result != 0 {
        1
    } else {
        0
    }
}

fn main() -> ExitCode {
    ExitCode::from(run() as u8)
}
