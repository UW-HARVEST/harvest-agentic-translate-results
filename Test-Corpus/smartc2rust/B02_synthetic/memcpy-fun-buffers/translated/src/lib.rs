
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::{c_char, c_int};

#[derive(Clone)]
struct Buffer {
    data: Vec<u8>,
    checksum: u32,
}

impl Buffer {
    fn new() -> Self {
        Buffer { data: Vec::new(), checksum: 0 }
    }

    fn from_data(data: Vec<u8>) -> Self {
        let checksum = calculate_checksum(&data);
        Buffer { data, checksum }
    }

    fn recompute_checksum(&mut self) {
        self.checksum = calculate_checksum(&self.data);
    }
}

const MAX_BUFFER_SIZE: usize = 256;
const MAX_BUFFER_COUNT: i32 = 100;

#[derive(Copy, Clone, PartialEq, Eq)]
enum Operation {
    Copy,
    Reverse,
    Merge,
    Split,
    Interleave,
    Rotate,
    Checksum,
}

fn operation_from_i32(v: i32) -> Option<Operation> {
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

fn calculate_checksum(data: &[u8]) -> u32 {
    data.iter().fold(0u32, |sum, &b| sum.wrapping_shl(3) ^ (b as u32))
}

fn validate_buffer(buf: &Buffer) -> bool {
    if buf.data.len() > MAX_BUFFER_SIZE {
        eprintln!("Error: Buffer length {} exceeds maximum {}", buf.data.len(), MAX_BUFFER_SIZE);
        return false;
    }
    let expected = calculate_checksum(&buf.data);
    if buf.checksum != expected {
        eprintln!("Warning: Checksum mismatch. Expected {}, got {}", expected, buf.checksum);
    }
    true
}

fn buffer_copy(src: &Buffer) -> Result<Buffer, String> {
    if !validate_buffer(src) {
        return Err("validation failed".to_string());
    }
    Ok(Buffer::from_data(src.data.clone()))
}

fn buffer_interleave(src1: &Buffer, src2: &Buffer) -> Result<Buffer, String> {
    if src1.data.len() + src2.data.len() > MAX_BUFFER_SIZE {
        eprintln!("Error: Interleaved length exceeds maximum");
        return Err("interleave overflow".to_string());
    }
    let max_len = src1.data.len().max(src2.data.len());
    let mut out = Vec::with_capacity(src1.data.len() + src2.data.len());
    for i in 0..max_len {
        if let Some(&b) = src1.data.get(i) {
            out.push(b);
        }
        if let Some(&b) = src2.data.get(i) {
            out.push(b);
        }
    }
    Ok(Buffer::from_data(out))
}

fn buffer_merge(src1: &Buffer, src2: &Buffer) -> Result<Buffer, String> {
    let total = src1.data.len() + src2.data.len();
    if total > MAX_BUFFER_SIZE {
        eprintln!("Error: Merged length {} exceeds maximum", total);
        return Err("merge overflow".to_string());
    }
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&src1.data);
    out.extend_from_slice(&src2.data);
    Ok(Buffer::from_data(out))
}

fn buffer_reverse(buf: &mut Buffer) {
    buf.data.reverse();
    buf.recompute_checksum();
}

fn buffer_rotate(buf: &mut Buffer, positions: i32) {
    if buf.data.is_empty() || positions == 0 {
        return;
    }
    let len = buf.data.len() as i32;
    let mut p = positions % len;
    if p < 0 {
        p += len;
    }
    buf.data.rotate_left(p as usize);
    buf.recompute_checksum();
}

fn buffer_split(src: &Buffer, split_pos: usize) -> Result<(Buffer, Buffer), String> {
    if split_pos > src.data.len() {
        eprintln!("Error: Split position {} exceeds length {}", split_pos, src.data.len());
        return Err("split out of range".to_string());
    }
    let (a, b) = src.data.split_at(split_pos);
    Ok((Buffer::from_data(a.to_vec()), Buffer::from_data(b.to_vec())))
}

fn read_int_token(iter: &mut std::str::SplitAsciiWhitespace) -> Option<i32> {
    iter.next()?.parse::<i32>().ok()
}

fn read_buffer(iter: &mut std::str::SplitAsciiWhitespace) -> Result<Buffer, String> {
    let length = read_int_token(iter).ok_or_else(|| {
        eprintln!("Error: Failed to read buffer length");
        "length read failed".to_string()
    })?;
    if !(0..=MAX_BUFFER_SIZE as i32).contains(&length) {
        eprintln!("Error: Invalid buffer length {}", length);
        return Err("invalid length".to_string());
    }
    let mut data = Vec::with_capacity(length as usize);
    for i in 0..length as usize {
        let byte = read_int_token(iter).ok_or_else(|| {
            eprintln!("Error: Failed to read byte {}", i);
            "byte read failed".to_string()
        })?;
        data.push(byte as u8);
    }
    Ok(Buffer::from_data(data))
}

fn write_buffer(buf: &Buffer) {
    let mut out = String::new();
    out.push_str(&buf.data.len().to_string());
    for b in &buf.data {
        out.push(' ');
        out.push_str(&b.to_string());
    }
    println!("{}", out);
}

fn run() -> i32 {
    use std::io::Read;
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        eprintln!("Error: Failed to read stdin");
        return 1;
    }
    let mut iter = input.split_ascii_whitespace();

    let operation_raw = match read_int_token(&mut iter) {
        Some(v) => v,
        None => {
            eprintln!("Error: Failed to read operation");
            return 1;
        }
    };

    let buffer_count = match read_int_token(&mut iter) {
        Some(v) => v,
        None => {
            eprintln!("Error: Failed to read buffer count");
            return 1;
        }
    };

    if buffer_count <= 0 || buffer_count > MAX_BUFFER_COUNT {
        eprintln!("Error: Invalid buffer count {}", buffer_count);
        return 1;
    }

    let mut buffers: Vec<Buffer> = Vec::with_capacity(buffer_count as usize);
    for _ in 0..buffer_count {
        match read_buffer(&mut iter) {
            Ok(b) => buffers.push(b),
            Err(_) => return 1,
        }
    }

    let op = match operation_from_i32(operation_raw) {
        Some(op) => op,
        None => {
            eprintln!("Error: Unknown operation {}", operation_raw);
            return 1;
        }
    };

    let result: Result<(), String> = match op {
        Operation::Copy => {
            if buffers.len() >= 2 {
                match buffer_copy(&buffers[0]) {
                    Ok(temp) => {
                        write_buffer(&temp);
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            } else {
                eprintln!("Error: Copy needs at least 2 buffers");
                Err("insufficient buffers".to_string())
            }
        }
        Operation::Reverse => {
            for buf in buffers.iter_mut() {
                buffer_reverse(buf);
                write_buffer(buf);
            }
            Ok(())
        }
        Operation::Merge => {
            if buffers.len() >= 2 {
                match buffer_merge(&buffers[0], &buffers[1]) {
                    Ok(merged) => {
                        write_buffer(&merged);
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            } else {
                eprintln!("Error: Merge needs at least 2 buffers");
                Err("insufficient buffers".to_string())
            }
        }
        Operation::Split => {
            if !buffers.is_empty() {
                let split_pos = match read_int_token(&mut iter) {
                    Some(v) => v,
                    None => {
                        eprintln!("Error: Failed to read split position");
                        return 1;
                    }
                };
                match buffer_split(&buffers[0], split_pos as usize) {
                    Ok((a, b)) => {
                        write_buffer(&a);
                        write_buffer(&b);
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            } else {
                Ok(())
            }
        }
        Operation::Interleave => {
            if buffers.len() >= 2 {
                match buffer_interleave(&buffers[0], &buffers[1]) {
                    Ok(inter) => {
                        write_buffer(&inter);
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            } else {
                eprintln!("Error: Interleave needs at least 2 buffers");
                Err("insufficient buffers".to_string())
            }
        }
        Operation::Rotate => {
            let positions = match read_int_token(&mut iter) {
                Some(v) => v,
                None => {
                    eprintln!("Error: Failed to read rotation amount");
                    return 1;
                }
            };
            for buf in buffers.iter_mut() {
                buffer_rotate(buf, positions);
                write_buffer(buf);
            }
            Ok(())
        }
        Operation::Checksum => {
            for buf in &buffers {
                println!("{}", buf.checksum);
            }
            Ok(())
        }
    };

    if result.is_err() { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main(_argc: c_int, _argv: *mut *mut c_char) -> c_int {
    run()
}