// Rust translation of c_src/src/main.c -- core implementation.
//
// Original copyright notice from the C source:
//
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

// The `cfg(test)` build of the library target drops the `main` export (see
// lib.rs), which makes `c_main`/`Scanner` look unused there.
#![allow(dead_code)]

use std::io::{self, Read, Write};
use std::os::raw::c_int;

/// Byte-at-a-time reader over an input stream that supports a one byte
/// "pushback", mirroring the way C's `scanf` peeks at (and ungets) the
/// character that terminates a conversion.
pub struct Scanner<R: Read> {
    inner: R,
    buf: Vec<u8>,
    pos: usize,
    len: usize,
    eof: bool,
}

impl<R: Read> Scanner<R> {
    pub fn new(inner: R) -> Self {
        Scanner {
            inner,
            // glibc sizes `stdin`'s buffer from st_blksize, i.e. 4096 for the
            // usual pipe/file/tty.  The size is not observable through the API,
            // it only decides how far ahead of the last consumed character the
            // file descriptor offset runs.
            buf: vec![0u8; 4096],
            pos: 0,
            len: 0,
            eof: false,
        }
    }

    /// Returns the next byte from the stream, or `None` at end-of-file.
    fn next_byte(&mut self) -> Option<u8> {
        loop {
            if self.pos < self.len {
                let b = self.buf[self.pos];
                self.pos += 1;
                return Some(b);
            }
            if self.eof {
                return None;
            }
            match self.inner.read(&mut self.buf) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(n) => {
                    self.pos = 0;
                    self.len = n;
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    /// Push the most recently read byte back onto the stream (`ungetc`).
    fn unget(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }

    /// Emulates `scanf("%d", &out)` as implemented by glibc.
    ///
    /// Returns 1 on a successful conversion, 0 on a matching failure
    /// (`conv_error`) and -1 (EOF) on an input failure (`input_error`), i.e.
    /// end-of-file hit before any character of the item was seen.
    ///
    /// glibc reads the digits into a scratch buffer, converts them with
    /// `strtol` (which saturates at `LONG_MAX` / `LONG_MIN` on overflow) and
    /// then stores `(int) num.l`, i.e. the low 32 bits of that `long`.
    pub fn scan_int(&mut self, out: &mut i32) -> i32 {
        // Skip leading whitespace, exactly the set isspace() matches in the C
        // locale: space, \t, \n, \v, \f, \r.  EOF here is an input failure.
        let mut c = loop {
            match self.next_byte() {
                None => return -1, // EOF before any input item -> input_error()
                Some(b) => {
                    if matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
                        continue;
                    }
                    break b;
                }
            }
        };

        let mut negative = false;
        if c == b'+' || c == b'-' {
            negative = c == b'-';
            match self.next_byte() {
                // Only the sign was read: glibc's `wpsize == 1 && wp[0] is a
                // sign` test fires and it reports a *matching* failure.
                None => return 0,
                Some(b) => c = b,
            }
        }

        if !c.is_ascii_digit() {
            // Matching failure: put the offending character back.
            self.unget();
            return 0;
        }

        let mut acc: i64 = 0;
        let mut overflow = false;
        loop {
            let digit = (c - b'0') as i64;
            if !overflow {
                if negative {
                    match acc.checked_mul(10).and_then(|v| v.checked_sub(digit)) {
                        Some(v) => acc = v,
                        None => overflow = true,
                    }
                } else {
                    match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                        Some(v) => acc = v,
                        None => overflow = true,
                    }
                }
            }
            match self.next_byte() {
                None => break,
                Some(b) => {
                    if b.is_ascii_digit() {
                        c = b;
                    } else {
                        // Terminating character is not consumed by scanf.
                        self.unget();
                        break;
                    }
                }
            }
        }

        if overflow {
            // strtol saturates at LONG_MAX / LONG_MIN, then the value is
            // stored into an int (low 32 bits kept).
            acc = if negative { i64::MIN } else { i64::MAX };
        }

        *out = acc as i32;
        1
    }
}

/// ```c
/// void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len) {
///     for (int i = 0; i < len; i++) {
///         out[i] = mul1[i] * mul2[i] + add[i];
///     }
/// }
/// ```
///
/// Implemented over raw pointers so that arbitrary aliasing between the four
/// buffers behaves exactly like the C: for every `i` the three operands are
/// loaded first and only then is `out[i]` stored.
///
/// Signed overflow is undefined behaviour in C; gcc/clang produce two's
/// complement wraparound here, which `wrapping_*` reproduces.
///
/// # Safety
/// The caller must guarantee that all four pointers are valid for `len`
/// elements when `len > 0` (exactly the contract the C code has).
pub unsafe fn fma_array_raw(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    let mut i: c_int = 0;
    while i < len {
        let idx = i as isize;
        let a = *mul1.offset(idx);
        let b = *mul2.offset(idx);
        let c = *add.offset(idx);
        *out.offset(idx) = a.wrapping_mul(b).wrapping_add(c);
        i = i.wrapping_add(1);
    }
}

/// ```c
/// void driver(int *out, int len) {
///     fma_array(out, out, out, out, len);
///     for (int i = 0; i < len; i++) {
///         printf("%d\n", out[i]);
///     }
/// }
/// ```
///
/// # Safety
/// `out` must be valid for `len` elements when `len > 0`.
pub unsafe fn driver_raw(out: *mut c_int, len: c_int) {
    fma_array_raw(out, out, out, out, len);

    let stdout = io::stdout();
    let mut w = stdout.lock();
    let mut i: c_int = 0;
    while i < len {
        let _ = writeln!(w, "{}", *out.offset(i as isize));
        i = i.wrapping_add(1);
    }
    let _ = w.flush();
}

/// ```c
/// int main() {
///     int data[100];
///     int i;
///     for (i = 0; i < 100; i++) {
///         if (scanf("%d", &data[i]) != 1) {
///             break;
///         }
///     }
///
///     driver(data, i);
///     return 0;
/// }
/// ```
///
/// `int data[100]` is uninitialised in C, but only the first `i` entries are
/// ever read back, so the zero-filled tail is never observed.
pub fn c_main() -> c_int {
    let mut data = [0i32; 100];

    let mut guard = stdin_scanner();
    let scanner = guard.get_or_insert_with(|| Scanner::new(io::stdin()));

    let mut i: usize = 0;
    while i < 100 {
        let mut value: i32 = 0;
        if scanner.scan_int(&mut value) != 1 {
            break;
        }
        data[i] = value;
        i += 1;
    }
    drop(guard);

    unsafe { driver_raw(data.as_mut_ptr(), i as c_int) };
    0
}

/// C's `stdin` is a process-wide `FILE` whose read-ahead buffer survives across
/// calls, so a second `main()` in the same process picks the input stream up
/// exactly where the first one stopped.  The scanner therefore has to be a
/// global too, rather than a fresh one per call.
fn stdin_scanner() -> std::sync::MutexGuard<'static, Option<Scanner<io::Stdin>>> {
    static STDIN: std::sync::Mutex<Option<Scanner<io::Stdin>>> = std::sync::Mutex::new(None);
    STDIN.lock().unwrap_or_else(|e| e.into_inner())
}
