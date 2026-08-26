use std::io::{self, Read, Write};

const OP_COPY: i32 = 0;
const OP_REVERSE: i32 = 1;
const OP_MERGE: i32 = 2;
const OP_SPLIT: i32 = 3;
const OP_INTERLEAVE: i32 = 4;
const OP_ROTATE: i32 = 5;
const OP_CHECKSUM: i32 = 6;

#[derive(Clone)]
struct Buffer {
    data: [u8; 256],
    length: usize,
    checksum: u32,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            data: [0; 256],
            length: 0,
            checksum: 0,
        }
    }
}

struct Scanner {
    input: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn new(input: Vec<u8>) -> Self {
        Self { input, pos: 0 }
    }

    fn read_int(&mut self) -> Option<i32> {
        while self.pos < self.input.len() && is_scanf_space(self.input[self.pos]) {
            self.pos += 1;
        }

        let start = self.pos;
        let mut sign = 1i64;
        if self.pos < self.input.len()
            && (self.input[self.pos] == b'+' || self.input[self.pos] == b'-')
        {
            if self.input[self.pos] == b'-' {
                sign = -1;
            }
            self.pos += 1;
        }

        let digits_start = self.pos;
        let mut value = 0i64;
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
            value = value
                .saturating_mul(10)
                .saturating_add((self.input[self.pos] - b'0') as i64);
            self.pos += 1;
        }

        if self.pos == digits_start {
            self.pos = start;
            return None;
        }

        Some((value.saturating_mul(sign)) as i32)
    }
}

fn is_scanf_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\n' | b'\t' | b'\r' | 0x0b | 0x0c)
}

fn calculate_checksum(data: &[u8; 256], length: usize) -> u32 {
    let mut sum = 0u32;
    for byte in data.iter().take(length) {
        sum = sum.wrapping_shl(3) ^ u32::from(*byte);
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

fn buffer_copy(src: &Buffer, dst: &mut Buffer, stderr: &mut impl Write) -> i32 {
    if !validate_buffer(src, stderr) {
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

fn buffer_rotate(buf: &mut Buffer, mut positions: i32) -> i32 {
    if buf.length == 0 || positions == 0 {
        return 0;
    }

    positions %= buf.length as i32;
    if positions < 0 {
        positions += buf.length as i32;
    }

    let positions = positions as usize;
    let mut temp = [0u8; 256];
    temp[..buf.length].copy_from_slice(&buf.data[..buf.length]);
    buf.data[..buf.length - positions].copy_from_slice(&temp[positions..buf.length]);
    buf.data[buf.length - positions..buf.length].copy_from_slice(&temp[..positions]);
    buf.checksum = calculate_checksum(&buf.data, buf.length);
    0
}

fn read_buffer(scanner: &mut Scanner, buf: &mut Buffer, stderr: &mut impl Write) -> i32 {
    let Some(length) = scanner.read_int() else {
        let _ = writeln!(stderr, "Error: Failed to read buffer length");
        return -1;
    };

    if !(0..=256).contains(&length) {
        let _ = writeln!(stderr, "Error: Invalid buffer length {}", length);
        return -1;
    }

    buf.length = length as usize;
    for i in 0..buf.length {
        let Some(byte) = scanner.read_int() else {
            let _ = writeln!(stderr, "Error: Failed to read byte {}", i);
            return -1;
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

fn run(input: Vec<u8>, stdout: &mut impl Write, stderr: &mut impl Write) -> i32 {
    let mut scanner = Scanner::new(input);

    let Some(operation) = scanner.read_int() else {
        let _ = writeln!(stderr, "Error: Failed to read operation");
        return 1;
    };

    let Some(buffer_count) = scanner.read_int() else {
        let _ = writeln!(stderr, "Error: Failed to read buffer count");
        return 1;
    };

    if buffer_count <= 0 || buffer_count > 100 {
        let _ = writeln!(stderr, "Error: Invalid buffer count {}", buffer_count);
        return 1;
    }

    let mut buffers = vec![Buffer::default(); buffer_count as usize];
    let mut count = 0usize;
    for buf in buffers.iter_mut().take(buffer_count as usize) {
        if read_buffer(&mut scanner, buf, stderr) != 0 {
            return 1;
        }
        count += 1;
    }

    let mut result = 0;
    match operation {
        OP_COPY => {
            if buffer_count >= 2 {
                let mut temp = Buffer::default();
                result = buffer_copy(&buffers[0], &mut temp, stderr);
                if result == 0 {
                    write_buffer(&temp, stdout);
                }
            } else {
                let _ = writeln!(stderr, "Error: Copy needs at least 2 buffers");
                result = -1;
            }
        }
        OP_REVERSE => {
            for buf in buffers.iter_mut().take(count) {
                result = buffer_reverse(buf);
                if result != 0 {
                    break;
                }
                write_buffer(buf, stdout);
            }
        }
        OP_MERGE => {
            if buffer_count >= 2 {
                let mut merged = Buffer::default();
                result = buffer_merge(&buffers[0], &buffers[1], &mut merged, stderr);
                if result == 0 {
                    write_buffer(&merged, stdout);
                }
            } else {
                let _ = writeln!(stderr, "Error: Merge needs at least 2 buffers");
                result = -1;
            }
        }
        OP_SPLIT => {
            if buffer_count >= 1 {
                let Some(split_pos) = scanner.read_int() else {
                    let _ = writeln!(stderr, "Error: Failed to read split position");
                    result = -1;
                    return if result != 0 { 1 } else { 0 };
                };
                let mut part1 = Buffer::default();
                let mut part2 = Buffer::default();
                result = buffer_split(
                    &buffers[0],
                    split_pos as usize,
                    &mut part1,
                    &mut part2,
                    stderr,
                );
                if result == 0 {
                    write_buffer(&part1, stdout);
                    write_buffer(&part2, stdout);
                }
            }
        }
        OP_INTERLEAVE => {
            if buffer_count >= 2 {
                let mut interleaved = Buffer::default();
                result = buffer_interleave(&buffers[0], &buffers[1], &mut interleaved, stderr);
                if result == 0 {
                    write_buffer(&interleaved, stdout);
                }
            } else {
                let _ = writeln!(stderr, "Error: Interleave needs at least 2 buffers");
                result = -1;
            }
        }
        OP_ROTATE => {
            let Some(positions) = scanner.read_int() else {
                let _ = writeln!(stderr, "Error: Failed to read rotation amount");
                result = -1;
                return if result != 0 { 1 } else { 0 };
            };
            for buf in buffers.iter_mut().take(count) {
                result = buffer_rotate(buf, positions);
                if result != 0 {
                    break;
                }
                write_buffer(buf, stdout);
            }
        }
        OP_CHECKSUM => {
            for buf in buffers.iter().take(count) {
                let _ = writeln!(stdout, "{}", buf.checksum);
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

fn main() {
    let mut input = Vec::new();
    let _ = io::stdin().read_to_end(&mut input);
    let mut stdout = io::BufWriter::new(io::stdout());
    let mut stderr = io::BufWriter::new(io::stderr());
    let code = run(input, &mut stdout, &mut stderr);
    let _ = stdout.flush();
    let _ = stderr.flush();
    std::process::exit(code);
}
