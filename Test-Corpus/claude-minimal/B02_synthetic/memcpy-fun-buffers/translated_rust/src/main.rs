// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust

use std::io::{self, Read, Write};
use std::process::ExitCode;

const BUFFER_MAX: usize = 256;

// ==================== Data Structures ====================

#[derive(Clone, Copy)]
struct Buffer {
    data: [u8; BUFFER_MAX],
    length: usize,
    checksum: u32,
}

impl Buffer {
    fn new() -> Self {
        Buffer {
            data: [0u8; BUFFER_MAX],
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
enum Operation {
    Copy = 0,
    Reverse = 1,
    Merge = 2,
    Split = 3,
    Interleave = 4,
    Rotate = 5,
    Checksum = 6,
}

impl Operation {
    fn from_i32(v: i32) -> Option<Operation> {
        match v {
            0 => Some(Operation::Copy),
            1 => Some(Operation::Reverse),
            2 => Some(Operation::Merge),
            3 => Some(Operation::Split),
            4 => Some(Operation::Interleave),
            5 => Some(Operation::Rotate),
            6 => Some(Operation::Checksum),
            _ => None,
        }
    }
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
    if buf.length > BUFFER_MAX {
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

    let mut temp = [0u8; BUFFER_MAX];
    temp[..buf.length].copy_from_slice(&buf.data[..buf.length]);

    for i in 0..buf.length {
        buf.data[i] = temp[buf.length - 1 - i];
    }

    buf.checksum = calculate_checksum(&buf.data, buf.length);
    0
}

fn buffer_merge(src1: &Buffer, src2: &Buffer, dst: &mut Buffer) -> i32 {
    if src1.length + src2.length > BUFFER_MAX {
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
    if src1.length + src2.length > BUFFER_MAX {
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

    let mut positions = positions % (buf.length as i32);
    if positions < 0 {
        positions += buf.length as i32;
    }
    let positions = positions as usize;

    let mut temp = [0u8; BUFFER_MAX];
    temp[..buf.length].copy_from_slice(&buf.data[..buf.length]);

    // Copy rotated portions
    let first_len = buf.length - positions;
    buf.data[..first_len].copy_from_slice(&temp[positions..positions + first_len]);
    buf.data[first_len..first_len + positions].copy_from_slice(&temp[..positions]);

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
    let mut dst_pos: usize = 0;
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

    let stride_us = stride as usize;
    let mut dst_pos: usize = 0;
    let mut i = 0usize;
    while i < src.length {
        dst.data[dst_pos] = src.data[i];
        dst_pos += 1;
        i += stride_us;
    }

    dst.length = dst_pos;
    dst.checksum = calculate_checksum(&dst.data, dst.length);

    0
}

// ==================== Complex Processing Functions ====================

#[allow(dead_code)]
fn process_buffer_array(arr: &mut BufferArray, op: Operation, param: i32) -> i32 {
    if arr.count == 0 {
        eprintln!("Error: Invalid buffer array");
        return -1;
    }

    match op {
        Operation::Copy => {
            let src = arr.buffers[0];
            for i in 1..(arr.count as usize) {
                if buffer_copy(&src, &mut arr.buffers[i]) != 0 {
                    return -1;
                }
            }
        }
        Operation::Reverse => {
            for i in 0..(arr.count as usize) {
                if buffer_reverse(&mut arr.buffers[i]) != 0 {
                    return -1;
                }
            }
        }
        Operation::Merge => {
            if arr.count < 2 {
                eprintln!("Error: Need at least 2 buffers for merge");
                return -1;
            }
            let mut i = 0usize;
            while i < (arr.count as usize) - 1 {
                let mut merged = Buffer::new();
                let b0 = arr.buffers[i];
                let b1 = arr.buffers[i + 1];
                if buffer_merge(&b0, &b1, &mut merged) != 0 {
                    return -1;
                }
                arr.buffers[i] = merged;
                i += 2;
            }
        }
        Operation::Rotate => {
            for i in 0..(arr.count as usize) {
                if buffer_rotate(&mut arr.buffers[i], param) != 0 {
                    return -1;
                }
            }
        }
        Operation::Checksum => {
            for i in 0..(arr.count as usize) {
                if !validate_buffer(&arr.buffers[i]) {
                    return -1;
                }
            }
        }
        _ => {
            eprintln!("Error: Unknown operation {:?}", op);
            return -1;
        }
    }

    0
}

// ==================== Input/Output Functions ====================

struct TokenReader {
    data: Vec<u8>,
    pos: usize,
}

impl TokenReader {
    fn new() -> io::Result<Self> {
        let mut data = Vec::new();
        io::stdin().read_to_end(&mut data)?;
        Ok(TokenReader { data, pos: 0 })
    }

    fn next_token(&mut self) -> Option<String> {
        // skip whitespace
        while self.pos < self.data.len() && self.data[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
        if self.pos >= self.data.len() {
            return None;
        }
        let start = self.pos;
        while self.pos < self.data.len() && !self.data[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
        Some(String::from_utf8_lossy(&self.data[start..self.pos]).to_string())
    }

    fn next_i32(&mut self) -> Option<i32> {
        self.next_token().and_then(|t| t.parse::<i32>().ok())
    }
}

fn read_buffer(reader: &mut TokenReader, buf: &mut Buffer) -> i32 {
    let length = match reader.next_i32() {
        Some(v) => v,
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
        let byte = match reader.next_i32() {
            Some(v) => v,
            None => {
                eprintln!("Error: Failed to read byte {}", i);
                return -1;
            }
        };
        buf.data[i] = byte as u8;
    }

    buf.checksum = calculate_checksum(&buf.data, buf.length);
    0
}

fn write_buffer(buf: &Buffer) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut s = format!("{}", buf.length);
    for i in 0..buf.length {
        s.push_str(&format!(" {}", buf.data[i]));
    }
    s.push('\n');
    let _ = out.write_all(s.as_bytes());
}

// ==================== Main Function ====================

fn run() -> i32 {
    let mut reader = match TokenReader::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: Failed to read stdin: {}", e);
            return 1;
        }
    };

    let operation = match reader.next_i32() {
        Some(v) => v,
        None => {
            eprintln!("Error: Failed to read operation");
            return 1;
        }
    };

    let buffer_count = match reader.next_i32() {
        Some(v) => v,
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

    for i in 0..(buffer_count as usize) {
        if read_buffer(&mut reader, &mut buffers.buffers[i]) != 0 {
            return 1;
        }
        buffers.count += 1;
    }

    let mut result: i32 = 0;

    match operation {
        x if x == Operation::Copy as i32 => {
            if buffer_count >= 2 {
                let mut temp = Buffer::new();
                let src = buffers.buffers[0];
                result = buffer_copy(&src, &mut temp);
                if result == 0 {
                    write_buffer(&temp);
                }
            } else {
                eprintln!("Error: Copy needs at least 2 buffers");
                result = -1;
            }
        }
        x if x == Operation::Reverse as i32 => {
            for i in 0..(buffer_count as usize) {
                result = buffer_reverse(&mut buffers.buffers[i]);
                if result != 0 {
                    break;
                }
                write_buffer(&buffers.buffers[i]);
            }
        }
        x if x == Operation::Merge as i32 => {
            if buffer_count >= 2 {
                let mut merged = Buffer::new();
                let b0 = buffers.buffers[0];
                let b1 = buffers.buffers[1];
                result = buffer_merge(&b0, &b1, &mut merged);
                if result == 0 {
                    write_buffer(&merged);
                }
            } else {
                eprintln!("Error: Merge needs at least 2 buffers");
                result = -1;
            }
        }
        x if x == Operation::Split as i32 => {
            if buffer_count >= 1 {
                let split_pos = match reader.next_i32() {
                    Some(v) => v,
                    None => {
                        eprintln!("Error: Failed to read split position");
                        result = -1;
                        return if result != 0 { 1 } else { 0 };
                    }
                };
                let mut part1 = Buffer::new();
                let mut part2 = Buffer::new();
                let src = buffers.buffers[0];
                result = buffer_split(&src, split_pos as usize, &mut part1, &mut part2);
                if result == 0 {
                    write_buffer(&part1);
                    write_buffer(&part2);
                }
            }
        }
        x if x == Operation::Interleave as i32 => {
            if buffer_count >= 2 {
                let mut interleaved = Buffer::new();
                let b0 = buffers.buffers[0];
                let b1 = buffers.buffers[1];
                result = buffer_interleave(&b0, &b1, &mut interleaved);
                if result == 0 {
                    write_buffer(&interleaved);
                }
            } else {
                eprintln!("Error: Interleave needs at least 2 buffers");
                result = -1;
            }
        }
        x if x == Operation::Rotate as i32 => {
            let positions = match reader.next_i32() {
                Some(v) => v,
                None => {
                    eprintln!("Error: Failed to read rotation amount");
                    result = -1;
                    return if result != 0 { 1 } else { 0 };
                }
            };
            for i in 0..(buffer_count as usize) {
                result = buffer_rotate(&mut buffers.buffers[i], positions);
                if result != 0 {
                    break;
                }
                write_buffer(&buffers.buffers[i]);
            }
        }
        x if x == Operation::Checksum as i32 => {
            for i in 0..(buffer_count as usize) {
                println!("{}", buffers.buffers[i].checksum);
            }
        }
        _ => {
            eprintln!("Error: Unknown operation {}", operation);
            result = -1;
        }
    }

    // Validate Operation enum is referenced (avoid unused warning indirectly)
    let _ = Operation::from_i32(operation);
    let _ = buffers.capacity;

    if result != 0 {
        1
    } else {
        0
    }
}

fn main() -> ExitCode {
    ExitCode::from(run() as u8)
}
