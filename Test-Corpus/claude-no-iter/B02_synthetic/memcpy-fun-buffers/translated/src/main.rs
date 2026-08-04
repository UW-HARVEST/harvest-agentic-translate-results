// Rust translation of c_src/src/main.c — produces byte-identical output.

use std::io::{self, Read, Write, BufWriter};
use std::process::ExitCode;

// ==================== Data Structures ====================

#[derive(Clone)]
struct Buffer {
    data: [u8; 256],
    length: usize,
    checksum: u32,
}

impl Buffer {
    fn new() -> Self {
        Buffer { data: [0u8; 256], length: 0, checksum: 0 }
    }
}

struct BufferArray {
    buffers: Vec<Buffer>,
    count: i32,
    #[allow(dead_code)]
    capacity: i32,
}

// Operation constants matching the C enum.
const OP_COPY: i32 = 0;
const OP_REVERSE: i32 = 1;
const OP_MERGE: i32 = 2;
const OP_SPLIT: i32 = 3;
const OP_INTERLEAVE: i32 = 4;
const OP_ROTATE: i32 = 5;
const OP_CHECKSUM: i32 = 6;

// ==================== Stdin reader emulating scanf("%d") ====================

struct StdinReader {
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
}

impl StdinReader {
    fn new() -> Self {
        let mut buf = Vec::new();
        // Read all of stdin upfront — this is acceptable because the C program
        // uses only scanf and we need scanf-equivalent whitespace handling.
        let _ = io::stdin().read_to_end(&mut buf);
        StdinReader { buf, pos: 0, eof: false }
    }

    fn peek(&self) -> Option<u8> {
        if self.pos < self.buf.len() {
            Some(self.buf[self.pos])
        } else {
            None
        }
    }

    fn advance(&mut self) {
        if self.pos < self.buf.len() {
            self.pos += 1;
        } else {
            self.eof = true;
        }
    }

    // Reads a signed integer like scanf("%d"). Skips leading whitespace
    // (including newlines), reads optional sign, then digits. Returns None
    // on failure (matching scanf return != 1).
    fn read_int(&mut self) -> Option<i64> {
        // Skip whitespace.
        loop {
            match self.peek() {
                Some(b) if (b as char).is_ascii_whitespace() => self.advance(),
                Some(_) => break,
                None => return None,
            }
        }

        let mut sign: i64 = 1;
        match self.peek() {
            Some(b'+') => { self.advance(); }
            Some(b'-') => { sign = -1; self.advance(); }
            _ => {}
        }

        let start = self.pos;
        let mut value: i64 = 0;
        let mut any = false;
        while let Some(b) = self.peek() {
            if b.is_ascii_digit() {
                // Use wrapping arithmetic to mimic C overflow behavior on
                // pathological inputs; for typical inputs this is exact.
                value = value.wrapping_mul(10).wrapping_add((b - b'0') as i64);
                self.advance();
                any = true;
            } else {
                break;
            }
        }

        if !any {
            // Restore position to before the (failed) read so subsequent
            // reads see the same characters — matches scanf behavior on
            // matching failure.
            self.pos = start;
            return None;
        }

        Some(sign.wrapping_mul(value))
    }
}

// ==================== Helper Functions ====================

fn calculate_checksum(data: &[u8]) -> u32 {
    let mut sum: u32 = 0;
    for &b in data {
        sum = (sum.wrapping_shl(3)) ^ (b as u32);
    }
    sum
}

fn validate_buffer(buf: &Buffer, stderr: &mut impl Write) -> bool {
    // The C version checks for NULL pointer first; in Rust we always have a
    // valid reference so that branch is unreachable.
    if buf.length > 256 {
        let _ = writeln!(stderr, "Error: Buffer length {} exceeds maximum 256", buf.length);
        return false;
    }
    let expected = calculate_checksum(&buf.data[..buf.length]);
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
    Some(BufferArray { buffers, count: 0, capacity: initial_capacity })
}

// ==================== Core Buffer Operations ====================

fn buffer_copy(src: &Buffer, dst: &mut Buffer, stderr: &mut impl Write) -> i32 {
    if !validate_buffer(src, stderr) {
        return -1;
    }
    let len = src.length;
    dst.data[..len].copy_from_slice(&src.data[..len]);
    dst.length = len;
    dst.checksum = calculate_checksum(&dst.data[..len]);
    0
}

fn buffer_reverse(buf: &mut Buffer, _stderr: &mut impl Write) -> i32 {
    if buf.length == 0 {
        return 0;
    }
    let len = buf.length;
    let mut temp = [0u8; 256];
    temp[..len].copy_from_slice(&buf.data[..len]);
    for i in 0..len {
        buf.data[i] = temp[len - 1 - i];
    }
    buf.checksum = calculate_checksum(&buf.data[..len]);
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
    dst.data[src1.length..src1.length + src2.length]
        .copy_from_slice(&src2.data[..src2.length]);
    dst.length = src1.length + src2.length;
    dst.checksum = calculate_checksum(&dst.data[..dst.length]);
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
    dst1.checksum = calculate_checksum(&dst1.data[..dst1.length]);

    let remaining = src.length - split_pos;
    if remaining > 0 {
        dst2.data[..remaining].copy_from_slice(&src.data[split_pos..split_pos + remaining]);
    }
    dst2.length = remaining;
    dst2.checksum = calculate_checksum(&dst2.data[..dst2.length]);
    0
}

fn buffer_interleave(
    src1: &Buffer,
    src2: &Buffer,
    dst: &mut Buffer,
    stderr: &mut impl Write,
) -> i32 {
    let max_len = if src1.length > src2.length { src1.length } else { src2.length };
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
    dst.checksum = calculate_checksum(&dst.data[..dst.length]);
    0
}

fn buffer_rotate(buf: &mut Buffer, mut positions: i32, _stderr: &mut impl Write) -> i32 {
    if buf.length == 0 || positions == 0 {
        return 0;
    }
    // Mimic C signed % semantics.
    let len_i = buf.length as i32;
    positions = positions % len_i;
    if positions < 0 {
        positions += len_i;
    }
    let p = positions as usize;
    let len = buf.length;
    let mut temp = [0u8; 256];
    temp[..len].copy_from_slice(&buf.data[..len]);
    // memcpy(buf->data, temp + positions, length - positions)
    buf.data[..len - p].copy_from_slice(&temp[p..len]);
    // memcpy(buf->data + (length - positions), temp, positions)
    buf.data[len - p..len].copy_from_slice(&temp[..p]);
    buf.checksum = calculate_checksum(&buf.data[..len]);
    0
}

// ==================== Input/Output Functions ====================

fn read_buffer(reader: &mut StdinReader, buf: &mut Buffer, stderr: &mut impl Write) -> i32 {
    let length = match reader.read_int() {
        Some(v) => v,
        None => {
            let _ = writeln!(stderr, "Error: Failed to read buffer length");
            return -1;
        }
    };

    // C compares against int range; values outside i32 will wrap when stored
    // into the C `int length`. We cast to i32 to match behavior on typical
    // inputs (values within i32 range).
    let length_i32 = length as i32;
    if length_i32 < 0 || length_i32 > 256 {
        let _ = writeln!(stderr, "Error: Invalid buffer length {}", length_i32);
        return -1;
    }

    buf.length = length_i32 as usize;
    for i in 0..buf.length {
        let byte = match reader.read_int() {
            Some(v) => v,
            None => {
                let _ = writeln!(stderr, "Error: Failed to read byte {}", i);
                return -1;
            }
        };
        // C: buf->data[i] = (uint8_t)byte; — truncates to low 8 bits.
        buf.data[i] = (byte as i32) as u8;
    }

    buf.checksum = calculate_checksum(&buf.data[..buf.length]);
    0
}

fn write_buffer(buf: &Buffer, stdout: &mut impl Write) {
    let _ = write!(stdout, "{}", buf.length);
    for i in 0..buf.length {
        let _ = write!(stdout, " {}", buf.data[i]);
    }
    let _ = writeln!(stdout);
}

// ==================== Main ====================

fn run() -> i32 {
    let stdout_handle = io::stdout();
    let mut stdout = BufWriter::new(stdout_handle.lock());
    let stderr_handle = io::stderr();
    let mut stderr = stderr_handle.lock();

    let mut reader = StdinReader::new();

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

    for i in 0..buffer_count as usize {
        if read_buffer(&mut reader, &mut buffers.buffers[i], &mut stderr) != 0 {
            return 1;
        }
        buffers.count += 1;
    }

    let mut result: i32 = 0;
    match operation {
        x if x == OP_COPY => {
            if buffer_count >= 2 {
                let mut temp = Buffer::new();
                let src_clone = buffers.buffers[0].clone();
                result = buffer_copy(&src_clone, &mut temp, &mut stderr);
                if result == 0 {
                    write_buffer(&temp, &mut stdout);
                }
            } else {
                let _ = writeln!(stderr, "Error: Copy needs at least 2 buffers");
                result = -1;
            }
        }
        x if x == OP_REVERSE => {
            for i in 0..buffer_count as usize {
                result = buffer_reverse(&mut buffers.buffers[i], &mut stderr);
                if result != 0 { break; }
                write_buffer(&buffers.buffers[i], &mut stdout);
            }
        }
        x if x == OP_MERGE => {
            if buffer_count >= 2 {
                let mut merged = Buffer::new();
                let src1 = buffers.buffers[0].clone();
                let src2 = buffers.buffers[1].clone();
                result = buffer_merge(&src1, &src2, &mut merged, &mut stderr);
                if result == 0 {
                    write_buffer(&merged, &mut stdout);
                }
            } else {
                let _ = writeln!(stderr, "Error: Merge needs at least 2 buffers");
                result = -1;
            }
        }
        x if x == OP_SPLIT => {
            if buffer_count >= 1 {
                let split_pos = match reader.read_int() {
                    Some(v) => v as i32,
                    None => {
                        let _ = writeln!(stderr, "Error: Failed to read split position");
                        result = -1;
                        // Fall through to end of arm.
                        i32::MIN // sentinel; not used because result != 0
                    }
                };
                if result == 0 {
                    let mut part1 = Buffer::new();
                    let mut part2 = Buffer::new();
                    let src_clone = buffers.buffers[0].clone();
                    // C passes split_pos as size_t; negative ints become huge size_t.
                    let split_pos_us = split_pos as isize as usize;
                    result = buffer_split(&src_clone, split_pos_us, &mut part1, &mut part2, &mut stderr);
                    if result == 0 {
                        write_buffer(&part1, &mut stdout);
                        write_buffer(&part2, &mut stdout);
                    }
                }
            }
        }
        x if x == OP_INTERLEAVE => {
            if buffer_count >= 2 {
                let mut interleaved = Buffer::new();
                let src1 = buffers.buffers[0].clone();
                let src2 = buffers.buffers[1].clone();
                result = buffer_interleave(&src1, &src2, &mut interleaved, &mut stderr);
                if result == 0 {
                    write_buffer(&interleaved, &mut stdout);
                }
            } else {
                let _ = writeln!(stderr, "Error: Interleave needs at least 2 buffers");
                result = -1;
            }
        }
        x if x == OP_ROTATE => {
            let positions = match reader.read_int() {
                Some(v) => v as i32,
                None => {
                    let _ = writeln!(stderr, "Error: Failed to read rotation amount");
                    result = -1;
                    0
                }
            };
            if result == 0 {
                for i in 0..buffer_count as usize {
                    result = buffer_rotate(&mut buffers.buffers[i], positions, &mut stderr);
                    if result != 0 { break; }
                    write_buffer(&buffers.buffers[i], &mut stdout);
                }
            }
        }
        x if x == OP_CHECKSUM => {
            for i in 0..buffer_count as usize {
                let _ = writeln!(stdout, "{}", buffers.buffers[i].checksum);
            }
        }
        _ => {
            let _ = writeln!(stderr, "Error: Unknown operation {}", operation);
            result = -1;
        }
    }

    let _ = stdout.flush();
    if result != 0 { 1 } else { 0 }
}

fn main() -> ExitCode {
    ExitCode::from(run() as u8)
}
