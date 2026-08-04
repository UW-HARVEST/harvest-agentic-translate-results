use std::io::{self, BufRead, Write};

const MAX_BUFFER_SIZE: usize = 256;

#[derive(Clone, Copy)]
struct Buffer {
    data: [u8; MAX_BUFFER_SIZE],
    length: usize,
    checksum: u32,
}

impl Buffer {
    fn new() -> Self {
        Buffer {
            data: [0; MAX_BUFFER_SIZE],
            length: 0,
            checksum: 0,
        }
    }
}

struct BufferArray {
    buffers: Vec<Buffer>,
    count: usize,
}

#[derive(Clone, Copy)]
#[repr(u8)]
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
    fn from_u8(value: u8) -> Option<Self> {
        match value {
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
    for &byte in data {
        sum = (sum << 3) ^ (byte as u32);
    }
    sum
}

fn validate_buffer(buf: &Buffer) -> bool {
    if buf.length > MAX_BUFFER_SIZE {
        eprintln!("Error: Buffer length {} exceeds maximum {}", buf.length, MAX_BUFFER_SIZE);
        return false;
    }
    let expected = calculate_checksum(&buf.data[..buf.length]);
    if buf.checksum != expected {
        eprintln!("Warning: Checksum mismatch. Expected {}, got {}", expected, buf.checksum);
    }
    true
}

fn init_buffer_array(initial_capacity: usize) -> Option<BufferArray> {
    if initial_capacity == 0 {
        eprintln!("Error: Invalid capacity {}", initial_capacity);
        return None;
    }
    Some(BufferArray {
        buffers: vec![Buffer::new(); initial_capacity],
        count: 0,
    })
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
    let temp: Vec<u8> = buf.data[..buf.length].to_vec();
    for i in 0..buf.length {
        buf.data[i] = temp[buf.length - 1 - i];
    }
    buf.checksum = calculate_checksum(&buf.data[..buf.length]);
    Ok(())
}

fn buffer_merge(src1: &Buffer, src2: &Buffer, dst: &mut Buffer) -> Result<(), ()> {
    if src1.length + src2.length > MAX_BUFFER_SIZE {
        eprintln!("Error: Merged length {} exceeds maximum", src1.length + src2.length);
        return Err(());
    }
    dst.data[..src1.length].copy_from_slice(&src1.data[..src1.length]);
    dst.data[src1.length..src1.length + src2.length].copy_from_slice(&src2.data[..src2.length]);
    dst.length = src1.length + src2.length;
    dst.checksum = calculate_checksum(&dst.data[..dst.length]);
    Ok(())
}

fn buffer_split(src: &Buffer, split_pos: usize, dst1: &mut Buffer, dst2: &mut Buffer) -> Result<(), ()> {
    if split_pos > src.length {
        eprintln!("Error: Split position {} exceeds length {}", split_pos, src.length);
        return Err(());
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
    Ok(())
}

fn buffer_interleave(src1: &Buffer, src2: &Buffer, dst: &mut Buffer) -> Result<(), ()> {
    let max_len = src1.length.max(src2.length);
    if src1.length + src2.length > MAX_BUFFER_SIZE {
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
    let len = buf.length as i32;
    let mut pos = positions % len;
    if pos < 0 {
        pos += len;
    }
    let pos = pos as usize;
    let temp: Vec<u8> = buf.data[..buf.length].to_vec();
    buf.data[..buf.length - pos].copy_from_slice(&temp[pos..]);
    buf.data[buf.length - pos..buf.length].copy_from_slice(&temp[..pos]);
    buf.checksum = calculate_checksum(&buf.data[..buf.length]);
    Ok(())
}

fn buffer_conditional_copy(src: &Buffer, dst: &mut Buffer, pattern: u8, copy_matching: bool) -> Result<(), ()> {
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

fn buffer_copy_strided(src: &Buffer, dst: &mut Buffer, stride: usize) -> Result<(), ()> {
    if stride == 0 {
        eprintln!("Error: Invalid stride {}", stride);
        return Err(());
    }
    let mut dst_pos = 0;
    let mut i = 0;
    while i < src.length {
        dst.data[dst_pos] = src.data[i];
        dst_pos += 1;
        i += stride;
    }
    dst.length = dst_pos;
    dst.checksum = calculate_checksum(&dst.data[..dst.length]);
    Ok(())
}

fn process_buffer_array(arr: &mut BufferArray, op: Operation, param: i32) -> Result<(), ()> {
    if arr.count == 0 {
        eprintln!("Error: Invalid buffer array");
        return Err(());
    }
    match op {
        Operation::Copy => {
            for i in 1..arr.count {
                buffer_copy(&arr.buffers[0], &mut arr.buffers[i])?;
            }
        }
        Operation::Reverse => {
            for i in 0..arr.count {
                buffer_reverse(&mut arr.buffers[i])?;
            }
        }
        Operation::Merge => {
            if arr.count < 2 {
                eprintln!("Error: Need at least 2 buffers for merge");
                return Err(());
            }
            let mut i = 0;
            while i < arr.count - 1 {
                let mut merged = Buffer::new();
                buffer_merge(&arr.buffers[i], &arr.buffers[i + 1], &mut merged)?;
                arr.buffers[i] = merged;
                i += 2;
            }
        }
        Operation::Rotate => {
            for i in 0..arr.count {
                buffer_rotate(&mut arr.buffers[i], param)?;
            }
        }
        Operation::Checksum => {
            for i in 0..arr.count {
                if !validate_buffer(&arr.buffers[i]) {
                    return Err(());
                }
            }
        }
        _ => {
            eprintln!("Error: Unknown operation");
            return Err(());
        }
    }
    Ok(())
}

fn read_buffer(buf: &mut Buffer, stdin: &mut io::StdinLock) -> Result<(), ()> {
    let mut line = String::new();
    if stdin.read_line(&mut line).is_err() {
        eprintln!("Error: Failed to read buffer length");
        return Err(());
    }
    let parts: Vec<&str> = line.trim().split_whitespace().collect();
    if parts.is_empty() {
        eprintln!("Error: Failed to read buffer length");
        return Err(());
    }
    let length: i32 = parts[0].parse().map_err(|_| {
        eprintln!("Error: Failed to read buffer length");
    })?;
    if length < 0 || length > MAX_BUFFER_SIZE as i32 {
        eprintln!("Error: Invalid buffer length {}", length);
        return Err(());
    }
    buf.length = length as usize;
    let mut byte_idx = 1;
    for i in 0..buf.length {
        let byte_val: i32 = if byte_idx < parts.len() {
            parts[byte_idx].parse().map_err(|_| {
                eprintln!("Error: Failed to read byte {}", i);
            })?
        } else {
            line.clear();
            if stdin.read_line(&mut line).is_err() {
                eprintln!("Error: Failed to read byte {}", i);
                return Err(());
            }
            let new_parts: Vec<&str> = line.trim().split_whitespace().collect();
            if new_parts.is_empty() {
                eprintln!("Error: Failed to read byte {}", i);
                return Err(());
            }
            byte_idx = 0;
            new_parts[byte_idx].parse().map_err(|_| {
                eprintln!("Error: Failed to read byte {}", i);
            })?
        };
        buf.data[i] = byte_val as u8;
        byte_idx += 1;
    }
    buf.checksum = calculate_checksum(&buf.data[..buf.length]);
    Ok(())
}

fn write_buffer(buf: &Buffer, stdout: &mut io::StdoutLock) {
    let _ = write!(stdout, "{}", buf.length);
    for i in 0..buf.length {
        let _ = write!(stdout, " {}", buf.data[i]);
    }
    let _ = writeln!(stdout);
}

fn main() {
    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();
    
    let mut line = String::new();
    
    if stdin_lock.read_line(&mut line).is_err() {
        eprintln!("Error: Failed to read operation");
        std::process::exit(1);
    }
    let operation: u8 = line.trim().parse().unwrap_or(255);
    
    line.clear();
    if stdin_lock.read_line(&mut line).is_err() {
        eprintln!("Error: Failed to read buffer count");
        std::process::exit(1);
    }
    let buffer_count: i32 = line.trim().parse().unwrap_or(-1);
    
    if buffer_count <= 0 || buffer_count > 100 {
        eprintln!("Error: Invalid buffer count {}", buffer_count);
        std::process::exit(1);
    }
    
    let mut buffers = match init_buffer_array(buffer_count as usize) {
        Some(b) => b,
        None => std::process::exit(1),
    };
    
    for i in 0..buffer_count as usize {
        if read_buffer(&mut buffers.buffers[i], &mut stdin_lock).is_err() {
            std::process::exit(1);
        }
        buffers.count += 1;
    }
    
    let op = match Operation::from_u8(operation) {
        Some(o) => o,
        None => {
            eprintln!("Error: Unknown operation {}", operation);
            std::process::exit(1);
        }
    };
    
    let result = match op {
        Operation::Copy => {
            if buffer_count >= 2 {
                let mut temp = Buffer::new();
                match buffer_copy(&buffers.buffers[0], &mut temp) {
                    Ok(()) => {
                        write_buffer(&temp, &mut stdout_lock);
                        Ok(())
                    }
                    Err(()) => Err(()),
                }
            } else {
                eprintln!("Error: Copy needs at least 2 buffers");
                Err(())
            }
        }
        Operation::Reverse => {
            let mut res = Ok(());
            for i in 0..buffers.count {
                if buffer_reverse(&mut buffers.buffers[i]).is_err() {
                    res = Err(());
                    break;
                }
                write_buffer(&buffers.buffers[i], &mut stdout_lock);
            }
            res
        }
        Operation::Merge => {
            if buffer_count >= 2 {
                let mut merged = Buffer::new();
                match buffer_merge(&buffers.buffers[0], &buffers.buffers[1], &mut merged) {
                    Ok(()) => {
                        write_buffer(&merged, &mut stdout_lock);
                        Ok(())
                    }
                    Err(()) => Err(()),
                }
            } else {
                eprintln!("Error: Merge needs at least 2 buffers");
                Err(())
            }
        }
        Operation::Split => {
            if buffer_count >= 1 {
                line.clear();
                if stdin_lock.read_line(&mut line).is_err() {
                    eprintln!("Error: Failed to read split position");
                    Err(())
                } else {
                    let split_pos: usize = line.trim().parse().unwrap_or(MAX_BUFFER_SIZE + 1);
                    let mut part1 = Buffer::new();
                    let mut part2 = Buffer::new();
                    match buffer_split(&buffers.buffers[0], split_pos, &mut part1, &mut part2) {
                        Ok(()) => {
                            write_buffer(&part1, &mut stdout_lock);
                            write_buffer(&part2, &mut stdout_lock);
                            Ok(())
                        }
                        Err(()) => Err(()),
                    }
                }
            } else {
                Ok(())
            }
        }
        Operation::Interleave => {
            if buffer_count >= 2 {
                let mut interleaved = Buffer::new();
                match buffer_interleave(&buffers.buffers[0], &buffers.buffers[1], &mut interleaved) {
                    Ok(()) => {
                        write_buffer(&interleaved, &mut stdout_lock);
                        Ok(())
                    }
                    Err(()) => Err(()),
                }
            } else {
                eprintln!("Error: Interleave needs at least 2 buffers");
                Err(())
            }
        }
        Operation::Rotate => {
            line.clear();
            if stdin_lock.read_line(&mut line).is_err() {
                eprintln!("Error: Failed to read rotation amount");
                Err(())
            } else {
                let positions: i32 = line.trim().parse().unwrap_or(0);
                let mut res = Ok(());
                for i in 0..buffers.count {
                    if buffer_rotate(&mut buffers.buffers[i], positions).is_err() {
                        res = Err(());
                        break;
                    }
                    write_buffer(&buffers.buffers[i], &mut stdout_lock);
                }
                res
            }
        }
        Operation::Checksum => {
            for i in 0..buffers.count {
                let _ = writeln!(stdout_lock, "{}", buffers.buffers[i].checksum);
            }
            Ok(())
        }
    };
    
    if result.is_err() {
        std::process::exit(1);
    }
}
