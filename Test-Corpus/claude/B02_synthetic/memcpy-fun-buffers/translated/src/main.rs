// Translation of c_src/src/main.c to Rust.
// Goal: byte-identical output.

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

// Operation constants (mirroring the C enum values).
const OP_COPY: i32 = 0;
const OP_REVERSE: i32 = 1;
const OP_MERGE: i32 = 2;
const OP_SPLIT: i32 = 3;
const OP_INTERLEAVE: i32 = 4;
const OP_ROTATE: i32 = 5;
const OP_CHECKSUM: i32 = 6;

// ==================== scanf-style integer tokenizer ====================

struct Scanner {
    data: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn from_stdin() -> Self {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf).unwrap_or(0);
        Scanner { data: buf, pos: 0 }
    }

    // Reproduce scanf("%d", ...) returning Some(value) on success,
    // None when no value can be parsed (e.g. EOF or non-numeric).
    fn read_int(&mut self) -> Option<i64> {
        // Skip leading whitespace (matches C's isspace for the "C" locale:
        // ' ', '\t', '\n', '\v', '\f', '\r').
        while self.pos < self.data.len() {
            let c = self.data[self.pos];
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
                self.pos += 1;
            } else {
                break;
            }
        }

        if self.pos >= self.data.len() {
            return None;
        }

        let start = self.pos;
        let mut negative = false;

        // Optional sign
        if self.data[self.pos] == b'+' {
            self.pos += 1;
        } else if self.data[self.pos] == b'-' {
            negative = true;
            self.pos += 1;
        }

        // Must have at least one digit
        let digits_start = self.pos;
        while self.pos < self.data.len() {
            let c = self.data[self.pos];
            if c.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }

        if self.pos == digits_start {
            // No digits found; rewind to start so behavior matches scanf
            // failure (scanf may consume the sign on failure, but in practice
            // we only need correct success behavior here).
            self.pos = start;
            return None;
        }

        let s = std::str::from_utf8(&self.data[digits_start..self.pos]).ok()?;
        // Use i128 to avoid overflow for large inputs, then narrow.
        let mut val: i128 = 0;
        for ch in s.bytes() {
            val = val.saturating_mul(10).saturating_add((ch - b'0') as i128);
        }
        if negative {
            val = -val;
        }
        // Truncate to i32 range (scanf %d stores into int).
        let v = val as i32;
        Some(v as i64)
    }
}

// ==================== stderr helpers ====================

fn eprint_str(s: &str) {
    let _ = io::stderr().write_all(s.as_bytes());
}

// ==================== Helper Functions ====================

fn calculate_checksum(data: &[u8], length: usize) -> u32 {
    let mut sum: u32 = 0;
    for i in 0..length {
        sum = sum.wrapping_shl(3) ^ (data[i] as u32);
    }
    sum
}

fn validate_buffer(buf: &Buffer) -> bool {
    if buf.length > 256 {
        eprint_str(&format!(
            "Error: Buffer length {} exceeds maximum 256\n",
            buf.length
        ));
        return false;
    }
    let expected = calculate_checksum(&buf.data, buf.length);
    if buf.checksum != expected {
        eprint_str(&format!(
            "Warning: Checksum mismatch. Expected {}, got {}\n",
            expected, buf.checksum
        ));
    }
    true
}

fn init_buffer_array(initial_capacity: i32) -> Option<BufferArray> {
    if initial_capacity <= 0 {
        eprint_str(&format!("Error: Invalid capacity {}\n", initial_capacity));
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

fn buffer_copy(src: &Buffer, dst: &mut Buffer) -> i32 {
    if !validate_buffer(src) {
        return -1;
    }

    dst.data[..src.length].copy_from_slice(&src.data[..src.length]);
    dst.length = src.length;
    dst.checksum = calculate_checksum(&dst.data, dst.length);

    0
}

fn buffer_reverse(buf: &mut Buffer) -> i32 {
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

fn buffer_merge(src1: &Buffer, src2: &Buffer, dst: &mut Buffer) -> i32 {
    if src1.length + src2.length > 256 {
        eprint_str(&format!(
            "Error: Merged length {} exceeds maximum\n",
            src1.length + src2.length
        ));
        return -1;
    }

    dst.data[..src1.length].copy_from_slice(&src1.data[..src1.length]);
    dst.data[src1.length..src1.length + src2.length]
        .copy_from_slice(&src2.data[..src2.length]);

    dst.length = src1.length + src2.length;
    dst.checksum = calculate_checksum(&dst.data, dst.length);

    0
}

fn buffer_split(src: &Buffer, split_pos: usize, dst1: &mut Buffer, dst2: &mut Buffer) -> i32 {
    if split_pos > src.length {
        eprint_str(&format!(
            "Error: Split position {} exceeds length {}\n",
            split_pos, src.length
        ));
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

fn buffer_interleave(src1: &Buffer, src2: &Buffer, dst: &mut Buffer) -> i32 {
    let max_len = if src1.length > src2.length {
        src1.length
    } else {
        src2.length
    };
    if src1.length + src2.length > 256 {
        eprint_str("Error: Interleaved length exceeds maximum\n");
        return -1;
    }

    let mut dst_pos: usize = 0;
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

fn buffer_rotate(buf: &mut Buffer, mut positions: i32) -> i32 {
    if buf.length == 0 || positions == 0 {
        return 0;
    }

    // Replicate C's `positions = positions % (int)buf->length;` followed by
    // adjustment for negative results. buf.length fits in i32 since it's <= 256.
    positions = positions % (buf.length as i32);
    if positions < 0 {
        positions += buf.length as i32;
    }

    let positions_us = positions as usize;

    let mut temp = [0u8; 256];
    temp[..buf.length].copy_from_slice(&buf.data[..buf.length]);

    // memcpy(buf->data, temp + positions, buf->length - positions);
    let first_chunk = buf.length - positions_us;
    if first_chunk > 0 {
        let src_slice = &temp[positions_us..positions_us + first_chunk];
        buf.data[..first_chunk].copy_from_slice(src_slice);
    }
    // memcpy(buf->data + (buf->length - positions), temp, positions);
    if positions_us > 0 {
        let dst_start = buf.length - positions_us;
        buf.data[dst_start..dst_start + positions_us].copy_from_slice(&temp[..positions_us]);
    }

    buf.checksum = calculate_checksum(&buf.data, buf.length);

    0
}

// ==================== Input/Output Functions ====================

fn read_buffer(scanner: &mut Scanner, buf: &mut Buffer) -> i32 {
    let length = match scanner.read_int() {
        Some(v) => v as i32,
        None => {
            eprint_str("Error: Failed to read buffer length\n");
            return -1;
        }
    };

    if length < 0 || length > 256 {
        eprint_str(&format!("Error: Invalid buffer length {}\n", length));
        return -1;
    }

    buf.length = length as usize;
    for i in 0..buf.length {
        let byte = match scanner.read_int() {
            Some(v) => v as i32,
            None => {
                eprint_str(&format!("Error: Failed to read byte {}\n", i));
                return -1;
            }
        };
        buf.data[i] = byte as u8;
    }

    buf.checksum = calculate_checksum(&buf.data, buf.length);
    0
}

fn write_buffer(out: &mut impl Write, buf: &Buffer) {
    let _ = write!(out, "{}", buf.length);
    for i in 0..buf.length {
        let _ = write!(out, " {}", buf.data[i] as u32);
    }
    let _ = writeln!(out);
}

// ==================== Main ====================

fn run() -> i32 {
    let mut scanner = Scanner::from_stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    let operation = match scanner.read_int() {
        Some(v) => v as i32,
        None => {
            eprint_str("Error: Failed to read operation\n");
            return 1;
        }
    };

    let buffer_count = match scanner.read_int() {
        Some(v) => v as i32,
        None => {
            eprint_str("Error: Failed to read buffer count\n");
            return 1;
        }
    };

    if buffer_count <= 0 || buffer_count > 100 {
        eprint_str(&format!("Error: Invalid buffer count {}\n", buffer_count));
        return 1;
    }

    let mut buffers = match init_buffer_array(buffer_count) {
        Some(b) => b,
        None => return 1,
    };

    for i in 0..buffer_count {
        if read_buffer(&mut scanner, &mut buffers.buffers[i as usize]) != 0 {
            return 1;
        }
        buffers.count += 1;
    }

    let mut result: i32 = 0;

    match operation {
        x if x == OP_COPY => {
            if buffer_count >= 2 {
                let mut temp = Buffer::new();
                let src = buffers.buffers[0];
                result = buffer_copy(&src, &mut temp);
                if result == 0 {
                    write_buffer(&mut out, &temp);
                }
            } else {
                eprint_str("Error: Copy needs at least 2 buffers\n");
                result = -1;
            }
        }

        x if x == OP_REVERSE => {
            for i in 0..buffer_count {
                result = buffer_reverse(&mut buffers.buffers[i as usize]);
                if result != 0 {
                    break;
                }
                let b = buffers.buffers[i as usize];
                write_buffer(&mut out, &b);
            }
        }

        x if x == OP_MERGE => {
            if buffer_count >= 2 {
                let mut merged = Buffer::new();
                let s1 = buffers.buffers[0];
                let s2 = buffers.buffers[1];
                result = buffer_merge(&s1, &s2, &mut merged);
                if result == 0 {
                    write_buffer(&mut out, &merged);
                }
            } else {
                eprint_str("Error: Merge needs at least 2 buffers\n");
                result = -1;
            }
        }

        x if x == OP_SPLIT => {
            if buffer_count >= 1 {
                match scanner.read_int() {
                    None => {
                        eprint_str("Error: Failed to read split position\n");
                        result = -1;
                    }
                    Some(v) => {
                        let split_pos = v as i32;
                        // C casts int to size_t. On 64-bit size_t platforms,
                        // a negative int becomes a very large size_t.
                        let split_pos_us: usize = split_pos as i64 as u64 as usize;

                        let mut part1 = Buffer::new();
                        let mut part2 = Buffer::new();
                        let src = buffers.buffers[0];
                        result = buffer_split(&src, split_pos_us, &mut part1, &mut part2);
                        if result == 0 {
                            write_buffer(&mut out, &part1);
                            write_buffer(&mut out, &part2);
                        }
                    }
                }
            }
        }

        x if x == OP_INTERLEAVE => {
            if buffer_count >= 2 {
                let mut interleaved = Buffer::new();
                let s1 = buffers.buffers[0];
                let s2 = buffers.buffers[1];
                result = buffer_interleave(&s1, &s2, &mut interleaved);
                if result == 0 {
                    write_buffer(&mut out, &interleaved);
                }
            } else {
                eprint_str("Error: Interleave needs at least 2 buffers\n");
                result = -1;
            }
        }

        x if x == OP_ROTATE => {
            match scanner.read_int() {
                None => {
                    eprint_str("Error: Failed to read rotation amount\n");
                    result = -1;
                }
                Some(v) => {
                    let positions = v as i32;
                    for i in 0..buffer_count {
                        result = buffer_rotate(&mut buffers.buffers[i as usize], positions);
                        if result != 0 {
                            break;
                        }
                        let b = buffers.buffers[i as usize];
                        write_buffer(&mut out, &b);
                    }
                }
            }
        }

        x if x == OP_CHECKSUM => {
            for i in 0..buffer_count {
                let _ = writeln!(out, "{}", buffers.buffers[i as usize].checksum);
            }
        }

        other => {
            eprint_str(&format!("Error: Unknown operation {}\n", other));
            result = -1;
        }
    }

    let _ = out.flush();
    drop(buffers);
    if result != 0 {
        1
    } else {
        0
    }
}

fn main() -> ExitCode {
    ExitCode::from(run() as u8)
}
