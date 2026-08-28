use std::io::{self, Write};
use std::os::raw::{c_char, c_int};
use std::process::ExitCode;

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

impl Buffer {
    fn new() -> Self {
        Self {
            data: [0; 256],
            length: 0,
            checksum: 0,
        }
    }
}

struct BufferArray {
    buffers: Vec<Buffer>,
    count: usize,
}

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

// Using the platform scanf preserves C's tokenization and stream behavior.
fn scan_int() -> Option<i32> {
    const FORMAT: &[u8] = b"%d\0";
    let mut value: c_int = 0;
    let result = unsafe { scanf(FORMAT.as_ptr().cast(), &mut value) };
    (result == 1).then_some(value)
}

fn calculate_checksum(data: &[u8], length: usize) -> u32 {
    let mut sum = 0_u32;
    for byte in &data[..length] {
        sum = sum.wrapping_shl(3) ^ u32::from(*byte);
    }
    sum
}

fn validate_buffer(buf: &Buffer) -> bool {
    if buf.length > 256 {
        eprintln!(
            "Error: Buffer length {} exceeds maximum 256",
            buf.length
        );
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

    let mut buffers = Vec::new();
    if buffers
        .try_reserve_exact(initial_capacity as usize)
        .is_err()
    {
        eprintln!("Error: Failed to allocate buffer storage");
        return None;
    }

    Some(BufferArray { buffers, count: 0 })
}

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

    let mut temp = [0_u8; 256];
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
        dst2.data[..remaining].copy_from_slice(&src.data[split_pos..src.length]);
    }
    dst2.length = remaining;
    dst2.checksum = calculate_checksum(&dst2.data, dst2.length);
    0
}

fn buffer_interleave(src1: &Buffer, src2: &Buffer, dst: &mut Buffer) -> i32 {
    let max_len = src1.length.max(src2.length);
    if src1.length + src2.length > 256 {
        eprintln!("Error: Interleaved length exceeds maximum");
        return -1;
    }

    let mut dst_pos = 0;
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

    let mut temp = [0_u8; 256];
    temp[..buf.length].copy_from_slice(&buf.data[..buf.length]);
    let first_len = buf.length - positions;
    buf.data[..first_len].copy_from_slice(&temp[positions..buf.length]);
    buf.data[first_len..buf.length].copy_from_slice(&temp[..positions]);
    buf.checksum = calculate_checksum(&buf.data, buf.length);
    0
}

#[allow(dead_code)]
fn buffer_conditional_copy(
    src: &Buffer,
    dst: &mut Buffer,
    pattern: u8,
    copy_matching: bool,
) -> i32 {
    let mut dst_pos = 0;
    for i in 0..src.length {
        let matches = src.data[i] == pattern;
        if matches == copy_matching {
            dst.data[dst_pos] = src.data[i];
            dst_pos += 1;
        }
    }

    dst.length = dst_pos;
    dst.checksum = calculate_checksum(&dst.data, dst.length);
    0
}

#[allow(dead_code)]
fn buffer_copy_strided(src: &Buffer, dst: &mut Buffer, stride: i32) -> i32 {
    if stride <= 0 {
        eprintln!("Error: Invalid stride {}", stride);
        return -1;
    }

    let mut dst_pos = 0;
    let mut i = 0;
    while i < src.length {
        dst.data[dst_pos] = src.data[i];
        dst_pos += 1;
        i += stride as usize;
    }

    dst.length = dst_pos;
    dst.checksum = calculate_checksum(&dst.data, dst.length);
    0
}

#[allow(dead_code)]
fn process_buffer_array(arr: &mut BufferArray, op: i32, param: i32) -> i32 {
    if arr.count == 0 {
        eprintln!("Error: Invalid buffer array");
        return -1;
    }

    match op {
        OP_COPY => {
            for i in 1..arr.count {
                let src = arr.buffers[0].clone();
                if buffer_copy(&src, &mut arr.buffers[i]) != 0 {
                    return -1;
                }
            }
        }
        OP_REVERSE => {
            for i in 0..arr.count {
                if buffer_reverse(&mut arr.buffers[i]) != 0 {
                    return -1;
                }
            }
        }
        OP_MERGE => {
            if arr.count < 2 {
                eprintln!("Error: Need at least 2 buffers for merge");
                return -1;
            }
            let mut i = 0;
            while i < arr.count - 1 {
                let mut merged = Buffer::new();
                if buffer_merge(&arr.buffers[i], &arr.buffers[i + 1], &mut merged) != 0 {
                    return -1;
                }
                arr.buffers[i] = merged;
                i += 2;
            }
        }
        OP_ROTATE => {
            for i in 0..arr.count {
                if buffer_rotate(&mut arr.buffers[i], param) != 0 {
                    return -1;
                }
            }
        }
        OP_CHECKSUM => {
            for i in 0..arr.count {
                if !validate_buffer(&arr.buffers[i]) {
                    return -1;
                }
            }
        }
        _ => {
            eprintln!("Error: Unknown operation {}", op);
            return -1;
        }
    }
    0
}

fn read_buffer() -> Option<Buffer> {
    let length = match scan_int() {
        Some(length) => length,
        None => {
            eprintln!("Error: Failed to read buffer length");
            return None;
        }
    };

    if !(0..=256).contains(&length) {
        eprintln!("Error: Invalid buffer length {}", length);
        return None;
    }

    let mut buf = Buffer::new();
    buf.length = length as usize;
    for i in 0..buf.length {
        let byte = match scan_int() {
            Some(byte) => byte,
            None => {
                eprintln!("Error: Failed to read byte {}", i);
                return None;
            }
        };
        buf.data[i] = byte as u8;
    }

    buf.checksum = calculate_checksum(&buf.data, buf.length);
    Some(buf)
}

fn write_buffer(output: &mut impl Write, buf: &Buffer) {
    let _ = write!(output, "{}", buf.length);
    for byte in &buf.data[..buf.length] {
        let _ = write!(output, " {}", byte);
    }
    let _ = writeln!(output);
}

fn run() -> u8 {
    let operation = match scan_int() {
        Some(operation) => operation,
        None => {
            eprintln!("Error: Failed to read operation");
            return 1;
        }
    };

    let buffer_count = match scan_int() {
        Some(buffer_count) => buffer_count,
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
        Some(buffers) => buffers,
        None => return 1,
    };

    for _ in 0..buffer_count {
        let buffer = match read_buffer() {
            Some(buffer) => buffer,
            None => return 1,
        };
        buffers.buffers.push(buffer);
        buffers.count += 1;
    }

    let stdout = io::stdout();
    let mut output = io::BufWriter::new(stdout.lock());
    let mut result = 0;

    match operation {
        OP_COPY => {
            if buffer_count >= 2 {
                let mut temp = Buffer::new();
                result = buffer_copy(&buffers.buffers[0], &mut temp);
                if result == 0 {
                    write_buffer(&mut output, &temp);
                }
            } else {
                eprintln!("Error: Copy needs at least 2 buffers");
                result = -1;
            }
        }
        OP_REVERSE => {
            for i in 0..buffer_count as usize {
                result = buffer_reverse(&mut buffers.buffers[i]);
                if result != 0 {
                    break;
                }
                write_buffer(&mut output, &buffers.buffers[i]);
            }
        }
        OP_MERGE => {
            if buffer_count >= 2 {
                let mut merged = Buffer::new();
                result = buffer_merge(&buffers.buffers[0], &buffers.buffers[1], &mut merged);
                if result == 0 {
                    write_buffer(&mut output, &merged);
                }
            } else {
                eprintln!("Error: Merge needs at least 2 buffers");
                result = -1;
            }
        }
        OP_SPLIT => {
            if buffer_count >= 1 {
                let split_pos = match scan_int() {
                    Some(split_pos) => split_pos,
                    None => {
                        eprintln!("Error: Failed to read split position");
                        result = -1;
                        0
                    }
                };
                if result == 0 {
                    let mut part1 = Buffer::new();
                    let mut part2 = Buffer::new();
                    result = buffer_split(
                        &buffers.buffers[0],
                        split_pos as usize,
                        &mut part1,
                        &mut part2,
                    );
                    if result == 0 {
                        write_buffer(&mut output, &part1);
                        write_buffer(&mut output, &part2);
                    }
                }
            }
        }
        OP_INTERLEAVE => {
            if buffer_count >= 2 {
                let mut interleaved = Buffer::new();
                result = buffer_interleave(
                    &buffers.buffers[0],
                    &buffers.buffers[1],
                    &mut interleaved,
                );
                if result == 0 {
                    write_buffer(&mut output, &interleaved);
                }
            } else {
                eprintln!("Error: Interleave needs at least 2 buffers");
                result = -1;
            }
        }
        OP_ROTATE => {
            let positions = match scan_int() {
                Some(positions) => positions,
                None => {
                    eprintln!("Error: Failed to read rotation amount");
                    result = -1;
                    0
                }
            };
            if result == 0 {
                for i in 0..buffer_count as usize {
                    result = buffer_rotate(&mut buffers.buffers[i], positions);
                    if result != 0 {
                        break;
                    }
                    write_buffer(&mut output, &buffers.buffers[i]);
                }
            }
        }
        OP_CHECKSUM => {
            for buffer in &buffers.buffers {
                let _ = writeln!(output, "{}", buffer.checksum);
            }
        }
        _ => {
            eprintln!("Error: Unknown operation {}", operation);
            result = -1;
        }
    }

    let _ = output.flush();
    if result != 0 { 1 } else { 0 }
}

fn main() -> ExitCode {
    ExitCode::from(run())
}
