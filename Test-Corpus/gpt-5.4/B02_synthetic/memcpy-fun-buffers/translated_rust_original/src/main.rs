use std::io::{self, Read};

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

struct BufferArray {
    buffers: Vec<Buffer>,
    count: i32,
    capacity: i32,
}

#[derive(Clone, Copy)]
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

fn calculate_checksum(data: &[u8], length: usize) -> u32 {
    let mut sum = 0u32;
    for &b in data.iter().take(length) {
        sum = (sum << 3) ^ b as u32;
    }
    sum
}

fn validate_buffer(buf: &Buffer) -> bool {
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
        buffers: vec![Buffer::default(); initial_capacity as usize],
        count: 0,
        capacity: initial_capacity,
    })
}

fn free_buffer_array(_arr: BufferArray) {}

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
    buf.data[..buf.length].reverse();
    buf.checksum = calculate_checksum(&buf.data, buf.length);
    0
}

fn buffer_merge(src1: &Buffer, src2: &Buffer, dst: &mut Buffer) -> i32 {
    if src1.length + src2.length > 256 {
        eprintln!("Error: Merged length {} exceeds maximum", src1.length + src2.length);
        return -1;
    }
    dst.data[..src1.length].copy_from_slice(&src1.data[..src1.length]);
    dst.data[src1.length..src1.length + src2.length].copy_from_slice(&src2.data[..src2.length]);
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

fn buffer_rotate(buf: &mut Buffer, positions: i32) -> i32 {
    if buf.length == 0 || positions == 0 {
        return 0;
    }
    let len = buf.length as i32;
    let mut positions = positions % len;
    if positions < 0 {
        positions += len;
    }
    let positions = positions as usize;
    buf.data[..buf.length].rotate_left(positions);
    buf.checksum = calculate_checksum(&buf.data, buf.length);
    0
}

fn buffer_conditional_copy(src: &Buffer, dst: &mut Buffer, pattern: u8, copy_matching: bool) -> i32 {
    let mut dst_pos = 0usize;
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

fn buffer_copy_strided(src: &Buffer, dst: &mut Buffer, stride: i32) -> i32 {
    if stride <= 0 {
        eprintln!("Error: Invalid stride {}", stride);
        return -1;
    }
    let mut dst_pos = 0usize;
    let stride = stride as usize;
    let mut i = 0usize;
    while i < src.length {
        dst.data[dst_pos] = src.data[i];
        dst_pos += 1;
        i += stride;
    }
    dst.length = dst_pos;
    dst.checksum = calculate_checksum(&dst.data, dst.length);
    0
}

fn process_buffer_array(arr: &mut BufferArray, op: Operation, param: i32) -> i32 {
    if arr.count == 0 {
        eprintln!("Error: Invalid buffer array");
        return -1;
    }
    match op {
        Operation::Copy => {
            for i in 1..arr.count as usize {
                let src = arr.buffers[0];
                if buffer_copy(&src, &mut arr.buffers[i]) != 0 {
                    return -1;
                }
            }
        }
        Operation::Reverse => {
            for i in 0..arr.count as usize {
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
            while i + 1 < arr.count as usize {
                let mut merged = Buffer::default();
                let left = arr.buffers[i];
                let right = arr.buffers[i + 1];
                if buffer_merge(&left, &right, &mut merged) != 0 {
                    return -1;
                }
                arr.buffers[i] = merged;
                i += 2;
            }
        }
        Operation::Rotate => {
            for i in 0..arr.count as usize {
                if buffer_rotate(&mut arr.buffers[i], param) != 0 {
                    return -1;
                }
            }
        }
        Operation::Checksum => {
            for i in 0..arr.count as usize {
                if !validate_buffer(&arr.buffers[i]) {
                    return -1;
                }
            }
        }
        _ => {
            eprintln!("Error: Unknown operation {}", op as i32);
            return -1;
        }
    }
    0
}

struct Scanner {
    tokens: Vec<String>,
    index: usize,
}

impl Scanner {
    fn new() -> Result<Self, ()> {
        let mut input = String::new();
        if io::stdin().read_to_string(&mut input).is_err() {
            return Err(());
        }
        Ok(Self {
            tokens: input.split_whitespace().map(|s| s.to_string()).collect(),
            index: 0,
        })
    }

    fn next_i32(&mut self) -> Option<i32> {
        let token = self.tokens.get(self.index)?;
        self.index += 1;
        token.parse::<i32>().ok()
    }
}

fn read_buffer(scanner: &mut Scanner, buf: &mut Buffer) -> i32 {
    let length = match scanner.next_i32() {
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
        let byte = match scanner.next_i32() {
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
    print!("{}", buf.length);
    for i in 0..buf.length {
        print!(" {}", buf.data[i]);
    }
    println!();
}

fn operation_from_i32(value: i32) -> Option<Operation> {
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

fn main() {
    let mut scanner = match Scanner::new() {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Error: Failed to read input");
            std::process::exit(1);
        }
    };

    let operation = match scanner.next_i32() {
        Some(v) => v,
        None => {
            eprintln!("Error: Failed to read operation");
            std::process::exit(1);
        }
    };

    let buffer_count = match scanner.next_i32() {
        Some(v) => v,
        None => {
            eprintln!("Error: Failed to read buffer count");
            std::process::exit(1);
        }
    };

    if buffer_count <= 0 || buffer_count > 100 {
        eprintln!("Error: Invalid buffer count {}", buffer_count);
        std::process::exit(1);
    }

    let mut buffers = match init_buffer_array(buffer_count) {
        Some(b) => b,
        None => std::process::exit(1),
    };

    for i in 0..buffer_count as usize {
        if read_buffer(&mut scanner, &mut buffers.buffers[i]) != 0 {
            free_buffer_array(buffers);
            std::process::exit(1);
        }
        buffers.count += 1;
    }

    let _ = buffers.capacity;
    let _ = buffer_conditional_copy as fn(&Buffer, &mut Buffer, u8, bool) -> i32;
    let _ = buffer_copy_strided as fn(&Buffer, &mut Buffer, i32) -> i32;
    let _ = process_buffer_array as fn(&mut BufferArray, Operation, i32) -> i32;

    let mut result = 0i32;
    match operation_from_i32(operation) {
        Some(Operation::Copy) => {
            if buffer_count >= 2 {
                let mut temp = Buffer::default();
                result = buffer_copy(&buffers.buffers[0], &mut temp);
                if result == 0 {
                    write_buffer(&temp);
                }
            } else {
                eprintln!("Error: Copy needs at least 2 buffers");
                result = -1;
            }
        }
        Some(Operation::Reverse) => {
            for i in 0..buffer_count as usize {
                result = buffer_reverse(&mut buffers.buffers[i]);
                if result != 0 {
                    break;
                }
                write_buffer(&buffers.buffers[i]);
            }
        }
        Some(Operation::Merge) => {
            if buffer_count >= 2 {
                let mut merged = Buffer::default();
                result = buffer_merge(&buffers.buffers[0], &buffers.buffers[1], &mut merged);
                if result == 0 {
                    write_buffer(&merged);
                }
            } else {
                eprintln!("Error: Merge needs at least 2 buffers");
                result = -1;
            }
        }
        Some(Operation::Split) => {
            if buffer_count >= 1 {
                let split_pos = match scanner.next_i32() {
                    Some(v) => v,
                    None => {
                        eprintln!("Error: Failed to read split position");
                        result = -1;
                        0
                    }
                };
                if result == 0 {
                    let mut part1 = Buffer::default();
                    let mut part2 = Buffer::default();
                    result = buffer_split(&buffers.buffers[0], split_pos as usize, &mut part1, &mut part2);
                    if result == 0 {
                        write_buffer(&part1);
                        write_buffer(&part2);
                    }
                }
            }
        }
        Some(Operation::Interleave) => {
            if buffer_count >= 2 {
                let mut interleaved = Buffer::default();
                result = buffer_interleave(&buffers.buffers[0], &buffers.buffers[1], &mut interleaved);
                if result == 0 {
                    write_buffer(&interleaved);
                }
            } else {
                eprintln!("Error: Interleave needs at least 2 buffers");
                result = -1;
            }
        }
        Some(Operation::Rotate) => {
            let positions = match scanner.next_i32() {
                Some(v) => v,
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
                    write_buffer(&buffers.buffers[i]);
                }
            }
        }
        Some(Operation::Checksum) => {
            for i in 0..buffer_count as usize {
                println!("{}", buffers.buffers[i].checksum);
            }
        }
        None => {
            eprintln!("Error: Unknown operation {}", operation);
            result = -1;
        }
    }

    free_buffer_array(buffers);
    if result != 0 {
        std::process::exit(1);
    }
}
