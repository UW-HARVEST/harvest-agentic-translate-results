use std::io::{self, BufRead, Write};
use std::process;

struct Buffer {
    data: [u8; 256],
    length: usize,
    checksum: u32,
}

impl Buffer {
    fn new() -> Self {
        Buffer {
            data: [0u8; 256],
            length: 0,
            checksum: 0,
        }
    }
}

struct BufferArray {
    buffers: Vec<Buffer>,
    count: i32,
}

const OP_COPY: i32 = 0;
const OP_REVERSE: i32 = 1;
const OP_MERGE: i32 = 2;
const OP_SPLIT: i32 = 3;
const OP_INTERLEAVE: i32 = 4;
const OP_ROTATE: i32 = 5;
const OP_CHECKSUM: i32 = 6;

fn calculate_checksum(data: &[u8], length: usize) -> u32 {
    let mut sum: u32 = 0;
    for i in 0..length {
        sum = (sum << 3) ^ data[i] as u32;
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
    let mut temp = [0u8; 256];
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
    positions = positions % buf.length as i32;
    if positions < 0 {
        positions += buf.length as i32;
    }
    let positions = positions as usize;
    let mut temp = [0u8; 256];
    temp[..buf.length].copy_from_slice(&buf.data[..buf.length]);
    buf.data[..buf.length - positions]
        .copy_from_slice(&temp[positions..buf.length]);
    buf.data[buf.length - positions..buf.length]
        .copy_from_slice(&temp[..positions]);
    buf.checksum = calculate_checksum(&buf.data, buf.length);
    0
}

// Scanner that mimics C's scanf("%d") behavior: reads whitespace-delimited integers
struct Scanner {
    tokens: Vec<String>,
    pos: usize,
}

impl Scanner {
    fn new() -> Self {
        let stdin = io::stdin();
        let mut tokens = Vec::new();
        for line in stdin.lock().lines() {
            let line = line.unwrap_or_default();
            for tok in line.split_whitespace() {
                tokens.push(tok.to_string());
            }
        }
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

fn write_buffer(buf: &Buffer) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    write!(out, "{}", buf.length).unwrap();
    for i in 0..buf.length {
        write!(out, " {}", buf.data[i]).unwrap();
    }
    writeln!(out).unwrap();
}

fn main() {
    let mut scanner = Scanner::new();

    let operation = match scanner.next_int() {
        Some(v) => v,
        None => {
            eprintln!("Error: Failed to read operation");
            process::exit(1);
        }
    };

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

    let mut arr = match init_buffer_array(buffer_count) {
        Some(a) => a,
        None => process::exit(1),
    };

    for i in 0..buffer_count as usize {
        // read_buffer inline
        let length = match scanner.next_int() {
            Some(v) => v,
            None => {
                eprintln!("Error: Failed to read buffer length");
                process::exit(1);
            }
        };
        if length < 0 || length > 256 {
            eprintln!("Error: Invalid buffer length {}", length);
            process::exit(1);
        }
        arr.buffers[i].length = length as usize;
        for j in 0..arr.buffers[i].length {
            let byte_val = match scanner.next_int() {
                Some(v) => v,
                None => {
                    eprintln!("Error: Failed to read byte {}", j);
                    process::exit(1);
                }
            };
            arr.buffers[i].data[j] = byte_val as u8;
        }
        arr.buffers[i].checksum =
            calculate_checksum(&arr.buffers[i].data, arr.buffers[i].length);
        arr.count += 1;
    }

    let mut result: i32 = 0;

    match operation {
        OP_COPY => {
            if buffer_count >= 2 {
                let mut temp = Buffer::new();
                result = buffer_copy(&arr.buffers[0], &mut temp);
                if result == 0 {
                    write_buffer(&temp);
                }
            } else {
                eprintln!("Error: Copy needs at least 2 buffers");
                result = -1;
            }
        }
        OP_REVERSE => {
            for i in 0..buffer_count as usize {
                result = buffer_reverse(&mut arr.buffers[i]);
                if result != 0 {
                    break;
                }
                write_buffer(&arr.buffers[i]);
            }
        }
        OP_MERGE => {
            if buffer_count >= 2 {
                let mut merged = Buffer::new();
                result = buffer_merge(&arr.buffers[0], &arr.buffers[1], &mut merged);
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
                        &arr.buffers[0],
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
                result = buffer_interleave(
                    &arr.buffers[0],
                    &arr.buffers[1],
                    &mut interleaved,
                );
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
                for i in 0..buffer_count as usize {
                    result = buffer_rotate(&mut arr.buffers[i], positions);
                    if result != 0 {
                        break;
                    }
                    write_buffer(&arr.buffers[i]);
                }
            }
        }
        OP_CHECKSUM => {
            let stdout = io::stdout();
            let mut out = stdout.lock();
            for i in 0..buffer_count as usize {
                writeln!(out, "{}", arr.buffers[i].checksum).unwrap();
            }
        }
        _ => {
            eprintln!("Error: Unknown operation {}", operation);
            result = -1;
        }
    }

    process::exit(if result != 0 { 1 } else { 0 });
}
