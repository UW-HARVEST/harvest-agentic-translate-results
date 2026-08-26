// Rust translation of c_src/src/main.c
// Reproduces byte-identical output for the same input.

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

// Operation codes
const OP_COPY: i32 = 0;
const OP_REVERSE: i32 = 1;
const OP_MERGE: i32 = 2;
const OP_SPLIT: i32 = 3;
const OP_INTERLEAVE: i32 = 4;
const OP_ROTATE: i32 = 5;
const OP_CHECKSUM: i32 = 6;

// ==================== Token Reader (mimicking scanf("%d")) ====================

struct TokenReader {
    data: Vec<u8>,
    pos: usize,
}

impl TokenReader {
    fn new() -> io::Result<Self> {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf)?;
        Ok(TokenReader { data: buf, pos: 0 })
    }

    /// Reads the next integer in scanf("%d", ...) style. Skips whitespace and
    /// then parses an optional sign followed by digits. Returns None if no
    /// integer can be read (matching scanf's "did not match a single item").
    fn read_int(&mut self) -> Option<i64> {
        // Skip leading whitespace (scanf treats space, tab, newline, etc. as ws)
        while self.pos < self.data.len() && is_whitespace(self.data[self.pos]) {
            self.pos += 1;
        }
        if self.pos >= self.data.len() {
            return None;
        }

        let start = self.pos;
        let mut sign: i64 = 1;
        if self.data[self.pos] == b'-' {
            sign = -1;
            self.pos += 1;
        } else if self.data[self.pos] == b'+' {
            self.pos += 1;
        }

        let digit_start = self.pos;
        let mut value: i64 = 0;
        while self.pos < self.data.len() && self.data[self.pos].is_ascii_digit() {
            value = value
                .wrapping_mul(10)
                .wrapping_add((self.data[self.pos] - b'0') as i64);
            self.pos += 1;
        }

        if self.pos == digit_start {
            // No digits found: rewind and return None
            self.pos = start;
            return None;
        }

        Some(value.wrapping_mul(sign))
    }
}

fn is_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
}

// ==================== Helper Functions ====================

fn calculate_checksum(data: &[u8], length: usize) -> u32 {
    let mut sum: u32 = 0;
    for i in 0..length {
        sum = (sum << 3) ^ (data[i] as u32);
    }
    sum
}

fn validate_buffer(buf: &Buffer) -> bool {
    // NULL check from C is unrepresentable in safe Rust; references are non-null.
    if buf.length > 256 {
        eprintln!("Error: Buffer length {} exceeds maximum 256", buf.length);
        return false;
    }
    let expected = calculate_checksum(&buf.data, buf.length);
    if buf.checksum != expected {
        eprintln!(
            "Warning: Checksum mismatch. Expected {}, got {}",
            expected, buf.checksum
        );
    }
    true
}

fn init_buffer_array(initial_capacity: i32) -> Option<BufferArray> {
    if initial_capacity <= 0 {
        eprintln!("Error: Invalid capacity {}", initial_capacity);
        return None;
    }
    Some(BufferArray {
        buffers: vec![Buffer::new(); initial_capacity as usize],
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
        eprintln!(
            "Error: Merged length {} exceeds maximum",
            src1.length + src2.length
        );
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
        eprintln!(
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

fn buffer_interleave(src1: &Buffer, src2: &Buffer, dst: &mut Buffer) -> i32 {
    let max_len = if src1.length > src2.length {
        src1.length
    } else {
        src2.length
    };
    if src1.length + src2.length > 256 {
        eprintln!("Error: Interleaved length exceeds maximum");
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

fn buffer_rotate(buf: &mut Buffer, positions: i32) -> i32 {
    if buf.length == 0 || positions == 0 {
        return 0;
    }
    let mut p = positions % (buf.length as i32);
    if p < 0 {
        p += buf.length as i32;
    }
    let p = p as usize;

    let mut temp = [0u8; 256];
    temp[..buf.length].copy_from_slice(&buf.data[..buf.length]);

    // memcpy(buf->data, temp + positions, buf->length - positions);
    let first_chunk = buf.length - p;
    buf.data[..first_chunk].copy_from_slice(&temp[p..p + first_chunk]);
    // memcpy(buf->data + (buf->length - positions), temp, positions);
    buf.data[first_chunk..first_chunk + p].copy_from_slice(&temp[..p]);

    buf.checksum = calculate_checksum(&buf.data, buf.length);
    0
}

// ==================== Input/Output Functions ====================

fn read_buffer(reader: &mut TokenReader, buf: &mut Buffer) -> i32 {
    let length = match reader.read_int() {
        Some(v) => v as i32,
        None => {
            eprintln!("Error: Failed to read buffer length");
            return -1;
        }
    };

    if length < 0 || length > 256 {
        eprintln!("Error: Invalid buffer length {}", length);
        return -1;
    }

    buf.length = length as usize;
    for i in 0..buf.length {
        match reader.read_int() {
            Some(v) => {
                buf.data[i] = v as u8;
            }
            None => {
                eprintln!("Error: Failed to read byte {}", i);
                return -1;
            }
        }
    }

    buf.checksum = calculate_checksum(&buf.data, buf.length);
    0
}

fn write_buffer(out: &mut impl Write, buf: &Buffer) {
    write!(out, "{}", buf.length).unwrap();
    for i in 0..buf.length {
        write!(out, " {}", buf.data[i]).unwrap();
    }
    writeln!(out).unwrap();
}

// ==================== Main ====================

fn run() -> i32 {
    let mut reader = match TokenReader::new() {
        Ok(r) => r,
        Err(_) => {
            eprintln!("Error: Failed to read operation");
            return 1;
        }
    };

    let stdout = io::stdout();
    let mut out = stdout.lock();

    let operation = match reader.read_int() {
        Some(v) => v as i32,
        None => {
            eprintln!("Error: Failed to read operation");
            return 1;
        }
    };

    let buffer_count = match reader.read_int() {
        Some(v) => v as i32,
        None => {
            eprintln!("Error: Failed to read buffer count");
            return 1;
        }
    };

    if buffer_count <= 0 || buffer_count > 100 {
        eprintln!("Error: Invalid buffer count {}", buffer_count);
        return 1;
    }

    let mut buffers = match init_buffer_array(buffer_count) {
        Some(b) => b,
        None => return 1,
    };

    for i in 0..buffer_count as usize {
        if read_buffer(&mut reader, &mut buffers.buffers[i]) != 0 {
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
                eprintln!("Error: Copy needs at least 2 buffers");
                result = -1;
            }
        }
        x if x == OP_REVERSE => {
            for i in 0..buffer_count as usize {
                result = buffer_reverse(&mut buffers.buffers[i]);
                if result != 0 {
                    break;
                }
                write_buffer(&mut out, &buffers.buffers[i]);
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
                eprintln!("Error: Merge needs at least 2 buffers");
                result = -1;
            }
        }
        x if x == OP_SPLIT => {
            if buffer_count >= 1 {
                let split_pos = match reader.read_int() {
                    Some(v) => v as i32,
                    None => {
                        eprintln!("Error: Failed to read split position");
                        result = -1;
                        return finalize(result, &mut buffers);
                    }
                };
                // C uses size_t for split_pos parameter, so negative values become huge
                let split_pos_usz = split_pos as usize; // matches C's implicit conversion to size_t
                let mut part1 = Buffer::new();
                let mut part2 = Buffer::new();
                let src = buffers.buffers[0];
                result = buffer_split(&src, split_pos_usz, &mut part1, &mut part2);
                if result == 0 {
                    write_buffer(&mut out, &part1);
                    write_buffer(&mut out, &part2);
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
                eprintln!("Error: Interleave needs at least 2 buffers");
                result = -1;
            }
        }
        x if x == OP_ROTATE => {
            let positions = match reader.read_int() {
                Some(v) => v as i32,
                None => {
                    eprintln!("Error: Failed to read rotation amount");
                    result = -1;
                    return finalize(result, &mut buffers);
                }
            };
            for i in 0..buffer_count as usize {
                result = buffer_rotate(&mut buffers.buffers[i], positions);
                if result != 0 {
                    break;
                }
                write_buffer(&mut out, &buffers.buffers[i]);
            }
        }
        x if x == OP_CHECKSUM => {
            for i in 0..buffer_count as usize {
                writeln!(out, "{}", buffers.buffers[i].checksum).unwrap();
            }
        }
        _ => {
            eprintln!("Error: Unknown operation {}", operation);
            result = -1;
        }
    }

    finalize(result, &mut buffers)
}

fn finalize(result: i32, _buffers: &mut BufferArray) -> i32 {
    if result != 0 {
        1
    } else {
        0
    }
}

fn main() -> ExitCode {
    let code = run();
    // Ensure stdout is flushed before exiting
    let _ = io::stdout().flush();
    ExitCode::from(code as u8)
}
