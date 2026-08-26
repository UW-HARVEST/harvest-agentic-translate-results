/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */
//! cio.rs -- `printf`/`fgets`/`sscanf` compatibility helpers.
//!
//! The C program writes every byte of its output through `printf` on `stdout`.
//! Rather than re-buffering, this module writes straight to file descriptor 1,
//! so the byte stream (and its ordering) is exactly what C produces once the C
//! stream has been flushed.

#![allow(dead_code)]

use std::io::Write;

/// Unbuffered `write(2)` sink on file descriptor 1, i.e. C's `stdout`.
pub struct CStdout;

impl Write for CStdout {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut written: usize = 0;
        while written < buf.len() {
            let n = unsafe {
                libc::write(
                    1,
                    buf[written..].as_ptr() as *const libc::c_void,
                    buf.len() - written,
                )
            };
            if n < 0 {
                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                break;
            }
            if n == 0 {
                break;
            }
            written += n as usize;
        }
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// `printf`-alike: like `printf`, write errors are silently ignored.
macro_rules! cprintf {
    ($out:expr, $($arg:tt)*) => {{
        let _ = std::io::Write::write_fmt($out, format_args!($($arg)*));
    }};
}

/// Writes raw bytes, the way `printf("%s", ...)` emits a C string verbatim
/// (no UTF-8 validation or lossy substitution).
pub fn w(out: &mut dyn Write, bytes: &[u8]) {
    let _ = out.write_all(bytes);
}

/// Formats a `double` the way glibc's `printf("%.<prec>f", v)` does.
///
/// Rust's `{:.p$}` matches glibc for every finite value (both convert the exact
/// binary value and break ties to even), but differs for the specials: glibc
/// prints `inf`/`-inf`/`nan`/`-nan` where Rust prints `inf`/`-inf`/`NaN`.
pub fn fmt_f(v: f64, prec: usize) -> String {
    if v.is_nan() {
        // glibc honours the sign bit of the NaN payload.
        if v.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        }
    } else if v.is_infinite() {
        if v.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        }
    } else {
        format!("{:.*}", prec, v)
    }
}

/// `accumulator += value` with the exact NaN propagation of the C build.
///
/// x86-64's `ADDSD` returns the *destination* operand when both operands are
/// NaN. The C build (`cc` with no optimization, exactly as `CMakeLists.txt`
/// compiles it) emits `total += x` as "load `x`, add `total` into it", i.e. with
/// `x` as the destination — so the NaN that is being *added* survives and the
/// last NaN in a sum is the one that gets printed. LLVM, in contrast, treats
/// `fadd` as commutative and swaps the operands when optimizing, which makes the
/// *first* NaN win in a release build and prints `$-nan` where C prints `$nan`.
/// Deciding the NaN cases explicitly keeps every build profile identical to C:
///
/// * `value` is NaN         -> that NaN, quieted (what `ADDSD` yields);
/// * only `accumulator` NaN -> that NaN, quieted;
/// * otherwise              -> the plain hardware addition (so `inf + -inf`
///   still produces the platform's default NaN).
pub fn fadd_c(accumulator: f64, value: f64) -> f64 {
    if value.is_nan() {
        return quiet_nan(value);
    }
    if accumulator.is_nan() {
        return quiet_nan(accumulator);
    }
    accumulator + value
}

/// Sets the quiet bit of a NaN, preserving sign and payload, the way the SSE
/// arithmetic units quiet a signalling NaN operand.
fn quiet_nan(x: f64) -> f64 {
    f64::from_bits(x.to_bits() | 0x0008_0000_0000_0000)
}

/// Bytes of a NUL-terminated fixed buffer, as `printf("%s", ...)` sees them.
pub fn c_str(buf: &[u8]) -> &[u8] {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    &buf[..end]
}

/// Bytes of a NUL-terminated C string reached through a raw pointer.
///
/// # Safety
/// `ptr` must be non-NULL and point at a NUL-terminated buffer.
pub unsafe fn c_str_ptr(ptr: *const libc::c_char) -> &'static [u8] {
    let mut len: usize = 0;
    // `read_volatile` keeps an invalid pointer faulting the way C's `printf`
    // would, instead of tripping Rust's debug-only null-pointer assertion.
    while std::ptr::read_volatile(ptr.add(len)) != 0 {
        len += 1;
    }
    std::slice::from_raw_parts(ptr as *const u8, len)
}

/// `strcmp(a, b)`, used by `find_items_by_category`.
///
/// # Safety
/// Both pointers must reference NUL-terminated buffers.
pub unsafe fn strcmp(a: *const u8, b: *const u8) -> i32 {
    let mut i: usize = 0;
    loop {
        let ca = std::ptr::read_volatile(a.add(i));
        let cb = std::ptr::read_volatile(b.add(i));
        if ca != cb {
            return ca as i32 - cb as i32;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
}

/// `strncpy(dst, src, N - 1); dst[N - 1] = '\0';` into a fresh fixed-size
/// buffer. `strncpy` NUL-pads the destination when `src` is shorter than the
/// limit, so every one of the `N` bytes is written.
///
/// # Safety
/// `src` must reference a NUL-terminated buffer.
pub unsafe fn strncpy_field<const N: usize>(src: *const libc::c_char) -> [u8; N] {
    let mut dst = [0u8; N];
    let limit = N - 1;
    let mut i: usize = 0;
    while i < limit {
        // Volatile so that a NULL `src` faults like C's `strncpy` instead of
        // tripping Rust's debug-only null-pointer assertion.
        let b = std::ptr::read_volatile(src.add(i)) as u8;
        if b == 0 {
            break;
        }
        dst[i] = b;
        i += 1;
    }
    dst[N - 1] = 0;
    dst
}

/// `fgets(buf, size, stdin)` on file descriptor 0: reads at most `size - 1`
/// bytes, stopping after a newline (kept) or at end of input. `None` models the
/// `NULL` return, i.e. nothing at all could be read.
pub fn fgets(size: usize) -> Option<Vec<u8>> {
    let max = size - 1;
    let mut buf: Vec<u8> = Vec::new();
    while buf.len() < max {
        let mut byte = [0u8; 1];
        let n = unsafe { libc::read(0, byte.as_mut_ptr() as *mut libc::c_void, 1) };
        if n == 0 {
            break;
        }
        if n < 0 {
            if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            break;
        }
        buf.push(byte[0]);
        if byte[0] == b'\n' {
            break;
        }
    }
    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

/// `sscanf(input, "%d", &out)`: `Some(value)` when the conversion succeeds
/// (return value 1), `None` on a matching failure or input failure.
///
/// Mirrors glibc, which accumulates through `strtol` (saturating at
/// `LONG_MIN`/`LONG_MAX`) and then stores the truncated `int`.
pub fn sscanf_d(input: &[u8]) -> Option<i32> {
    // A C string ends at the first NUL byte.
    let s = {
        let end = input.iter().position(|&b| b == 0).unwrap_or(input.len());
        &input[..end]
    };

    let mut i = 0usize;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let start_digits = i;
    let mut acc: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = (s[i] - b'0') as i64;
        acc = if negative {
            acc.saturating_mul(10).saturating_sub(digit)
        } else {
            acc.saturating_mul(10).saturating_add(digit)
        };
        i += 1;
    }

    if i == start_digits {
        // No digits: matching failure (0), or input failure (EOF) if the input
        // ran out while skipping whitespace. Neither equals 1.
        return None;
    }

    Some(acc as i32)
}
