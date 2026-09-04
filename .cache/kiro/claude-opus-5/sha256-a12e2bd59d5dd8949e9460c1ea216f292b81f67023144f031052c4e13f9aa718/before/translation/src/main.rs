// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Rust translation of `c_src/src/main.c`.
//!
//! Behavioral quirks of the original C are reproduced deliberately, not fixed:
//!   * `scanf("%d", ...)` skips arbitrary whitespace (including newlines) and
//!     truncates the parsed `long` to `int`, so out-of-range digits wrap.
//!   * A negative split position is converted to `size_t` before the bounds
//!     check, so it becomes a huge value and is reported as such.
//!   * `buffer_rotate` uses C's truncating `%`, then normalizes negatives.
//!   * `process_buffer_array` is never called by `main` (dead code in the C).

use std::io::{self, Read, Write};

// ==================== Data Structures ====================

#[derive(Clone, Copy)]
struct Buffer {
    data: [u8; 256],
    length: usize,
    checksum: u32,
}

impl Buffer {
    /// Matches the C `buffer_t` declared as an uninitialized local/heap slot.
    /// The original never reads `data` beyond `length`, so zeroing is
    /// observationally equivalent.
    const fn new() -> Self {
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
    #[allow(dead_code)]
    capacity: i32,
}

// operation_t
const OP_COPY: i32 = 0;
const OP_REVERSE: i32 = 1;
const OP_MERGE: i32 = 2;
const OP_SPLIT: i32 = 3;
const OP_INTERLEAVE: i32 = 4;
const OP_ROTATE: i32 = 5;
const OP_CHECKSUM: i32 = 6;

// ==================== stdio emulation ====================

/// Byte-level stdin reader supporting one byte of pushback, mirroring the way
/// `scanf` consumes exactly as much input as it needs.
struct Scanner {
    src: io::Stdin,
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
}

impl Scanner {
    fn new() -> Self {
        Scanner {
            src: io::stdin(),
            buf: Vec::new(),
            pos: 0,
            eof: false,
        }
    }

    fn fill(&mut self) -> bool {
        if self.pos < self.buf.len() {
            return true;
        }
        if self.eof {
            return false;
        }
        let mut chunk = [0u8; 4096];
        loop {
            match self.src.read(&mut chunk) {
                Ok(0) => {
                    self.eof = true;
                    return false;
                }
                Ok(n) => {
                    self.buf.clear();
                    self.buf.extend_from_slice(&chunk[..n]);
                    self.pos = 0;
                    return true;
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return false;
                }
            }
        }
    }

    fn peek(&mut self) -> Option<u8> {
        if self.fill() {
            Some(self.buf[self.pos])
        } else {
            None
        }
    }

    fn bump(&mut self) {
        self.pos += 1;
    }

    /// Equivalent of `scanf("%d", &out)`: returns `None` on a matching failure
    /// or EOF (both of which the caller treats as "!= 1").
    ///
    /// glibc converts the digit sequence as a `long` (saturating at
    /// `LONG_MIN`/`LONG_MAX`) and then stores the low 32 bits into the `int`,
    /// which is what the truncating cast below reproduces.
    fn scan_int(&mut self) -> Option<i32> {
        // Skip leading whitespace, exactly as the %d directive does.
        loop {
            match self.peek() {
                Some(c) if is_c_space(c) => self.bump(),
                Some(_) => break,
                None => return None,
            }
        }

        let mut negative = false;
        match self.peek() {
            Some(b'-') => {
                negative = true;
                self.bump();
            }
            Some(b'+') => self.bump(),
            _ => {}
        }

        let mut saw_digit = false;
        let mut acc: i64 = 0;
        let mut saturated = false;
        while let Some(c) = self.peek() {
            if !c.is_ascii_digit() {
                break;
            }
            self.bump();
            saw_digit = true;
            let d = i64::from(c - b'0');
            if !saturated {
                match acc.checked_mul(10).and_then(|v| {
                    if negative {
                        v.checked_sub(d)
                    } else {
                        v.checked_add(d)
                    }
                }) {
                    Some(v) => acc = v,
                    None => saturated = true,
                }
            }
        }

        if !saw_digit {
            return None;
        }

        let as_long = if saturated {
            if negative {
                i64::MIN
            } else {
                i64::MAX
            }
        } else {
            acc
        };

        Some(as_long as i32)
    }
}

fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Block-buffered stdout, flushed once before exit, matching C's behavior when
/// stdout is a pipe or file.
struct Out {
    buf: Vec<u8>,
}

impl Out {
    fn new() -> Self {
        Out {
            buf: Vec::with_capacity(64 * 1024),
        }
    }

    fn write_str(&mut self, s: &str) {
        self.buf.extend_from_slice(s.as_bytes());
    }

    fn flush(&mut self) {
        let stdout = io::stdout();
        let mut lock = stdout.lock();
        let _ = lock.write_all(&self.buf);
        let _ = lock.flush();
        self.buf.clear();
    }
}

fn err(msg: &str) {
    let stderr = io::stderr();
    let mut lock = stderr.lock();
    let _ = lock.write_all(msg.as_bytes());
    let _ = lock.flush();
}

// ==================== Helper Functions ====================

// Calculate simple checksum
fn calculate_checksum(data: &[u8], length: usize) -> u32 {
    let mut sum: u32 = 0;
    for i in 0..length {
        sum = (sum << 3) ^ u32::from(data[i]);
    }
    sum
}

// Validate buffer integrity
fn validate_buffer(buf: &Buffer) -> bool {
    // The NULL check in the C cannot trigger for a reference.
    if buf.length > 256 {
        err(&format!(
            "Error: Buffer length {} exceeds maximum 256\n",
            buf.length
        ));
        return false;
    }
    let expected = calculate_checksum(&buf.data, buf.length);
    if buf.checksum != expected {
        err(&format!(
            "Warning: Checksum mismatch. Expected {}, got {}\n",
            expected, buf.checksum
        ));
    }
    true
}

// Initialize buffer array
fn init_buffer_array(initial_capacity: i32) -> Option<BufferArray> {
    if initial_capacity <= 0 {
        err(&format!("Error: Invalid capacity {}\n", initial_capacity));
        return None;
    }

    Some(BufferArray {
        buffers: vec![Buffer::new(); initial_capacity as usize],
        count: 0,
        capacity: initial_capacity,
    })
}

// ==================== Core Buffer Operations ====================

// Simple copy operation with memcpy
fn buffer_copy(src: &Buffer, dst: &mut Buffer) -> i32 {
    if !validate_buffer(src) {
        return -1;
    }

    dst.data[..src.length].copy_from_slice(&src.data[..src.length]);
    dst.length = src.length;
    dst.checksum = calculate_checksum(&dst.data, dst.length);

    0
}

// Reverse buffer contents
fn buffer_reverse(buf: &mut Buffer) -> i32 {
    if buf.length == 0 {
        return 0; // Nothing to reverse
    }

    let mut temp = [0u8; 256];
    temp[..buf.length].copy_from_slice(&buf.data[..buf.length]);

    for i in 0..buf.length {
        buf.data[i] = temp[buf.length - 1 - i];
    }

    buf.checksum = calculate_checksum(&buf.data, buf.length);
    0
}

// Merge two buffers into destination
fn buffer_merge(src1: &Buffer, src2: &Buffer, dst: &mut Buffer) -> i32 {
    if src1.length + src2.length > 256 {
        err(&format!(
            "Error: Merged length {} exceeds maximum\n",
            src1.length + src2.length
        ));
        return -1;
    }

    // Copy first buffer
    dst.data[..src1.length].copy_from_slice(&src1.data[..src1.length]);
    // Copy second buffer after first
    dst.data[src1.length..src1.length + src2.length].copy_from_slice(&src2.data[..src2.length]);

    dst.length = src1.length + src2.length;
    dst.checksum = calculate_checksum(&dst.data, dst.length);

    0
}

// Split buffer at position into two buffers
fn buffer_split(src: &Buffer, split_pos: usize, dst1: &mut Buffer, dst2: &mut Buffer) -> i32 {
    if split_pos > src.length {
        err(&format!(
            "Error: Split position {} exceeds length {}\n",
            split_pos, src.length
        ));
        return -1;
    }

    // Copy first part
    if split_pos > 0 {
        dst1.data[..split_pos].copy_from_slice(&src.data[..split_pos]);
    }
    dst1.length = split_pos;
    dst1.checksum = calculate_checksum(&dst1.data, dst1.length);

    // Copy second part
    let remaining = src.length - split_pos;
    if remaining > 0 {
        dst2.data[..remaining].copy_from_slice(&src.data[split_pos..split_pos + remaining]);
    }
    dst2.length = remaining;
    dst2.checksum = calculate_checksum(&dst2.data, dst2.length);

    0
}

// Interleave two buffers (alternating bytes)
fn buffer_interleave(src1: &Buffer, src2: &Buffer, dst: &mut Buffer) -> i32 {
    let max_len = if src1.length > src2.length {
        src1.length
    } else {
        src2.length
    };
    if src1.length + src2.length > 256 {
        err("Error: Interleaved length exceeds maximum\n");
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

// Rotate buffer left by n positions
fn buffer_rotate(buf: &mut Buffer, positions: i32) -> i32 {
    if buf.length == 0 || positions == 0 {
        return 0; // Nothing to rotate
    }

    // Normalize positions to valid range (C's % truncates toward zero)
    let mut positions = positions.wrapping_rem(buf.length as i32);
    if positions < 0 {
        positions += buf.length as i32;
    }
    let positions = positions as usize;

    let mut temp = [0u8; 256];
    temp[..buf.length].copy_from_slice(&buf.data[..buf.length]);

    // Copy rotated portions
    let tail = buf.length - positions;
    for i in 0..tail {
        buf.data[i] = temp[positions + i];
    }
    for i in 0..positions {
        buf.data[tail + i] = temp[i];
    }

    buf.checksum = calculate_checksum(&buf.data, buf.length);

    0
}

// Conditional copy based on pattern matching
#[allow(dead_code)]
fn buffer_conditional_copy(
    src: &Buffer,
    dst: &mut Buffer,
    pattern: u8,
    copy_matching: bool,
) -> i32 {
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

// Copy with stride (every nth byte)
#[allow(dead_code)]
fn buffer_copy_strided(src: &Buffer, dst: &mut Buffer, stride: i32) -> i32 {
    if stride <= 0 {
        err(&format!("Error: Invalid stride {}\n", stride));
        return -1;
    }

    let mut dst_pos = 0usize;
    let mut i = 0usize;
    while i < src.length {
        dst.data[dst_pos] = src.data[i];
        dst_pos += 1;
        i += stride as usize;
    }

    dst.length = dst_pos;
    dst.checksum = calculate_checksum(&dst.data, dst.length);

    0
}

// ==================== Complex Processing Functions ====================

// Process buffer array with operation (unused by main, as in the C source)
#[allow(dead_code)]
fn process_buffer_array(arr: &mut BufferArray, op: i32, param: i32) -> i32 {
    if arr.count == 0 {
        err("Error: Invalid buffer array\n");
        return -1;
    }

    match op {
        OP_COPY => {
            // Copy first buffer to all others
            for i in 1..arr.count as usize {
                let src = arr.buffers[0];
                if buffer_copy(&src, &mut arr.buffers[i]) != 0 {
                    return -1;
                }
            }
        }

        OP_REVERSE => {
            // Reverse all buffers
            for i in 0..arr.count as usize {
                if buffer_reverse(&mut arr.buffers[i]) != 0 {
                    return -1;
                }
            }
        }

        OP_MERGE => {
            // Merge consecutive pairs
            if arr.count < 2 {
                err("Error: Need at least 2 buffers for merge\n");
                return -1;
            }
            let mut i = 0i32;
            while i < arr.count - 1 {
                let mut merged = Buffer::new();
                let a = arr.buffers[i as usize];
                let b = arr.buffers[i as usize + 1];
                if buffer_merge(&a, &b, &mut merged) != 0 {
                    return -1;
                }
                arr.buffers[i as usize] = merged;
                i += 2;
            }
        }

        OP_ROTATE => {
            // Rotate all buffers by param positions
            for i in 0..arr.count as usize {
                if buffer_rotate(&mut arr.buffers[i], param) != 0 {
                    return -1;
                }
            }
        }

        OP_CHECKSUM => {
            // Verify all checksums
            for i in 0..arr.count as usize {
                if !validate_buffer(&arr.buffers[i]) {
                    return -1;
                }
            }
        }

        _ => {
            err(&format!("Error: Unknown operation {}\n", op));
            return -1;
        }
    }

    0
}

// ==================== Input/Output Functions ====================

// Read buffer from stdin
fn read_buffer(buf: &mut Buffer, sc: &mut Scanner) -> i32 {
    let length = match sc.scan_int() {
        Some(v) => v,
        None => {
            err("Error: Failed to read buffer length\n");
            return -1;
        }
    };

    if length < 0 || length > 256 {
        err(&format!("Error: Invalid buffer length {}\n", length));
        return -1;
    }

    buf.length = length as usize;
    for i in 0..buf.length {
        let byte = match sc.scan_int() {
            Some(v) => v,
            None => {
                err(&format!("Error: Failed to read byte {}\n", i));
                return -1;
            }
        };
        buf.data[i] = byte as u8;
    }

    buf.checksum = calculate_checksum(&buf.data, buf.length);
    0
}

// Write buffer to stdout
fn write_buffer(buf: &Buffer, out: &mut Out) {
    out.write_str(&format!("{}", buf.length));
    for i in 0..buf.length {
        out.write_str(&format!(" {}", buf.data[i]));
    }
    out.write_str("\n");
}

// ==================== Main Function ====================

fn run(sc: &mut Scanner, out: &mut Out) -> i32 {
    // Read operation type
    let operation = match sc.scan_int() {
        Some(v) => v,
        None => {
            err("Error: Failed to read operation\n");
            return 1;
        }
    };

    // Read buffer count
    let buffer_count = match sc.scan_int() {
        Some(v) => v,
        None => {
            err("Error: Failed to read buffer count\n");
            return 1;
        }
    };

    if buffer_count <= 0 || buffer_count > 100 {
        err(&format!("Error: Invalid buffer count {}\n", buffer_count));
        return 1;
    }

    // Allocate buffer array
    let mut buffers = match init_buffer_array(buffer_count) {
        Some(b) => b,
        None => return 1,
    };

    // Read all buffers
    for i in 0..buffer_count as usize {
        if read_buffer(&mut buffers.buffers[i], sc) != 0 {
            return 1;
        }
        buffers.count += 1;
    }

    // Execute operation based on type
    let mut result: i32 = 0;
    match operation {
        OP_COPY => {
            if buffer_count >= 2 {
                let mut temp = Buffer::new();
                let src = buffers.buffers[0];
                result = buffer_copy(&src, &mut temp);
                if result == 0 {
                    write_buffer(&temp, out);
                }
            } else {
                err("Error: Copy needs at least 2 buffers\n");
                result = -1;
            }
        }

        OP_REVERSE => {
            for i in 0..buffer_count as usize {
                result = buffer_reverse(&mut buffers.buffers[i]);
                if result != 0 {
                    break;
                }
                write_buffer(&buffers.buffers[i], out);
            }
        }

        OP_MERGE => {
            if buffer_count >= 2 {
                let mut merged = Buffer::new();
                let a = buffers.buffers[0];
                let b = buffers.buffers[1];
                result = buffer_merge(&a, &b, &mut merged);
                if result == 0 {
                    write_buffer(&merged, out);
                }
            } else {
                err("Error: Merge needs at least 2 buffers\n");
                result = -1;
            }
        }

        OP_SPLIT => {
            if buffer_count >= 1 {
                match sc.scan_int() {
                    None => {
                        err("Error: Failed to read split position\n");
                        result = -1;
                    }
                    Some(split_pos) => {
                        let mut part1 = Buffer::new();
                        let mut part2 = Buffer::new();
                        let src = buffers.buffers[0];
                        // int -> size_t conversion sign-extends, exactly as in C.
                        result =
                            buffer_split(&src, split_pos as i64 as usize, &mut part1, &mut part2);
                        if result == 0 {
                            write_buffer(&part1, out);
                            write_buffer(&part2, out);
                        }
                    }
                }
            }
        }

        OP_INTERLEAVE => {
            if buffer_count >= 2 {
                let mut interleaved = Buffer::new();
                let a = buffers.buffers[0];
                let b = buffers.buffers[1];
                result = buffer_interleave(&a, &b, &mut interleaved);
                if result == 0 {
                    write_buffer(&interleaved, out);
                }
            } else {
                err("Error: Interleave needs at least 2 buffers\n");
                result = -1;
            }
        }

        OP_ROTATE => match sc.scan_int() {
            None => {
                err("Error: Failed to read rotation amount\n");
                result = -1;
            }
            Some(positions) => {
                for i in 0..buffer_count as usize {
                    result = buffer_rotate(&mut buffers.buffers[i], positions);
                    if result != 0 {
                        break;
                    }
                    write_buffer(&buffers.buffers[i], out);
                }
            }
        },

        OP_CHECKSUM => {
            for i in 0..buffer_count as usize {
                out.write_str(&format!("{}\n", buffers.buffers[i].checksum));
            }
        }

        _ => {
            err(&format!("Error: Unknown operation {}\n", operation));
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
    let mut sc = Scanner::new();
    let mut out = Out::new();
    let code = run(&mut sc, &mut out);
    out.flush();
    std::process::exit(code);
}
