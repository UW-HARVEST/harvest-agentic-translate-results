use std::io::{self, Read, Write};

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

struct Scanner {
    input: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn new() -> io::Result<Self> {
        let mut input = Vec::new();
        io::stdin().read_to_end(&mut input)?;
        Ok(Self { input, pos: 0 })
    }

    fn scan_i32(&mut self) -> Option<i32> {
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }

        if self.pos >= self.input.len() {
            return None;
        }

        let start = self.pos;
        if matches!(self.input[self.pos], b'+' | b'-') {
            self.pos += 1;
        }

        let digits_start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
            self.pos += 1;
        }

        if digits_start == self.pos {
            self.pos = start;
            return None;
        }

        std::str::from_utf8(&self.input[start..self.pos])
            .ok()?
            .parse::<i32>()
            .ok()
    }
}

fn calculate_checksum(data: &[u8], length: usize) -> u32 {
    let mut sum = 0u32;
    for &byte in data.iter().take(length) {
        sum = (sum << 3) ^ u32::from(byte);
    }
    sum
}

fn validate_buffer(buf: Option<&Buffer>, stderr: &mut dyn Write) -> bool {
    let Some(buf) = buf else {
        let _ = writeln!(stderr, "Error: NULL buffer");
        return false;
    };

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

fn init_buffer_array(initial_capacity: i32, stderr: &mut dyn Write) -> Option<BufferArray> {
    if initial_capacity <= 0 {
        let _ = writeln!(stderr, "Error: Invalid capacity {}", initial_capacity);
        return None;
    }

    let capacity = usize::try_from(initial_capacity).ok()?;
    let buffers = vec![Buffer::default(); capacity];
    Some(BufferArray {
        buffers,
        count: 0,
        capacity: initial_capacity,
    })
}

fn buffer_copy(src: Option<&Buffer>, dst: Option<&mut Buffer>, stderr: &mut dyn Write) -> i32 {
    let (Some(src), Some(dst)) = (src, dst) else {
        let _ = writeln!(stderr, "Error: NULL pointer in buffer_copy");
        return -1;
    };

    if !validate_buffer(Some(src), stderr) {
        return -1;
    }

    dst.data[..src.length].copy_from_slice(&src.data[..src.length]);
    dst.length = src.length;
    dst.checksum = calculate_checksum(&dst.data, dst.length);

    0
}

fn buffer_reverse(buf: Option<&mut Buffer>, stderr: &mut dyn Write) -> i32 {
    let Some(buf) = buf else {
        let _ = writeln!(stderr, "Error: NULL buffer in reverse");
        return -1;
    };

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

fn buffer_merge(
    src1: Option<&Buffer>,
    src2: Option<&Buffer>,
    dst: Option<&mut Buffer>,
    stderr: &mut dyn Write,
) -> i32 {
    let (Some(src1), Some(src2), Some(dst)) = (src1, src2, dst) else {
        let _ = writeln!(stderr, "Error: NULL pointer in buffer_merge");
        return -1;
    };

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
    src: Option<&Buffer>,
    split_pos: usize,
    dst1: Option<&mut Buffer>,
    dst2: Option<&mut Buffer>,
    stderr: &mut dyn Write,
) -> i32 {
    let (Some(src), Some(dst1), Some(dst2)) = (src, dst1, dst2) else {
        let _ = writeln!(stderr, "Error: NULL pointer in buffer_split");
        return -1;
    };

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
    src1: Option<&Buffer>,
    src2: Option<&Buffer>,
    dst: Option<&mut Buffer>,
    stderr: &mut dyn Write,
) -> i32 {
    let (Some(src1), Some(src2), Some(dst)) = (src1, src2, dst) else {
        let _ = writeln!(stderr, "Error: NULL pointer in buffer_interleave");
        return -1;
    };

    let max_len = src1.length.max(src2.length);
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

fn buffer_rotate(buf: Option<&mut Buffer>, positions: i32, stderr: &mut dyn Write) -> i32 {
    let Some(buf) = buf else {
        let _ = writeln!(stderr, "Error: NULL buffer in rotate");
        return -1;
    };

    if buf.length == 0 || positions == 0 {
        return 0;
    }

    let mut positions = positions % (buf.length as i32);
    if positions < 0 {
        positions += buf.length as i32;
    }

    let positions = positions as usize;
    let mut temp = [0u8; 256];
    temp[..buf.length].copy_from_slice(&buf.data[..buf.length]);

    let prefix_len = buf.length - positions;
    buf.data[..prefix_len].copy_from_slice(&temp[positions..positions + prefix_len]);
    buf.data[prefix_len..prefix_len + positions].copy_from_slice(&temp[..positions]);
    buf.checksum = calculate_checksum(&buf.data, buf.length);

    0
}

fn buffer_conditional_copy(
    src: Option<&Buffer>,
    dst: Option<&mut Buffer>,
    pattern: u8,
    copy_matching: bool,
    stderr: &mut dyn Write,
) -> i32 {
    let (Some(src), Some(dst)) = (src, dst) else {
        let _ = writeln!(stderr, "Error: NULL pointer in conditional_copy");
        return -1;
    };

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

fn buffer_copy_strided(
    src: Option<&Buffer>,
    dst: Option<&mut Buffer>,
    stride: i32,
    stderr: &mut dyn Write,
) -> i32 {
    let (Some(src), Some(dst)) = (src, dst) else {
        let _ = writeln!(stderr, "Error: NULL pointer in copy_strided");
        return -1;
    };

    if stride <= 0 {
        let _ = writeln!(stderr, "Error: Invalid stride {}", stride);
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

fn process_buffer_array(
    arr: Option<&mut BufferArray>,
    op: i32,
    param: i32,
    stderr: &mut dyn Write,
) -> i32 {
    let Some(arr) = arr else {
        let _ = writeln!(stderr, "Error: Invalid buffer array");
        return -1;
    };

    if arr.count == 0 {
        let _ = writeln!(stderr, "Error: Invalid buffer array");
        return -1;
    }

    match op {
        x if x == Operation::Copy as i32 => {
            for i in 1..arr.count as usize {
                let src = arr.buffers[0].clone();
                if buffer_copy(Some(&src), Some(&mut arr.buffers[i]), stderr) != 0 {
                    return -1;
                }
            }
        }
        x if x == Operation::Reverse as i32 => {
            for i in 0..arr.count as usize {
                if buffer_reverse(Some(&mut arr.buffers[i]), stderr) != 0 {
                    return -1;
                }
            }
        }
        x if x == Operation::Merge as i32 => {
            if arr.count < 2 {
                let _ = writeln!(stderr, "Error: Need at least 2 buffers for merge");
                return -1;
            }
            let mut i = 0usize;
            while i + 1 < arr.count as usize {
                let src1 = arr.buffers[i].clone();
                let src2 = arr.buffers[i + 1].clone();
                let mut merged = Buffer::default();
                if buffer_merge(Some(&src1), Some(&src2), Some(&mut merged), stderr) != 0 {
                    return -1;
                }
                arr.buffers[i] = merged;
                i += 2;
            }
        }
        x if x == Operation::Rotate as i32 => {
            for i in 0..arr.count as usize {
                if buffer_rotate(Some(&mut arr.buffers[i]), param, stderr) != 0 {
                    return -1;
                }
            }
        }
        x if x == Operation::Checksum as i32 => {
            for i in 0..arr.count as usize {
                if !validate_buffer(Some(&arr.buffers[i]), stderr) {
                    return -1;
                }
            }
        }
        _ => {
            let _ = writeln!(stderr, "Error: Unknown operation {}", op);
            return -1;
        }
    }

    0
}

fn read_buffer(buf: Option<&mut Buffer>, scanner: &mut Scanner, stderr: &mut dyn Write) -> i32 {
    let Some(buf) = buf else {
        let _ = writeln!(stderr, "Error: NULL buffer in read_buffer");
        return -1;
    };

    let Some(length) = scanner.scan_i32() else {
        let _ = writeln!(stderr, "Error: Failed to read buffer length");
        return -1;
    };

    if !(0..=256).contains(&length) {
        let _ = writeln!(stderr, "Error: Invalid buffer length {}", length);
        return -1;
    }

    buf.length = length as usize;
    for i in 0..buf.length {
        let Some(byte) = scanner.scan_i32() else {
            let _ = writeln!(stderr, "Error: Failed to read byte {}", i);
            return -1;
        };
        buf.data[i] = byte as u8;
    }

    buf.checksum = calculate_checksum(&buf.data, buf.length);
    0
}

fn write_buffer(buf: Option<&Buffer>, stdout: &mut dyn Write, stderr: &mut dyn Write) {
    let Some(buf) = buf else {
        let _ = writeln!(stderr, "Error: NULL buffer in write_buffer");
        return;
    };

    let _ = write!(stdout, "{}", buf.length);
    for i in 0..buf.length {
        let _ = write!(stdout, " {}", buf.data[i]);
    }
    let _ = writeln!(stdout);
}

fn run(stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let mut scanner = match Scanner::new() {
        Ok(scanner) => scanner,
        Err(_) => return 1,
    };

    let operation = match scanner.scan_i32() {
        Some(value) => value,
        None => {
            let _ = writeln!(stderr, "Error: Failed to read operation");
            return 1;
        }
    };

    let buffer_count = match scanner.scan_i32() {
        Some(value) => value,
        None => {
            let _ = writeln!(stderr, "Error: Failed to read buffer count");
            return 1;
        }
    };

    if buffer_count <= 0 || buffer_count > 100 {
        let _ = writeln!(stderr, "Error: Invalid buffer count {}", buffer_count);
        return 1;
    }

    let mut buffers = match init_buffer_array(buffer_count, stderr) {
        Some(buffers) => buffers,
        None => return 1,
    };

    for i in 0..buffer_count as usize {
        if read_buffer(Some(&mut buffers.buffers[i]), &mut scanner, stderr) != 0 {
            return 1;
        }
        buffers.count += 1;
    }

    let mut result = 0;
    match operation {
        x if x == Operation::Copy as i32 => {
            if buffer_count >= 2 {
                let mut temp = Buffer::default();
                result = buffer_copy(Some(&buffers.buffers[0]), Some(&mut temp), stderr);
                if result == 0 {
                    write_buffer(Some(&temp), stdout, stderr);
                }
            } else {
                let _ = writeln!(stderr, "Error: Copy needs at least 2 buffers");
                result = -1;
            }
        }
        x if x == Operation::Reverse as i32 => {
            for i in 0..buffer_count as usize {
                result = buffer_reverse(Some(&mut buffers.buffers[i]), stderr);
                if result != 0 {
                    break;
                }
                write_buffer(Some(&buffers.buffers[i]), stdout, stderr);
            }
        }
        x if x == Operation::Merge as i32 => {
            if buffer_count >= 2 {
                let mut merged = Buffer::default();
                result = buffer_merge(
                    Some(&buffers.buffers[0]),
                    Some(&buffers.buffers[1]),
                    Some(&mut merged),
                    stderr,
                );
                if result == 0 {
                    write_buffer(Some(&merged), stdout, stderr);
                }
            } else {
                let _ = writeln!(stderr, "Error: Merge needs at least 2 buffers");
                result = -1;
            }
        }
        x if x == Operation::Split as i32 => {
            if buffer_count >= 1 {
                match scanner.scan_i32() {
                    Some(split_pos) => {
                        let mut part1 = Buffer::default();
                        let mut part2 = Buffer::default();
                        result = buffer_split(
                            Some(&buffers.buffers[0]),
                            split_pos as usize,
                            Some(&mut part1),
                            Some(&mut part2),
                            stderr,
                        );
                        if result == 0 {
                            write_buffer(Some(&part1), stdout, stderr);
                            write_buffer(Some(&part2), stdout, stderr);
                        }
                    }
                    None => {
                        let _ = writeln!(stderr, "Error: Failed to read split position");
                        result = -1;
                    }
                }
            }
        }
        x if x == Operation::Interleave as i32 => {
            if buffer_count >= 2 {
                let mut interleaved = Buffer::default();
                result = buffer_interleave(
                    Some(&buffers.buffers[0]),
                    Some(&buffers.buffers[1]),
                    Some(&mut interleaved),
                    stderr,
                );
                if result == 0 {
                    write_buffer(Some(&interleaved), stdout, stderr);
                }
            } else {
                let _ = writeln!(stderr, "Error: Interleave needs at least 2 buffers");
                result = -1;
            }
        }
        x if x == Operation::Rotate as i32 => match scanner.scan_i32() {
            Some(positions) => {
                for i in 0..buffer_count as usize {
                    result = buffer_rotate(Some(&mut buffers.buffers[i]), positions, stderr);
                    if result != 0 {
                        break;
                    }
                    write_buffer(Some(&buffers.buffers[i]), stdout, stderr);
                }
            }
            None => {
                let _ = writeln!(stderr, "Error: Failed to read rotation amount");
                result = -1;
            }
        },
        x if x == Operation::Checksum as i32 => {
            for i in 0..buffer_count as usize {
                let _ = writeln!(stdout, "{}", buffers.buffers[i].checksum);
            }
        }
        _ => {
            let _ = writeln!(stderr, "Error: Unknown operation {}", operation);
            result = -1;
        }
    }

    if result != 0 { 1 } else { 0 }
}

fn main() {
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    let mut stderr = io::BufWriter::new(io::stderr().lock());
    let status = run(&mut stdout, &mut stderr);
    let _ = stdout.flush();
    let _ = stderr.flush();
    std::process::exit(status);
}
