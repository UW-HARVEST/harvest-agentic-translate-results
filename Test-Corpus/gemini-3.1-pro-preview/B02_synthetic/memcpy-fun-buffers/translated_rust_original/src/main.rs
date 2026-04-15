use std::io::{self, BufRead};

#[derive(Clone, Copy)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    fn from_i32(val: i32) -> Option<Self> {
        match val {
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

fn calculate_checksum(data: &[u8]) -> u32 {
    let mut sum: u32 = 0;
    for &b in data {
        sum = (sum << 3) ^ (b as u32);
    }
    sum
}

fn validate_buffer(buf: &Buffer) -> bool {
    if buf.length > 256 {
        eprintln!("Error: Buffer length {} exceeds maximum 256", buf.length);
        return false;
    }
    let expected = calculate_checksum(&buf.data[..buf.length]);
    if buf.checksum != expected {
        eprintln!(
            "Warning: Checksum mismatch. Expected {}, got {}",
            expected, buf.checksum
        );
    }
    true
}

fn buffer_copy(src: &Buffer, dst: &mut Buffer) -> Result<(), ()> {
    if !validate_buffer(src) {
        return Err(());
    }
    dst.data[..src.length].copy_from_slice(&src.data[..src.length]);
    dst.length = src.length;
    dst.checksum = calculate_checksum(&dst.data[..dst.length]);
    Ok(())
}

fn buffer_reverse(buf: &mut Buffer) -> Result<(), ()> {
    if buf.length == 0 {
        return Ok(());
    }
    buf.data[..buf.length].reverse();
    buf.checksum = calculate_checksum(&buf.data[..buf.length]);
    Ok(())
}

fn buffer_merge(src1: &Buffer, src2: &Buffer, dst: &mut Buffer) -> Result<(), ()> {
    if src1.length + src2.length > 256 {
        eprintln!(
            "Error: Merged length {} exceeds maximum",
            src1.length + src2.length
        );
        return Err(());
    }
    dst.data[..src1.length].copy_from_slice(&src1.data[..src1.length]);
    dst.data[src1.length..src1.length + src2.length].copy_from_slice(&src2.data[..src2.length]);
    dst.length = src1.length + src2.length;
    dst.checksum = calculate_checksum(&dst.data[..dst.length]);
    Ok(())
}

fn buffer_split(
    src: &Buffer,
    split_pos: usize,
    dst1: &mut Buffer,
    dst2: &mut Buffer,
) -> Result<(), ()> {
    if split_pos > src.length {
        eprintln!(
            "Error: Split position {} exceeds length {}",
            split_pos, src.length
        );
        return Err(());
    }
    if split_pos > 0 {
        dst1.data[..split_pos].copy_from_slice(&src.data[..split_pos]);
    }
    dst1.length = split_pos;
    dst1.checksum = calculate_checksum(&dst1.data[..dst1.length]);

    let remaining = src.length - split_pos;
    if remaining > 0 {
        dst2.data[..remaining].copy_from_slice(&src.data[split_pos..src.length]);
    }
    dst2.length = remaining;
    dst2.checksum = calculate_checksum(&dst2.data[..dst2.length]);
    Ok(())
}

fn buffer_interleave(src1: &Buffer, src2: &Buffer, dst: &mut Buffer) -> Result<(), ()> {
    let max_len = src1.length.max(src2.length);
    if src1.length + src2.length > 256 {
        eprintln!("Error: Interleaved length exceeds maximum");
        return Err(());
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
    dst.checksum = calculate_checksum(&dst.data[..dst.length]);
    Ok(())
}

fn buffer_rotate(buf: &mut Buffer, positions: i32) -> Result<(), ()> {
    if buf.length == 0 || positions == 0 {
        return Ok(());
    }
    let mut pos = positions % (buf.length as i32);
    if pos < 0 {
        pos += buf.length as i32;
    }
    let pos = pos as usize;
    buf.data[..buf.length].rotate_left(pos);
    buf.checksum = calculate_checksum(&buf.data[..buf.length]);
    Ok(())
}

#[allow(dead_code)]
fn buffer_conditional_copy(
    src: &Buffer,
    dst: &mut Buffer,
    pattern: u8,
    copy_matching: bool,
) -> Result<(), ()> {
    let mut dst_pos = 0;
    for i in 0..src.length {
        let matches = src.data[i] == pattern;
        if matches == copy_matching {
            dst.data[dst_pos] = src.data[i];
            dst_pos += 1;
        }
    }
    dst.length = dst_pos;
    dst.checksum = calculate_checksum(&dst.data[..dst.length]);
    Ok(())
}

#[allow(dead_code)]
fn buffer_copy_strided(src: &Buffer, dst: &mut Buffer, stride: i32) -> Result<(), ()> {
    if stride <= 0 {
        eprintln!("Error: Invalid stride {}", stride);
        return Err(());
    }
    let stride = stride as usize;
    let mut dst_pos = 0;
    for i in (0..src.length).step_by(stride) {
        dst.data[dst_pos] = src.data[i];
        dst_pos += 1;
    }
    dst.length = dst_pos;
    dst.checksum = calculate_checksum(&dst.data[..dst.length]);
    Ok(())
}

#[allow(dead_code)]
fn process_buffer_array(arr: &mut Vec<Buffer>, op: Operation, param: i32) -> Result<(), ()> {
    if arr.is_empty() {
        eprintln!("Error: Invalid buffer array");
        return Err(());
    }
    match op {
        Operation::Copy => {
            let src = arr[0];
            for i in 1..arr.len() {
                buffer_copy(&src, &mut arr[i])?;
            }
        }
        Operation::Reverse => {
            for buf in arr.iter_mut() {
                buffer_reverse(buf)?;
            }
        }
        Operation::Merge => {
            if arr.len() < 2 {
                eprintln!("Error: Need at least 2 buffers for merge");
                return Err(());
            }
            for i in (0..arr.len() - 1).step_by(2) {
                let mut merged = Buffer::default();
                buffer_merge(&arr[i], &arr[i + 1], &mut merged)?;
                arr[i] = merged;
            }
        }
        Operation::Rotate => {
            for buf in arr.iter_mut() {
                buffer_rotate(buf, param)?;
            }
        }
        Operation::Checksum => {
            for buf in arr.iter() {
                if !validate_buffer(buf) {
                    return Err(());
                }
            }
        }
        _ => {
            eprintln!("Error: Unknown operation {:?}", op);
            return Err(());
        }
    }
    Ok(())
}

struct Scanner<R> {
    reader: R,
    buffer: Vec<String>,
}

impl<R: BufRead> Scanner<R> {
    fn new(reader: R) -> Self {
        Self {
            reader,
            buffer: Vec::new(),
        }
    }

    fn next<T: std::str::FromStr>(&mut self) -> Option<T> {
        loop {
            if let Some(token) = self.buffer.pop() {
                if let Ok(val) = token.parse() {
                    return Some(val);
                } else {
                    return None;
                }
            }
            let mut line = String::new();
            if self.reader.read_line(&mut line).unwrap_or(0) == 0 {
                return None;
            }
            self.buffer = line.split_whitespace().rev().map(String::from).collect();
        }
    }
}

fn read_buffer<R: BufRead>(scanner: &mut Scanner<R>, buf: &mut Buffer) -> Result<(), ()> {
    let length: i32 = scanner.next().ok_or_else(|| {
        eprintln!("Error: Failed to read buffer length");
    })?;
    if length < 0 || length > 256 {
        eprintln!("Error: Invalid buffer length {}", length);
        return Err(());
    }
    buf.length = length as usize;
    for i in 0..buf.length {
        let byte: i32 = scanner.next().ok_or_else(|| {
            eprintln!("Error: Failed to read byte {}", i);
        })?;
        buf.data[i] = byte as u8;
    }
    buf.checksum = calculate_checksum(&buf.data[..buf.length]);
    Ok(())
}

fn write_buffer(buf: &Buffer) {
    print!("{}", buf.length);
    for i in 0..buf.length {
        print!(" {}", buf.data[i]);
    }
    println!();
}

fn main() {
    let stdin = io::stdin();
    let mut scanner = Scanner::new(stdin.lock());

    let op_val: i32 = if let Some(val) = scanner.next() {
        val
    } else {
        eprintln!("Error: Failed to read operation");
        std::process::exit(1);
    };

    let buffer_count: i32 = if let Some(val) = scanner.next() {
        val
    } else {
        eprintln!("Error: Failed to read buffer count");
        std::process::exit(1);
    };

    if buffer_count <= 0 || buffer_count > 100 {
        eprintln!("Error: Invalid buffer count {}", buffer_count);
        std::process::exit(1);
    }

    let mut buffers = Vec::with_capacity(buffer_count as usize);
    for _ in 0..buffer_count {
        let mut buf = Buffer::default();
        if read_buffer(&mut scanner, &mut buf).is_err() {
            std::process::exit(1);
        }
        buffers.push(buf);
    }

    let mut result = 0;
    let operation = Operation::from_i32(op_val);

    match operation {
        Some(Operation::Copy) => {
            if buffer_count >= 2 {
                let mut temp = Buffer::default();
                if buffer_copy(&buffers[0], &mut temp).is_ok() {
                    write_buffer(&temp);
                } else {
                    result = -1;
                }
            } else {
                eprintln!("Error: Copy needs at least 2 buffers");
                result = -1;
            }
        }
        Some(Operation::Reverse) => {
            for buf in buffers.iter_mut() {
                if buffer_reverse(buf).is_err() {
                    result = -1;
                    break;
                }
                write_buffer(buf);
            }
        }
        Some(Operation::Merge) => {
            if buffer_count >= 2 {
                let mut merged = Buffer::default();
                if buffer_merge(&buffers[0], &buffers[1], &mut merged).is_ok() {
                    write_buffer(&merged);
                } else {
                    result = -1;
                }
            } else {
                eprintln!("Error: Merge needs at least 2 buffers");
                result = -1;
            }
        }
        Some(Operation::Split) => {
            if buffer_count >= 1 {
                if let Some(split_pos) = scanner.next::<i32>() {
                    let mut part1 = Buffer::default();
                    let mut part2 = Buffer::default();
                    if buffer_split(&buffers[0], split_pos as usize, &mut part1, &mut part2).is_ok()
                    {
                        write_buffer(&part1);
                        write_buffer(&part2);
                    } else {
                        result = -1;
                    }
                } else {
                    eprintln!("Error: Failed to read split position");
                    result = -1;
                }
            }
        }
        Some(Operation::Interleave) => {
            if buffer_count >= 2 {
                let mut interleaved = Buffer::default();
                if buffer_interleave(&buffers[0], &buffers[1], &mut interleaved).is_ok() {
                    write_buffer(&interleaved);
                } else {
                    result = -1;
                }
            } else {
                eprintln!("Error: Interleave needs at least 2 buffers");
                result = -1;
            }
        }
        Some(Operation::Rotate) => {
            if let Some(positions) = scanner.next::<i32>() {
                for buf in buffers.iter_mut() {
                    if buffer_rotate(buf, positions).is_err() {
                        result = -1;
                        break;
                    }
                    write_buffer(buf);
                }
            } else {
                eprintln!("Error: Failed to read rotation amount");
                result = -1;
            }
        }
        Some(Operation::Checksum) => {
            for buf in buffers.iter() {
                println!("{}", buf.checksum);
            }
        }
        None => {
            eprintln!("Error: Unknown operation {}", op_val);
            result = -1;
        }
    }

    if result != 0 {
        std::process::exit(1);
    }
}
