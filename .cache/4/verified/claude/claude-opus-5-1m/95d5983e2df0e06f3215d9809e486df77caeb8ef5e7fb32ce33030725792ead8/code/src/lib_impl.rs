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
// Rust translation of c_src/src/main.c -- byte-identical behavior.
//
// This module is shared verbatim by both build targets:
//   * `src/lib.rs`   -- the `cdylib`/`rlib`, which additionally exports the
//                       C `main` entry point.
//   * `src/main.rs`  -- the `driver` executable (via `#[path]`), which uses a
//                       normal Rust `fn main` shim.
// Keeping the translation in one file guarantees the executable and the shared
// library cannot drift apart.

#![allow(dead_code)]


use core::ffi::{c_char, c_int, c_void};
use core::ptr::{addr_of, addr_of_mut, null_mut};

// ==================== Data Structures ====================

/// `buffer_t` -- layout must match the C struct exactly:
/// `struct { uint8_t data[256]; size_t length; uint32_t checksum; }`
/// (256 + 8 + 4 = 268 bytes, padded to 272 for the 8 byte alignment).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BufferT {
    pub data: [u8; 256],
    pub length: usize,
    pub checksum: u32,
}

impl BufferT {
    /// Equivalent of an (uninitialized) `buffer_t` on the stack / in malloc'd
    /// storage.  Every code path in the original program only ever reads back
    /// the first `length` bytes that it has just written, so zero filling is
    /// observationally equivalent.
    pub fn new() -> BufferT {
        BufferT {
            data: [0u8; 256],
            length: 0,
            checksum: 0,
        }
    }
}

/// `buffer_array_t` -- `struct { buffer_t *buffers; int count; int capacity; }`
#[repr(C)]
pub struct BufferArrayT {
    pub buffers: *mut BufferT,
    pub count: c_int,
    pub capacity: c_int,
}

// operation_t
pub const OP_COPY: c_int = 0;
pub const OP_REVERSE: c_int = 1;
pub const OP_MERGE: c_int = 2;
pub const OP_SPLIT: c_int = 3;
pub const OP_INTERLEAVE: c_int = 4;
pub const OP_ROTATE: c_int = 5;
pub const OP_CHECKSUM: c_int = 6;

/// Size of the fixed `data` member.  Several C helpers copy `buf->length`
/// bytes through a `uint8_t temp[256]` scratch array *without* first checking
/// `length <= 256`; for `length > 256` the C code therefore has undefined
/// behaviour (stack/struct overflow).  The Rust translation clamps the memory
/// traffic to `DATA_LEN` in exactly those spots so that it stays memory safe.
/// `read_buffer()` -- the only way the shipped program can ever populate a
/// buffer -- rejects `length > 256`, so the clamp is unreachable in practice.
pub const DATA_LEN: usize = 256;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
}

// ==================== stdio emulation ====================

/// glibc's default buffer size for a `FILE` attached to a regular file / pipe.
const IO_BUF: usize = 4096;

/// Byte oriented reader over stdin with one byte of push-back, used to
/// reproduce C `scanf("%d", ...)` semantics exactly (whitespace, including
/// newlines, is skipped before a conversion).
///
/// The read-ahead buffer is *owned by this struct*, exactly like C's
/// `FILE *stdin`.  It deliberately does not go through `std::io::stdin()`,
/// whose buffer is a process-wide singleton: the exported `read_buffer()`
/// symbol needs a stream whose state can be reset independently (the C side
/// does that with `freopen`).
pub struct Stdin {
    buf: [u8; IO_BUF],
    pos: usize,
    end: usize,
    peeked: Option<u8>,
    eof: bool,
}

impl Stdin {
    pub fn new() -> Stdin {
        Stdin {
            buf: [0u8; IO_BUF],
            pos: 0,
            end: 0,
            peeked: None,
            eof: false,
        }
    }

    fn getc(&mut self) -> Option<u8> {
        if let Some(c) = self.peeked.take() {
            return Some(c);
        }
        if self.pos < self.end {
            let c = self.buf[self.pos];
            self.pos += 1;
            return Some(c);
        }
        if self.eof {
            return None;
        }
        loop {
            let n = unsafe { read(0, self.buf.as_mut_ptr() as *mut c_void, IO_BUF) };
            if n > 0 {
                self.pos = 1;
                self.end = n as usize;
                return Some(self.buf[0]);
            }
            if n == 0 {
                self.eof = true;
                return None;
            }
            // n < 0: retry on EINTR like the C library does, otherwise report EOF.
            if std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
                self.eof = true;
                return None;
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
    pub fn scan_int(&mut self) -> Option<i32> {
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

/// Write `bytes` to `fd`, restarting short writes and `EINTR` — the loop the C
/// library performs internally when it drains a `FILE` buffer.
fn write_fd(fd: c_int, bytes: &[u8]) {
    let mut off = 0usize;
    while off < bytes.len() {
        let n = unsafe {
            write(
                fd,
                bytes[off..].as_ptr() as *const c_void,
                bytes.len() - off,
            )
        };
        if n > 0 {
            off += n as usize;
            continue;
        }
        if n == 0 {
            break;
        }
        if std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
            break;
        }
    }
}

/// Block buffered stdout (matching glibc's behavior for a redirected stream);
/// stderr stays unbuffered.
///
/// As with `Stdin`, the buffer belongs to the struct rather than to
/// `std::io::stdout()`, whose buffer is a process-wide singleton with
/// line-buffering semantics of its own.
pub struct Stdout {
    buf: Vec<u8>,
    /// When false every write goes straight to the file descriptor.  This is
    /// what the *exported* `write_buffer()` symbol uses: a foreign caller has
    /// no way to reach our private FILE buffer, so the bytes have to be
    /// visible as soon as the call returns.
    buffered: bool,
}

impl Stdout {
    pub fn new() -> Stdout {
        Stdout {
            buf: Vec::with_capacity(IO_BUF),
            buffered: true,
        }
    }

    pub fn unbuffered() -> Stdout {
        Stdout {
            buf: Vec::new(),
            buffered: false,
        }
    }

    pub fn write_str(&mut self, s: &str) {
        self.buf.extend_from_slice(s.as_bytes());
        if !self.buffered || self.buf.len() >= IO_BUF {
            self.flush();
        }
    }

    pub fn flush(&mut self) {
        if !self.buf.is_empty() {
            write_fd(1, &self.buf);
            self.buf.clear();
        }
    }
}

/// C's `stderr` is unbuffered, so every `fprintf(stderr, ...)` reaches fd 2
/// immediately.
fn eprint_str(s: &str) {
    write_fd(2, s.as_bytes());
}

// ==================== Helper Functions ====================

/// Body of `calculate_checksum()`; kept pointer based so that a `length` of 0
/// combined with a NULL `data` behaves like the C original (no dereference).
#[inline]
unsafe fn checksum_raw(data: *const u8, length: usize) -> u32 {
    let mut sum: u32 = 0;
    let mut i: usize = 0;
    while i < length {
        sum = (sum << 3) ^ (*data.add(i) as u32);
        i += 1;
    }
    sum
}

// Calculate simple checksum
#[no_mangle]
pub unsafe extern "C" fn calculate_checksum(data: *const u8, length: usize) -> u32 {
    checksum_raw(data, length)
}

// Validate buffer integrity
#[no_mangle]
pub unsafe extern "C" fn validate_buffer(buf: *const BufferT) -> bool {
    if buf.is_null() {
        eprint_str("Error: NULL buffer\n");
        return false;
    }
    let length = (*buf).length;
    if length > 256 {
        eprint_str(&format!(
            "Error: Buffer length {} exceeds maximum 256\n",
            length
        ));
        return false;
    }
    let expected = checksum_raw(addr_of!((*buf).data) as *const u8, length);
    let got = (*buf).checksum;
    if got != expected {
        eprint_str(&format!(
            "Warning: Checksum mismatch. Expected {}, got {}\n",
            expected, got
        ));
    }
    true
}

// Initialize buffer array
#[no_mangle]
pub unsafe extern "C" fn init_buffer_array(initial_capacity: c_int) -> *mut BufferArrayT {
    if initial_capacity <= 0 {
        eprint_str(&format!("Error: Invalid capacity {}\n", initial_capacity));
        return null_mut();
    }

    let arr = malloc(core::mem::size_of::<BufferArrayT>()) as *mut BufferArrayT;
    if arr.is_null() {
        eprint_str("Error: Failed to allocate buffer array\n");
        return null_mut();
    }

    let buffers = malloc(core::mem::size_of::<BufferT>() * initial_capacity as usize) as *mut BufferT;
    if buffers.is_null() {
        eprint_str("Error: Failed to allocate buffer storage\n");
        free(arr as *mut c_void);
        return null_mut();
    }
    // Like the C original the storage is deliberately left uninitialized: every
    // consumer (`read_buffer`) writes `length`, `data[0..length]` and `checksum`
    // before anything reads them back, and only ever reads those written bytes.
    // Pre-filling would also make a huge `initial_capacity` (where C's malloc
    // simply fails) pathologically slow instead of instant.

    (*arr).buffers = buffers;
    (*arr).count = 0;
    (*arr).capacity = initial_capacity;
    arr
}

// Free buffer array
#[no_mangle]
pub unsafe extern "C" fn free_buffer_array(arr: *mut BufferArrayT) {
    if !arr.is_null() {
        free((*arr).buffers as *mut c_void);
        free(arr as *mut c_void);
    }
}

// ==================== Core Buffer Operations ====================

// Simple copy operation with memcpy
#[no_mangle]
pub unsafe extern "C" fn buffer_copy(src: *const BufferT, dst: *mut BufferT) -> c_int {
    if src.is_null() || dst.is_null() {
        eprint_str("Error: NULL pointer in buffer_copy\n");
        return -1;
    }

    if !validate_buffer(src) {
        return -1;
    }

    // Use memcpy for data transfer.  `validate_buffer` has already rejected
    // `length > 256`, so this cannot overrun `data`.
    let n = (*src).length;
    core::ptr::copy(
        addr_of!((*src).data) as *const u8,
        addr_of_mut!((*dst).data) as *mut u8,
        n,
    );
    (*dst).length = n;
    (*dst).checksum = checksum_raw(addr_of!((*dst).data) as *const u8, n);

    0
}

// Reverse buffer contents
#[no_mangle]
pub unsafe extern "C" fn buffer_reverse(buf: *mut BufferT) -> c_int {
    if buf.is_null() {
        eprint_str("Error: NULL buffer in reverse\n");
        return -1;
    }

    let length = (*buf).length;
    if length == 0 {
        return 0; // Nothing to reverse
    }

    // C: `uint8_t temp[256]; memcpy(temp, buf->data, buf->length);`
    let n = if length > DATA_LEN { DATA_LEN } else { length };
    let data = addr_of_mut!((*buf).data) as *mut u8;
    let mut temp = [0u8; DATA_LEN];
    core::ptr::copy(data, temp.as_mut_ptr(), n);

    for i in 0..n {
        *data.add(i) = temp[n - 1 - i];
    }

    (*buf).checksum = checksum_raw(data, n);
    0
}

// Merge two buffers into destination
#[no_mangle]
pub unsafe extern "C" fn buffer_merge(
    src1: *const BufferT,
    src2: *const BufferT,
    dst: *mut BufferT,
) -> c_int {
    if src1.is_null() || src2.is_null() || dst.is_null() {
        eprint_str("Error: NULL pointer in buffer_merge\n");
        return -1;
    }

    let l1 = (*src1).length;
    let l2 = (*src2).length;
    let total = l1.wrapping_add(l2);
    if total > 256 {
        eprint_str(&format!("Error: Merged length {} exceeds maximum\n", total));
        return -1;
    }

    let d = addr_of_mut!((*dst).data) as *mut u8;
    // Copy first buffer
    core::ptr::copy(addr_of!((*src1).data) as *const u8, d, l1);
    // Copy second buffer after first
    core::ptr::copy(addr_of!((*src2).data) as *const u8, d.add(l1), l2);

    (*dst).length = total;
    (*dst).checksum = checksum_raw(d, total);

    0
}

// Split buffer at position into two buffers
#[no_mangle]
pub unsafe extern "C" fn buffer_split(
    src: *const BufferT,
    split_pos: usize,
    dst1: *mut BufferT,
    dst2: *mut BufferT,
) -> c_int {
    if src.is_null() || dst1.is_null() || dst2.is_null() {
        eprint_str("Error: NULL pointer in buffer_split\n");
        return -1;
    }

    let length = (*src).length;
    if split_pos > length {
        eprint_str(&format!(
            "Error: Split position {} exceeds length {}\n",
            split_pos, length
        ));
        return -1;
    }

    let s = addr_of!((*src).data) as *const u8;
    // `length > 256` is undefined behaviour in the C original (it memcpy's out
    // of `data`); clamp so the Rust side stays memory safe.
    let pos_eff = if split_pos > DATA_LEN { DATA_LEN } else { split_pos };

    // Copy first part
    let d1 = addr_of_mut!((*dst1).data) as *mut u8;
    if pos_eff > 0 {
        core::ptr::copy(s, d1, pos_eff);
    }
    (*dst1).length = split_pos;
    (*dst1).checksum = checksum_raw(d1, pos_eff);

    // Copy second part.
    //
    // C: `size_t remaining = src->length - split_pos;` -- `src->length` is read
    // *here*, i.e. AFTER `dst1->length` has been assigned.  When the caller
    // aliases `dst1` onto `src` that assignment has already overwritten
    // `src->length` with `split_pos`, so `remaining` becomes 0.  The read must
    // therefore not be hoisted to the top of the function.
    let remaining = (*src).length.wrapping_sub(split_pos);
    let rem_eff = if remaining > DATA_LEN - pos_eff {
        DATA_LEN - pos_eff
    } else {
        remaining
    };
    let d2 = addr_of_mut!((*dst2).data) as *mut u8;
    if rem_eff > 0 {
        core::ptr::copy(s.add(pos_eff), d2, rem_eff);
    }
    (*dst2).length = remaining;
    (*dst2).checksum = checksum_raw(d2, rem_eff);

    0
}

// Interleave two buffers (alternating bytes)
#[no_mangle]
pub unsafe extern "C" fn buffer_interleave(
    src1: *const BufferT,
    src2: *const BufferT,
    dst: *mut BufferT,
) -> c_int {
    if src1.is_null() || src2.is_null() || dst.is_null() {
        eprint_str("Error: NULL pointer in buffer_interleave\n");
        return -1;
    }

    let l1 = (*src1).length;
    let l2 = (*src2).length;
    let max_len = if l1 > l2 { l1 } else { l2 };
    if l1.wrapping_add(l2) > 256 {
        eprint_str("Error: Interleaved length exceeds maximum\n");
        return -1;
    }

    let s1 = addr_of!((*src1).data) as *const u8;
    let s2 = addr_of!((*src2).data) as *const u8;
    let d = addr_of_mut!((*dst).data) as *mut u8;

    let mut dst_pos: usize = 0;
    for i in 0..max_len {
        if i < l1 {
            *d.add(dst_pos) = *s1.add(i);
            dst_pos += 1;
        }
        if i < l2 {
            *d.add(dst_pos) = *s2.add(i);
            dst_pos += 1;
        }
    }

    (*dst).length = dst_pos;
    (*dst).checksum = checksum_raw(d, dst_pos);

    0
}

// Rotate buffer left by n positions
#[no_mangle]
pub unsafe extern "C" fn buffer_rotate(buf: *mut BufferT, positions: c_int) -> c_int {
    if buf.is_null() {
        eprint_str("Error: NULL buffer in rotate\n");
        return -1;
    }

    let length = (*buf).length;
    if length == 0 || positions == 0 {
        return 0; // Nothing to rotate
    }

    // Normalize positions to valid range
    let mut positions: c_int = positions % (length as c_int);
    if positions < 0 {
        // C: `positions += buf->length` -- the addition happens in size_t and
        // the result is converted back to int.
        positions = (positions as isize as usize).wrapping_add(length) as u32 as c_int;
    }
    let positions = positions as usize;

    // C: `uint8_t temp[256]; memcpy(temp, buf->data, buf->length);`
    let n = if length > DATA_LEN { DATA_LEN } else { length };
    let positions = if positions > n { n } else { positions };
    let data = addr_of_mut!((*buf).data) as *mut u8;
    let mut temp = [0u8; DATA_LEN];
    core::ptr::copy(data, temp.as_mut_ptr(), n);

    // Copy rotated portions
    core::ptr::copy(temp.as_ptr().add(positions), data, n - positions);
    core::ptr::copy(temp.as_ptr(), data.add(n - positions), positions);

    (*buf).checksum = checksum_raw(data, n);

    0
}

// Conditional copy based on pattern matching
//
// NOTE: `copy_matching` is a C `_Bool`.  It is taken as a `u8` here so that a
// caller passing a value other than 0/1 (which a C caller *can* do across the
// ABI) does not create a Rust `bool` with an invalid bit pattern; the
// comparison below reproduces the C promotion to `int` exactly.
#[no_mangle]
pub unsafe extern "C" fn buffer_conditional_copy(
    src: *const BufferT,
    dst: *mut BufferT,
    pattern: u8,
    copy_matching: u8,
) -> c_int {
    if src.is_null() || dst.is_null() {
        eprint_str("Error: NULL pointer in conditional_copy\n");
        return -1;
    }

    let length = (*src).length;
    let n = if length > DATA_LEN { DATA_LEN } else { length };
    let s = addr_of!((*src).data) as *const u8;
    let d = addr_of_mut!((*dst).data) as *mut u8;

    let mut dst_pos: usize = 0;
    for i in 0..n {
        let matches: u8 = if *s.add(i) == pattern { 1 } else { 0 };
        if matches == copy_matching {
            *d.add(dst_pos) = *s.add(i);
            dst_pos += 1;
        }
    }

    (*dst).length = dst_pos;
    (*dst).checksum = checksum_raw(d, dst_pos);

    0
}

// Copy with stride (every nth byte)
#[no_mangle]
pub unsafe extern "C" fn buffer_copy_strided(
    src: *const BufferT,
    dst: *mut BufferT,
    stride: c_int,
) -> c_int {
    if src.is_null() || dst.is_null() {
        eprint_str("Error: NULL pointer in copy_strided\n");
        return -1;
    }

    if stride <= 0 {
        eprint_str(&format!("Error: Invalid stride {}\n", stride));
        return -1;
    }

    let length = (*src).length;
    let n = if length > DATA_LEN { DATA_LEN } else { length };
    let s = addr_of!((*src).data) as *const u8;
    let d = addr_of_mut!((*dst).data) as *mut u8;

    let mut dst_pos: usize = 0;
    let mut i: usize = 0;
    while i < n {
        *d.add(dst_pos) = *s.add(i);
        dst_pos += 1;
        // C: `i += stride` with `i` a size_t and `stride` a positive int.
        i = i.wrapping_add(stride as usize);
    }

    (*dst).length = dst_pos;
    (*dst).checksum = checksum_raw(d, dst_pos);

    0
}

// ==================== Complex Processing Functions ====================

// Process buffer array with operation
#[no_mangle]
pub unsafe extern "C" fn process_buffer_array(
    arr: *mut BufferArrayT,
    op: c_int,
    param: c_int,
) -> c_int {
    if arr.is_null() || (*arr).count == 0 {
        eprint_str("Error: Invalid buffer array\n");
        return -1;
    }

    let count = (*arr).count;
    let buffers = (*arr).buffers;

    match op {
        OP_COPY => {
            // Copy first buffer to all others
            let mut i: c_int = 1;
            while i < count {
                if buffer_copy(buffers, buffers.offset(i as isize)) != 0 {
                    return -1;
                }
                i += 1;
            }
        }

        OP_REVERSE => {
            // Reverse all buffers
            let mut i: c_int = 0;
            while i < count {
                if buffer_reverse(buffers.offset(i as isize)) != 0 {
                    return -1;
                }
                i += 1;
            }
        }

        OP_MERGE => {
            // Merge consecutive pairs
            if count < 2 {
                eprint_str("Error: Need at least 2 buffers for merge\n");
                return -1;
            }
            let mut i: c_int = 0;
            while i < count - 1 {
                // C: `buffer_t merged;` -- an uninitialized stack object whose
                // tail beyond `merged.length` is copied verbatim into
                // `arr->buffers[i]`.  Those tail bytes are indeterminate in C
                // and are never read back by any consumer.
                let mut merged = BufferT::new();
                if buffer_merge(
                    buffers.offset(i as isize),
                    buffers.offset(i as isize + 1),
                    &mut merged as *mut BufferT,
                ) != 0
                {
                    return -1;
                }
                core::ptr::copy(
                    &merged as *const BufferT,
                    buffers.offset(i as isize),
                    1,
                );
                i += 2;
            }
        }

        OP_ROTATE => {
            // Rotate all buffers by param positions
            let mut i: c_int = 0;
            while i < count {
                if buffer_rotate(buffers.offset(i as isize), param) != 0 {
                    return -1;
                }
                i += 1;
            }
        }

        OP_CHECKSUM => {
            // Verify all checksums
            let mut i: c_int = 0;
            while i < count {
                if !validate_buffer(buffers.offset(i as isize)) {
                    return -1;
                }
                i += 1;
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

/// Shared body of `read_buffer()`.
unsafe fn read_buffer_from(buf: *mut BufferT, inp: &mut Stdin) -> c_int {
    if buf.is_null() {
        eprint_str("Error: NULL buffer in read_buffer\n");
        return -1;
    }

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

    (*buf).length = length as usize;
    let data = addr_of_mut!((*buf).data) as *mut u8;
    for i in 0..(*buf).length {
        let byte: i32 = match inp.scan_int() {
            Some(v) => v,
            None => {
                eprint_str(&format!("Error: Failed to read byte {}\n", i));
                return -1;
            }
        };
        *data.add(i) = byte as u8;
    }

    (*buf).checksum = checksum_raw(data, (*buf).length);
    0
}

/// Process wide stdin state used by the *exported* `read_buffer()` symbol,
/// mirroring C's single global `FILE *stdin`.
static mut EXPORTED_STDIN: Option<Stdin> = None;

/// Reset the exported `read_buffer()` push-back state.  Not part of the C API
/// (C reaches the same effect with `freopen(..., stdin)`); used by the
/// differential test suite so that consecutive scenarios start from a clean
/// stream.
#[no_mangle]
pub unsafe extern "C" fn driver_reset_exported_stdin() {
    EXPORTED_STDIN = Some(Stdin::new());
}

// Read buffer from stdin
#[no_mangle]
pub unsafe extern "C" fn read_buffer(buf: *mut BufferT) -> c_int {
    let inp = {
        let slot = addr_of_mut!(EXPORTED_STDIN);
        if (*slot).is_none() {
            *slot = Some(Stdin::new());
        }
        (*slot).as_mut().unwrap()
    };
    read_buffer_from(buf, inp)
}

/// Shared body of `write_buffer()`.
unsafe fn write_buffer_to(buf: *const BufferT, out: &mut Stdout) {
    if buf.is_null() {
        eprint_str("Error: NULL buffer in write_buffer\n");
        return;
    }

    let length = (*buf).length;
    out.write_str(&format!("{}", length));
    let data = addr_of!((*buf).data) as *const u8;
    let n = if length > DATA_LEN { DATA_LEN } else { length };
    for i in 0..n {
        out.write_str(&format!(" {}", *data.add(i)));
    }
    out.write_str("\n");
}

// Write buffer to stdout
#[no_mangle]
pub unsafe extern "C" fn write_buffer(buf: *const BufferT) {
    // A foreign caller cannot see our private FILE buffer, so the exported
    // entry point writes through immediately.
    let mut out = Stdout::unbuffered();
    write_buffer_to(buf, &mut out);
    out.flush();
}

// ==================== Main Function ====================

/// Faithful translation of C `int main(int argc, char *argv[])`.
pub unsafe fn c_main(_argc: c_int, _argv: *mut *mut c_char) -> c_int {
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
    let buffers = init_buffer_array(buffer_count);
    if buffers.is_null() {
        out.flush();
        return 1;
    }
    let storage = (*buffers).buffers;

    // Read all buffers
    for i in 0..buffer_count {
        if read_buffer_from(storage.offset(i as isize), inp) != 0 {
            free_buffer_array(buffers);
            out.flush();
            return 1;
        }
        (*buffers).count += 1;
    }

    // Execute operation based on type
    let mut result: c_int = 0;
    match operation {
        OP_COPY => {
            if buffer_count >= 2 {
                let mut temp = BufferT::new();
                result = buffer_copy(storage, &mut temp as *mut BufferT);
                if result == 0 {
                    write_buffer_to(&temp as *const BufferT, out);
                }
            } else {
                eprint_str("Error: Copy needs at least 2 buffers\n");
                result = -1;
            }
        }

        OP_REVERSE => {
            for i in 0..buffer_count {
                result = buffer_reverse(storage.offset(i as isize));
                if result != 0 {
                    break;
                }
                write_buffer_to(storage.offset(i as isize), out);
            }
        }

        OP_MERGE => {
            if buffer_count >= 2 {
                let mut merged = BufferT::new();
                result = buffer_merge(storage, storage.offset(1), &mut merged as *mut BufferT);
                if result == 0 {
                    write_buffer_to(&merged as *const BufferT, out);
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
                        let mut part1 = BufferT::new();
                        let mut part2 = BufferT::new();
                        // int -> size_t conversion (sign extension)
                        result = buffer_split(
                            storage,
                            split_pos as isize as usize,
                            &mut part1 as *mut BufferT,
                            &mut part2 as *mut BufferT,
                        );
                        if result == 0 {
                            write_buffer_to(&part1 as *const BufferT, out);
                            write_buffer_to(&part2 as *const BufferT, out);
                        }
                    }
                }
            }
        }

        OP_INTERLEAVE => {
            if buffer_count >= 2 {
                let mut interleaved = BufferT::new();
                result = buffer_interleave(
                    storage,
                    storage.offset(1),
                    &mut interleaved as *mut BufferT,
                );
                if result == 0 {
                    write_buffer_to(&interleaved as *const BufferT, out);
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
                for i in 0..buffer_count {
                    result = buffer_rotate(storage.offset(i as isize), positions);
                    if result != 0 {
                        break;
                    }
                    write_buffer_to(storage.offset(i as isize), out);
                }
            }
        },

        OP_CHECKSUM => {
            for i in 0..buffer_count {
                out.write_str(&format!("{}\n", (*storage.offset(i as isize)).checksum));
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
