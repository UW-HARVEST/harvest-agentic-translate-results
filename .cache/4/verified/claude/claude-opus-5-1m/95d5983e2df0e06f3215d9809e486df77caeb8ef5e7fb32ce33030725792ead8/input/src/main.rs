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
//
// Rust translation of src/main.c -- byte-identical behavior.

use std::io::{Read, Write};

// ==================== Data Structures ====================

#[derive(Clone, Copy)]
struct Buffer {
    data: [u8; 256],
    length: usize,
    checksum: u32,
}

impl Buffer {
    // Equivalent of an (uninitialized) `buffer_t` on the stack / in malloc'd
    // storage.  Every code path in the original program only ever reads back
    // the first `length` bytes that it has just written, so zero filling is
    // observationally equivalent.
    fn new() -> Buffer {
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

/// Byte oriented reader over stdin with one byte of push-back, used to
/// reproduce C `scanf("%d", ...)` semantics exactly (whitespace, including
/// newlines, is skipped before a conversion).
struct Stdin {
    inner: std::io::Stdin,
    peeked: Option<u8>,
    eof: bool,
}

impl Stdin {
    fn new() -> Stdin {
        Stdin {
            inner: std::io::stdin(),
            peeked: None,
            eof: false,
        }
    }

    fn getc(&mut self) -> Option<u8> {
        if let Some(c) = self.peeked.take() {
            return Some(c);
        }
        if self.eof {
            return None;
        }
        let mut b = [0u8; 1];
        loop {
            match self.inner.read(&mut b) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(_) => return Some(b[0]),
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    fn ungetc(&mut self, c: u8) {
        self.peeked = Some(c);
    }

    /// C `isspace()` for the default locale.
    fn is_space(c: u8) -> bool {
        matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    }

    /// `scanf("%d", &out)`: returns the number of assigned items (1) or a
    /// failure (0 on matching failure, EOF otherwise -- the original code only
    /// distinguishes "== 1" from "!= 1").
    fn scan_int(&mut self) -> Option<i32> {
        // Skip leading white space.
        let mut c = loop {
            match self.getc() {
                Some(c) if Stdin::is_space(c) => continue,
                Some(c) => break c,
                None => return None, // EOF before any conversion
            }
        };

        let mut negative = false;
        if c == b'+' || c == b'-' {
            negative = c == b'-';
            match self.getc() {
                Some(n) => c = n,
                None => return None,
            }
        }

        if !c.is_ascii_digit() {
            // Matching failure: the offending character stays in the stream.
            self.ungetc(c);
            return None;
        }

        // glibc converts the collected digits with strtol() (64-bit long) and
        // then stores the value truncated to `int`; on overflow strtol
        // saturates at LONG_MAX / LONG_MIN.
        let mut value: i64 = 0;
        let mut overflow = false;
        loop {
            let digit = (c - b'0') as i64;
            if !overflow {
                match value.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                    Some(v) => value = v,
                    None => overflow = true,
                }
            }
            match self.getc() {
                Some(n) if n.is_ascii_digit() => c = n,
                Some(n) => {
                    self.ungetc(n);
                    break;
                }
                None => break,
            }
        }

        let result: i64 = if overflow {
            if negative {
                i64::MIN
            } else {
                i64::MAX
            }
        } else if negative {
            -value
        } else {
            value
        };

        Some(result as i32)
    }
}

/// Block buffered stdout (matching glibc's behavior for a redirected stream);
/// stderr stays unbuffered.
struct Stdout {
    buf: Vec<u8>,
    out: std::io::Stdout,
}

impl Stdout {
    fn new() -> Stdout {
        Stdout {
            buf: Vec::with_capacity(4096),
            out: std::io::stdout(),
        }
    }

    fn write_str(&mut self, s: &str) {
        self.buf.extend_from_slice(s.as_bytes());
        if self.buf.len() >= 4096 {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if !self.buf.is_empty() {
            let _ = self.out.write_all(&self.buf);
            self.buf.clear();
        }
        let _ = self.out.flush();
    }
}

fn eprint_str(s: &str) {
    let _ = std::io::stderr().write_all(s.as_bytes());
}

// ==================== Helper Functions ====================

// Calculate simple checksum
fn calculate_checksum(data: &[u8], length: usize) -> u32 {
    let mut sum: u32 = 0;
    for i in 0..length {
        sum = (sum << 3) ^ (data[i] as u32);
    }
    sum
}

// Validate buffer integrity
fn validate_buffer(buf: &Buffer) -> bool {
    // (the NULL check of the original cannot trigger here)
    if buf.length > 256 {
        eprint_str(&format!(
            "Error: Buffer length {} exceeds maximum 256\n",
            buf.length
        ));
        return false;
    }
    let expected = calculate_checksum(&buf.data, buf.length);
    if buf.checksum != expected {
        eprint_str(&format!(
            "Warning: Checksum mismatch. Expected {}, got {}\n",
            expected, buf.checksum
        ));
    }
    true
}

// Initialize buffer array
fn init_buffer_array(initial_capacity: i32) -> Option<BufferArray> {
    if initial_capacity <= 0 {
        eprint_str(&format!("Error: Invalid capacity {}\n", initial_capacity));
        return None;
    }

    Some(BufferArray {
        buffers: vec![Buffer::new(); initial_capacity as usize],
        count: 0,
        capacity: initial_capacity,
    })
}

// Free buffer array
fn free_buffer_array(arr: BufferArray) {
    drop(arr);
}

// ==================== Core Buffer Operations ====================

// Simple copy operation with memcpy
fn buffer_copy(src: &Buffer, dst: &mut Buffer) -> i32 {
    // (the NULL checks of the original cannot trigger here)
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
        eprint_str(&format!(
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
        eprint_str(&format!(
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
        eprint_str("Error: Interleaved length exceeds maximum\n");
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

    // Normalize positions to valid range
    let mut positions: i32 = positions % (buf.length as i32);
    if positions < 0 {
        // C: `positions += buf->length` -- the addition happens in size_t and
        // the result is converted back to int.
        positions = (positions as i64 as u64).wrapping_add(buf.length as u64) as u32 as i32;
    }
    let positions = positions as usize;

    let mut temp = [0u8; 256];
    temp[..buf.length].copy_from_slice(&buf.data[..buf.length]);

    // Copy rotated portions
    let len = buf.length;
    buf.data[..len - positions].copy_from_slice(&temp[positions..len]);
    buf.data[len - positions..len].copy_from_slice(&temp[..positions]);

    buf.checksum = calculate_checksum(&buf.data, buf.length);

    0
}

// Conditional copy based on pattern matching
#[allow(dead_code)]
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

// Copy with stride (every nth byte)
#[allow(dead_code)]
fn buffer_copy_strided(src: &Buffer, dst: &mut Buffer, stride: i32) -> i32 {
    if stride <= 0 {
        eprint_str(&format!("Error: Invalid stride {}\n", stride));
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

// Process buffer array with operation
#[allow(dead_code)]
fn process_buffer_array(arr: &mut BufferArray, op: i32, param: i32) -> i32 {
    if arr.count == 0 {
        eprint_str("Error: Invalid buffer array\n");
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
                eprint_str("Error: Need at least 2 buffers for merge\n");
                return -1;
            }
            let mut i = 0i32;
            while i < arr.count - 1 {
                let mut merged = Buffer::new();
                let (a, b) = (arr.buffers[i as usize], arr.buffers[i as usize + 1]);
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
            eprint_str(&format!("Error: Unknown operation {}\n", op));
            return -1;
        }
    }

    0
}

// ==================== Input/Output Functions ====================

// Read buffer from stdin
fn read_buffer(buf: &mut Buffer, inp: &mut Stdin) -> i32 {
    let length: i32 = match inp.scan_int() {
        Some(v) => v,
        None => {
            eprint_str("Error: Failed to read buffer length\n");
            return -1;
        }
    };

    if length < 0 || length > 256 {
        eprint_str(&format!("Error: Invalid buffer length {}\n", length));
        return -1;
    }

    buf.length = length as usize;
    for i in 0..buf.length {
        let byte: i32 = match inp.scan_int() {
            Some(v) => v,
            None => {
                eprint_str(&format!("Error: Failed to read byte {}\n", i));
                return -1;
            }
        };
        buf.data[i] = byte as u8;
    }

    buf.checksum = calculate_checksum(&buf.data, buf.length);
    0
}

// Write buffer to stdout
fn write_buffer(buf: &Buffer, out: &mut Stdout) {
    out.write_str(&format!("{}", buf.length));
    for i in 0..buf.length {
        out.write_str(&format!(" {}", buf.data[i]));
    }
    out.write_str("\n");
}

// ==================== Main Function ====================

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let inp = &mut Stdin::new();
    let out = &mut Stdout::new();

    // Read operation type
    let operation: i32 = match inp.scan_int() {
        Some(v) => v,
        None => {
            eprint_str("Error: Failed to read operation\n");
            out.flush();
            return 1;
        }
    };

    // Read buffer count
    let buffer_count: i32 = match inp.scan_int() {
        Some(v) => v,
        None => {
            eprint_str("Error: Failed to read buffer count\n");
            out.flush();
            return 1;
        }
    };

    if buffer_count <= 0 || buffer_count > 100 {
        eprint_str(&format!("Error: Invalid buffer count {}\n", buffer_count));
        out.flush();
        return 1;
    }

    // Allocate buffer array
    let mut buffers = match init_buffer_array(buffer_count) {
        Some(b) => b,
        None => {
            out.flush();
            return 1;
        }
    };

    // Read all buffers
    for i in 0..buffer_count as usize {
        if read_buffer(&mut buffers.buffers[i], inp) != 0 {
            free_buffer_array(buffers);
            out.flush();
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
                eprint_str("Error: Copy needs at least 2 buffers\n");
                result = -1;
            }
        }

        OP_REVERSE => {
            for i in 0..buffer_count as usize {
                result = buffer_reverse(&mut buffers.buffers[i]);
                if result != 0 {
                    break;
                }
                let b = buffers.buffers[i];
                write_buffer(&b, out);
            }
        }

        OP_MERGE => {
            if buffer_count >= 2 {
                let mut merged = Buffer::new();
                let (a, b) = (buffers.buffers[0], buffers.buffers[1]);
                result = buffer_merge(&a, &b, &mut merged);
                if result == 0 {
                    write_buffer(&merged, out);
                }
            } else {
                eprint_str("Error: Merge needs at least 2 buffers\n");
                result = -1;
            }
        }

        OP_SPLIT => {
            if buffer_count >= 1 {
                match inp.scan_int() {
                    None => {
                        eprint_str("Error: Failed to read split position\n");
                        result = -1;
                    }
                    Some(split_pos) => {
                        let mut part1 = Buffer::new();
                        let mut part2 = Buffer::new();
                        let src = buffers.buffers[0];
                        // int -> size_t conversion (sign extension)
                        result = buffer_split(&src, split_pos as usize, &mut part1, &mut part2);
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
                let (a, b) = (buffers.buffers[0], buffers.buffers[1]);
                result = buffer_interleave(&a, &b, &mut interleaved);
                if result == 0 {
                    write_buffer(&interleaved, out);
                }
            } else {
                eprint_str("Error: Interleave needs at least 2 buffers\n");
                result = -1;
            }
        }

        OP_ROTATE => match inp.scan_int() {
            None => {
                eprint_str("Error: Failed to read rotation amount\n");
                result = -1;
            }
            Some(positions) => {
                for i in 0..buffer_count as usize {
                    result = buffer_rotate(&mut buffers.buffers[i], positions);
                    if result != 0 {
                        break;
                    }
                    let b = buffers.buffers[i];
                    write_buffer(&b, out);
                }
            }
        },

        OP_CHECKSUM => {
            for i in 0..buffer_count as usize {
                out.write_str(&format!("{}\n", buffers.buffers[i].checksum));
            }
        }

        _ => {
            eprint_str(&format!("Error: Unknown operation {}\n", operation));
            result = -1;
        }
    }

    free_buffer_array(buffers);
    out.flush();
    if result != 0 {
        1
    } else {
        0
    }
}
