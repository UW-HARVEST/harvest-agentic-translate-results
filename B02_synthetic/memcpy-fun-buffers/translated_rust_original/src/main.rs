use std::io::{self, Read};
use std::process;

const MAX_BUF_SIZE: usize = 256;

const OP_COPY: i32 = 0;
const OP_REVERSE: i32 = 1;
const OP_MERGE: i32 = 2;
const OP_SPLIT: i32 = 3;
const OP_INTERLEAVE: i32 = 4;
const OP_ROTATE: i32 = 5;
const OP_CHECKSUM: i32 = 6;

// ==================== Scanner (mimics C scanf %d) ====================

struct Scanner {
    tokens: Vec<String>,
    pos: usize,
}

impl Scanner {
    fn new() -> Self {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input).unwrap_or(0);
        let tokens: Vec<String> = input.split_whitespace().map(|s| s.to_string()).collect();
        Scanner { tokens, pos: 0 }
    }

    fn next_int(&mut self) -> Option<i32> {
        if self.pos < self.tokens.len() {
            let val = self.tokens[self.pos].parse::<i32>().ok();
            if val.is_some() {
                self.pos += 1;
            }
            val
        } else {
            None
        }
    }
}

// ==================== Data Structures ====================

#[derive(Clone)]
struct Buffer {
    data: [u8; MAX_BUF_SIZE],
    length: usize,
    checksum: u32,
}

impl Buffer {
    fn new() -> Self {
        Buffer {
            data: [0u8; MAX_BUF_SIZE],
            length: 0,
            checksum: 0,
        }
    }
}

struct BufferArray {
    buffers: Vec<Buffer>,
    count: i32,
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

    let mut buffers = Vec::with_capacity(initial_capacity as usize);
    for _ in 0..initial_capacity {
        buffers.push(Buffer::new());
    }

    Some(BufferArray {
        buffers,
        count: 0,
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

    let mut temp = [0u8; MAX_BUF_SIZE];
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

fn buffer_rotate(buf: &mut Buffer, mut positions: i32) -> i32 {
    if buf.length == 0 || positions == 0 {
        return 0;
    }

    positions = positions % (buf.length as i32);
    if positions < 0 {
        positions += buf.length as i32;
    }
    let positions = positions as usize;

    let mut temp = [0u8; MAX_BUF_SIZE];
    temp[..buf.length].copy_from_slice(&buf.data[..buf.length]);

    buf.data[..buf.length - positions]
        .copy_from_slice(&temp[positions..buf.length]);
    buf.data[buf.length - positions..buf.length]
        .copy_from_slice(&temp[..positions]);

    buf.checksum = calculate_checksum(&buf.data, buf.length);
    0
}

// ==================== I/O Functions ====================

fn read_buffer(buf: &mut Buffer, scanner: &mut Scanner) -> i32 {
    let length = match scanner.next_int() {
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
        let byte = match scanner.next_int() {
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

// ==================== Main ====================

fn main() {
    let mut scanner = Scanner::new();

    // Read operation type
    let operation = match scanner.next_int() {
        Some(v) => v,
        None => {
            eprintln!("Error: Failed to read operation");
            process::exit(1);
        }
    };

    // Read buffer count
    let buffer_count = match scanner.next_int() {
        Some(v) => v,
        None => {
            eprintln!("Error: Failed to read buffer count");
            process::exit(1);
        }
    };

    if buffer_count <= 0 || buffer_count > 100 {
        eprintln!("Error: Invalid buffer count {}", buffer_count);
        process::exit(1);
    }

    // Allocate buffer array
    let mut buffers = match init_buffer_array(buffer_count) {
        Some(b) => b,
        None => process::exit(1),
    };

    // Read all buffers
    for i in 0..buffer_count {
        if read_buffer(&mut buffers.buffers[i as usize], &mut scanner) != 0 {
            process::exit(1);
        }
        buffers.count += 1;
    }

    // Execute operation
    let mut result: i32 = 0;
    match operation {
        OP_COPY => {
            if buffer_count >= 2 {
                let mut temp = Buffer::new();
                result = buffer_copy(&buffers.buffers[0], &mut temp);
                if result == 0 {
                    write_buffer(&temp);
                }
            } else {
                eprintln!("Error: Copy needs at least 2 buffers");
                result = -1;
            }
        }
        OP_REVERSE => {
            for i in 0..buffer_count {
                result = buffer_reverse(&mut buffers.buffers[i as usize]);
                if result != 0 {
                    break;
                }
                write_buffer(&buffers.buffers[i as usize]);
            }
        }
        OP_MERGE => {
            if buffer_count >= 2 {
                let mut merged = Buffer::new();
                let (left, right) = buffers.buffers.split_at(1);
                result = buffer_merge(&left[0], &right[0], &mut merged);
                if result == 0 {
                    write_buffer(&merged);
                }
            } else {
                eprintln!("Error: Merge needs at least 2 buffers");
                result = -1;
            }
        }
        OP_SPLIT => {
            if buffer_count >= 1 {
                let split_pos = match scanner.next_int() {
                    Some(v) => v,
                    None => {
                        eprintln!("Error: Failed to read split position");
                        result = -1;
                        -1
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
                        write_buffer(&part1);
                        write_buffer(&part2);
                    }
                }
            }
        }
        OP_INTERLEAVE => {
            if buffer_count >= 2 {
                let mut interleaved = Buffer::new();
                let (left, right) = buffers.buffers.split_at(1);
                result = buffer_interleave(&left[0], &right[0], &mut interleaved);
                if result == 0 {
                    write_buffer(&interleaved);
                }
            } else {
                eprintln!("Error: Interleave needs at least 2 buffers");
                result = -1;
            }
        }
        OP_ROTATE => {
            let positions = match scanner.next_int() {
                Some(v) => v,
                None => {
                    eprintln!("Error: Failed to read rotation amount");
                    result = -1;
                    0
                }
            };
            if result == 0 {
                for i in 0..buffer_count {
                    result = buffer_rotate(&mut buffers.buffers[i as usize], positions);
                    if result != 0 {
                        break;
                    }
                    write_buffer(&buffers.buffers[i as usize]);
                }
            }
        }
        OP_CHECKSUM => {
            for i in 0..buffer_count {
                println!("{}", buffers.buffers[i as usize].checksum);
            }
        }
        _ => {
            eprintln!("Error: Unknown operation {}", operation);
            result = -1;
        }
    }

    if result != 0 {
        process::exit(1);
    }
}
