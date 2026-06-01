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
            data: [0; 256],
            length: 0,
            checksum: 0,
        }
    }
}

struct BufferArray {
    buffers: Vec<Buffer>,
    count: i32,
    #[allow(dead_code)]
    capacity: i32,
}

const OP_COPY: i32 = 0;
const OP_REVERSE: i32 = 1;
const OP_MERGE: i32 = 2;
const OP_SPLIT: i32 = 3;
const OP_INTERLEAVE: i32 = 4;
const OP_ROTATE: i32 = 5;
const OP_CHECKSUM: i32 = 6;

// ==================== Token Reader ====================
// Mimics C's scanf("%d", ...) reading whitespace-separated integers from stdin.

struct TokenReader {
    data: Vec<u8>,
    pos: usize,
}

impl TokenReader {
    fn new() -> Self {
        let mut data = Vec::new();
        io::stdin().read_to_end(&mut data).ok();
        TokenReader { data, pos: 0 }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' || b == 0x0b || b == 0x0c {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    // Read a signed integer in the manner of scanf("%d"). Returns None on failure.
    // This wraps on overflow like C signed int (32-bit). For our use, reasonable inputs.
    fn read_int(&mut self) -> Option<i32> {
        self.skip_ws();
        if self.pos >= self.data.len() {
            return None;
        }
        let start = self.pos;
        let mut sign: i64 = 1;
        if self.data[self.pos] == b'+' {
            self.pos += 1;
        } else if self.data[self.pos] == b'-' {
            sign = -1;
            self.pos += 1;
        }
        let digits_start = self.pos;
        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            if b.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == digits_start {
            // No digits read; restore position
            self.pos = start;
            return None;
        }
        let s = std::str::from_utf8(&self.data[digits_start..self.pos]).ok()?;
        // Parse as i64 then truncate to i32 like C's scanf %d (which would have
        // undefined behavior on overflow; we approximate by wrapping).
        let val: i64 = s.parse::<i64>().unwrap_or(0);
        let signed = sign * val;
        Some(signed as i32)
    }
}

// ==================== Helper Functions ====================

fn calculate_checksum(data: &[u8]) -> u32 {
    let mut sum: u32 = 0;
    for &b in data {
        sum = (sum << 3) ^ (b as u32);
    }
    sum
}

fn validate_buffer(buf: &Buffer, stderr: &mut io::Stderr) -> bool {
    if buf.length > 256 {
        let _ = writeln!(
            stderr,
            "Error: Buffer length {} exceeds maximum 256",
            buf.length
        );
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

fn init_buffer_array(initial_capacity: i32, stderr: &mut io::Stderr) -> Option<BufferArray> {
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

fn buffer_copy(src: &Buffer, dst: &mut Buffer, stderr: &mut io::Stderr) -> i32 {
    if !validate_buffer(src, stderr) {
        return -1;
    }
    dst.data[..src.length].copy_from_slice(&src.data[..src.length]);
    dst.length = src.length;
    dst.checksum = calculate_checksum(&dst.data[..dst.length]);
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
    buf.checksum = calculate_checksum(&buf.data[..buf.length]);
    0
}

fn buffer_merge(src1: &Buffer, src2: &Buffer, dst: &mut Buffer, stderr: &mut io::Stderr) -> i32 {
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
    stderr: &mut io::Stderr,
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
    stderr: &mut io::Stderr,
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
    dst.checksum = calculate_checksum(&dst.data[..dst.length]);
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
    let n1 = buf.length - p;
    if n1 > 0 {
        let src_slice: [u8; 256] = temp;
        buf.data[..n1].copy_from_slice(&src_slice[p..p + n1]);
    }
    // memcpy(buf->data + (buf->length - positions), temp, positions);
    if p > 0 {
        buf.data[n1..n1 + p].copy_from_slice(&temp[..p]);
    }
    buf.checksum = calculate_checksum(&buf.data[..buf.length]);
    0
}

// ==================== I/O ====================

fn read_buffer(buf: &mut Buffer, reader: &mut TokenReader, stderr: &mut io::Stderr) -> i32 {
    let length = match reader.read_int() {
        Some(v) => v,
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
            Some(v) => v,
            None => {
                let _ = writeln!(stderr, "Error: Failed to read byte {}", i);
                return -1;
            }
        };
        buf.data[i] = byte as u8;
    }
    buf.checksum = calculate_checksum(&buf.data[..buf.length]);
    0
}

fn write_buffer(buf: &Buffer, stdout: &mut io::StdoutLock) {
    let _ = write!(stdout, "{}", buf.length);
    for i in 0..buf.length {
        let _ = write!(stdout, " {}", buf.data[i]);
    }
    let _ = writeln!(stdout);
}

// ==================== Main ====================

fn run() -> i32 {
    let mut reader = TokenReader::new();
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();
    let mut stderr = io::stderr();

    let operation = match reader.read_int() {
        Some(v) => v,
        None => {
            let _ = writeln!(stderr, "Error: Failed to read operation");
            return 1;
        }
    };

    let buffer_count = match reader.read_int() {
        Some(v) => v,
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
        if read_buffer(&mut buffers.buffers[i], &mut reader, &mut stderr) != 0 {
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
                result = buffer_copy(&src, &mut temp, &mut stderr);
                if result == 0 {
                    write_buffer(&temp, &mut stdout_lock);
                }
            } else {
                let _ = writeln!(stderr, "Error: Copy needs at least 2 buffers");
                result = -1;
            }
        }
        x if x == OP_REVERSE => {
            for i in 0..buffer_count as usize {
                result = buffer_reverse(&mut buffers.buffers[i]);
                if result != 0 {
                    break;
                }
                write_buffer(&buffers.buffers[i], &mut stdout_lock);
            }
        }
        x if x == OP_MERGE => {
            if buffer_count >= 2 {
                let mut merged = Buffer::new();
                let s1 = buffers.buffers[0];
                let s2 = buffers.buffers[1];
                result = buffer_merge(&s1, &s2, &mut merged, &mut stderr);
                if result == 0 {
                    write_buffer(&merged, &mut stdout_lock);
                }
            } else {
                let _ = writeln!(stderr, "Error: Merge needs at least 2 buffers");
                result = -1;
            }
        }
        x if x == OP_SPLIT => {
            if buffer_count >= 1 {
                match reader.read_int() {
                    None => {
                        let _ = writeln!(stderr, "Error: Failed to read split position");
                        result = -1;
                    }
                    Some(split_pos) => {
                        let mut part1 = Buffer::new();
                        let mut part2 = Buffer::new();
                        let src = buffers.buffers[0];
                        // C casts int -> size_t. Negative becomes huge.
                        let split_usize: usize = split_pos as usize;
                        result = buffer_split(
                            &src,
                            split_usize,
                            &mut part1,
                            &mut part2,
                            &mut stderr,
                        );
                        if result == 0 {
                            write_buffer(&part1, &mut stdout_lock);
                            write_buffer(&part2, &mut stdout_lock);
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
                result = buffer_interleave(&s1, &s2, &mut interleaved, &mut stderr);
                if result == 0 {
                    write_buffer(&interleaved, &mut stdout_lock);
                }
            } else {
                let _ = writeln!(stderr, "Error: Interleave needs at least 2 buffers");
                result = -1;
            }
        }
        x if x == OP_ROTATE => match reader.read_int() {
            None => {
                let _ = writeln!(stderr, "Error: Failed to read rotation amount");
                result = -1;
            }
            Some(positions) => {
                for i in 0..buffer_count as usize {
                    result = buffer_rotate(&mut buffers.buffers[i], positions);
                    if result != 0 {
                        break;
                    }
                    write_buffer(&buffers.buffers[i], &mut stdout_lock);
                }
            }
        },
        x if x == OP_CHECKSUM => {
            for i in 0..buffer_count as usize {
                let _ = writeln!(stdout_lock, "{}", buffers.buffers[i].checksum);
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
