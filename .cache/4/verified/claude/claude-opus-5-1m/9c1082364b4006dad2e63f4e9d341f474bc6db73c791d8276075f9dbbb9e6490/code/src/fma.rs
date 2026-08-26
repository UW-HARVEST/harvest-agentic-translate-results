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

#![allow(dead_code)]

use std::io::{self, Read, Write};

/// `int` on every platform this library targets.
pub type CInt = i32;

// ---------------------------------------------------------------------------
// void fma_array(int *restrict out, const int *mul1, const int *mul2,
//                const int *add, int len)
// ---------------------------------------------------------------------------
//
// The C loop is `for (int i = 0; i < len; i++)`, so a `len` of zero or less
// performs no iterations and therefore never dereferences any of the four
// pointers.  For `len > 0` the C code dereferences them unconditionally, so
// this translation does the same with raw pointers: a null/invalid pointer
// faults exactly the way the C does instead of tripping a Rust-side check.
//
// `out[i] = mul1[i] * mul2[i] + add[i]` is `int` arithmetic; signed overflow
// is UB in C but wraps in practice on every mainstream compiler, so wrapping
// operations are used here.
/// # Safety
///
/// For `len > 0`, `out` must be valid for `len` writes and `mul1`, `mul2` and
/// `add` valid for `len` reads, exactly as the C function requires.
pub unsafe fn fma_array_raw(
    out: *mut CInt,
    mul1: *const CInt,
    mul2: *const CInt,
    add: *const CInt,
    len: CInt,
) {
    let mut i: CInt = 0;
    while i < len {
        let idx = i as isize;
        // `ptr::read`/`ptr::write` rather than `*p` so that an invalid pointer
        // faults with SIGSEGV exactly as the C does. A plain `*p` deref is
        // instrumented with a null check when debug assertions are on, which
        // would turn the C's SIGSEGV into a Rust SIGABRT and diverge.
        let a = std::ptr::read(mul1.offset(idx));
        let b = std::ptr::read(mul2.offset(idx));
        let c = std::ptr::read(add.offset(idx));
        std::ptr::write(out.offset(idx), a.wrapping_mul(b).wrapping_add(c));
        i += 1;
    }
}

/// Safe-slice convenience form of [`fma_array_raw`], used by the driver.
pub fn fma_array_slices(out: &mut [CInt], mul1: &[CInt], mul2: &[CInt], add: &[CInt], len: CInt) {
    if len <= 0 {
        return;
    }
    let n = len as usize;
    assert!(out.len() >= n && mul1.len() >= n && mul2.len() >= n && add.len() >= n);
    unsafe {
        fma_array_raw(
            out.as_mut_ptr(),
            mul1.as_ptr(),
            mul2.as_ptr(),
            add.as_ptr(),
            len,
        );
    }
}

// ---------------------------------------------------------------------------
// int call_fma(const int *data, int len)
// ---------------------------------------------------------------------------
//
//     if (len == 0) return 0;
//     int out[len]; int ones[len]; int zeros[len];
//     out[0] = 0;
//     for (i = 0; i < len; i++) { ones[i] = 1; zeros[i] = 0; }
//     fma_array(out, ones, data, zeros, len);
//     return out[len-1];
//
// For `len > 0` the observable result is `data[len-1] * 1 + 0`, but the whole
// pipeline is reproduced (scratch buffers, the fill loop and the real
// `fma_array` call) so that any value-dependent behaviour matches.
//
// `len < 0` declares variable-length arrays with a negative size and then
// reads `out[len-1]`, i.e. out of bounds: undefined behaviour.  Measured
// against gcc 11 -O2 the C returns unpredictable stack garbage, so no defined
// behaviour exists to match; this translation returns 0 without faulting.
/// # Safety
///
/// For `len > 0`, `data` must be valid for `len` reads, as the C requires.
pub unsafe fn call_fma_raw(data: *const CInt, len: CInt) -> CInt {
    if len == 0 {
        return 0;
    }
    if len < 0 {
        // Undefined behaviour in C (negative VLA size + out-of-bounds read).
        return 0;
    }
    let n = len as usize;

    // int out[len]; int ones[len]; int zeros[len];  (VLAs, uninitialized)
    let mut out: Vec<CInt> = vec![0; n];
    let mut ones: Vec<CInt> = vec![0; n];
    let mut zeros: Vec<CInt> = vec![0; n];

    out[0] = 0;
    let mut i: CInt = 0;
    while i < len {
        ones[i as usize] = 1;
        zeros[i as usize] = 0;
        i += 1;
    }

    fma_array_raw(out.as_mut_ptr(), ones.as_ptr(), data, zeros.as_ptr(), len);
    out[n - 1]
}

/// Safe-slice convenience form of [`call_fma_raw`], used by the driver.
pub fn call_fma_slice(data: &[CInt], len: CInt) -> CInt {
    if len == 0 {
        return 0;
    }
    if len < 0 {
        return 0;
    }
    assert!(data.len() >= len as usize);
    unsafe { call_fma_raw(data.as_ptr(), len) }
}

// ---------------------------------------------------------------------------
// int main(void)
// ---------------------------------------------------------------------------

/// Outcome of one `scanf("%d", ...)` conversion.
pub enum ScanResult {
    /// Conversion succeeded; scanf returned 1.
    Value(CInt),
    /// Matching failure: input present but not a valid integer; scanf returned 0.
    MatchFailure,
    /// Input failure: end of input (or read error) before any conversion; scanf
    /// returned EOF.
    Eof,
}

/// A byte-at-a-time reader with a single byte of pushback, mirroring the way
/// C's `scanf` consumes only as much of the stream as it needs.
pub struct Scanner<R: Read> {
    reader: R,
    buf: [u8; 4096],
    len: usize,
    pos: usize,
    pushback: Option<u8>,
    eof: bool,
}

impl<R: Read> Scanner<R> {
    pub fn new(reader: R) -> Self {
        Scanner {
            reader,
            buf: [0u8; 4096],
            len: 0,
            pos: 0,
            pushback: None,
            eof: false,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.pushback.take() {
            return Some(b);
        }
        loop {
            if self.pos < self.len {
                let b = self.buf[self.pos];
                self.pos += 1;
                return Some(b);
            }
            if self.eof {
                return None;
            }
            match self.reader.read(&mut self.buf) {
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

    fn unget(&mut self, b: u8) {
        self.pushback = Some(b);
    }

    /// Emulates glibc's `scanf("%d", &v)` for a single conversion:
    ///   * leading whitespace (C locale `isspace`) is skipped,
    ///   * an optional '+'/'-' sign is accepted,
    ///   * one or more decimal digits are required,
    ///   * the value saturates at the platform `long` bounds and is then
    ///     truncated to `int`.
    pub fn scan_int(&mut self) -> ScanResult {
        // Skip whitespace. Hitting end of input here is an input failure (EOF).
        let mut c = loop {
            match self.next_byte() {
                None => return ScanResult::Eof,
                Some(b) => {
                    if is_c_space(b) {
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
                None => return ScanResult::MatchFailure,
                Some(b) => c = b,
            }
        }

        if !c.is_ascii_digit() {
            self.unget(c);
            return ScanResult::MatchFailure;
        }

        // Accumulate the magnitude, flagging (and then ignoring) any excess so
        // that the running value never overflows.
        let mut magnitude: u128 = 0;
        let mut overflowed = false;
        loop {
            let digit = u128::from(c - b'0');
            if !overflowed {
                if magnitude > u128::from(u64::MAX) {
                    overflowed = true;
                } else {
                    magnitude = magnitude * 10 + digit;
                }
            }
            match self.next_byte() {
                None => break,
                Some(b) => {
                    if b.is_ascii_digit() {
                        c = b;
                    } else {
                        self.unget(b);
                        break;
                    }
                }
            }
        }

        // glibc saturates at LONG_MIN/LONG_MAX before narrowing to int.
        // The negative side may reach 2^63 exactly (that is LONG_MIN), so the
        // bound there is `> LONG_MAX` rather than `>= LONG_MAX`.
        let wide: i64 = if negative {
            if overflowed || magnitude > i64::MAX as u128 {
                i64::MIN
            } else {
                -(magnitude as i64)
            }
        } else if overflowed || magnitude > i64::MAX as u128 {
            i64::MAX
        } else {
            magnitude as i64
        };

        ScanResult::Value(wide as CInt)
    }
}

/// C-locale `isspace`.
pub fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Body of the C `main`, parameterised over its streams so it can be reused by
/// both the `#[no_mangle] main` export and the standalone driver binary.
pub fn main_body<R: Read, W: Write>(input: R, mut output: W) -> CInt {
    // int data[100];
    let mut data: [CInt; 100] = [0; 100];
    let mut scanner = Scanner::new(input);

    let mut i: CInt = 0;
    while i < 100 {
        match scanner.scan_int() {
            ScanResult::Value(v) => data[i as usize] = v,
            ScanResult::MatchFailure | ScanResult::Eof => break,
        }
        i += 1;
    }

    let result = call_fma_slice(&data, i);
    // C: printf("%d\n", result);
    let _ = writeln!(output, "{result}");
    let _ = output.flush();

    0
}

/// The C `main` reading real stdin and writing real stdout.
pub fn main_stdio() -> CInt {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let r = main_body(stdin.lock(), stdout.lock());
    let _ = io::stdout().flush();
    r
}
