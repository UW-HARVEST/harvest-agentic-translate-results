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

//! Rust translation of `c_src/src/lib.c`.
//!
//! The C library compiles a single translation unit whose only exported
//! (non-`static`) symbol is `memchra2`; every other routine in the file has
//! internal linkage and is therefore translated as a private Rust function.
//!
//! Behaviour — including integer wrap-around, the raw `int`/`float` type pun,
//! the little-endian byte reinterpretation and the exact order of the early
//! return / validation checks — is reproduced verbatim. No bugs are "fixed".

use std::ffi::c_int;
use std::fmt::Write as _;

/// `static int memchra(const char *str, int c, size_t n)`
///
/// Counts how many of the first `n` bytes of `str` equal `(char)c`.
/// The C comparison is performed after truncating `c` to `char`, so only the
/// low 8 bits of `c` participate.
fn memchra(str_: &[u8], c: c_int, n: usize) -> c_int {
    let mut count: c_int = 0;
    let needle = c as u8; // (char)c — narrowing conversion, compared bitwise
    for i in 0..n {
        if str_[i] == needle {
            count = count.wrapping_add(1);
        }
    }
    count
}

/// `static int process_buffer(char *buffer, size_t len)`
///
/// Returns -1 for a NULL or empty buffer, otherwise the sum of the byte values
/// (as signed `char`, i.e. sign-extended on x86-64) up to `len` bytes or the
/// first NUL, whichever comes first.
fn process_buffer(buffer: Option<&[u8]>, len: usize) -> c_int {
    let buffer = match buffer {
        None => return -1,
        Some(b) => {
            // `*buffer == '\0'`
            if b.first().copied().unwrap_or(0) == 0 {
                return -1;
            }
            b
        }
    };

    let mut result: c_int = 0;
    for idx in 0..len {
        let byte = buffer[idx];
        if byte == 0 {
            break;
        }
        // `result += (int)(*i)` where `*i` has type `char` (signed on x86-64).
        result = result.wrapping_add(byte as i8 as c_int);
    }
    result
}

/// `static float int_to_float_bits(int value)`
///
/// Type pun through a union: reinterprets the object representation of an
/// `int` as a `float`.
fn int_to_float_bits(value: c_int) -> f32 {
    f32::from_bits(value as u32)
}

/// `static int process_strings(char **strings, int count, const char *target)`
///
/// Counts the entries of `strings` that start with `target`
/// (`strncmp(*i, target, strlen(target)) == 0`). NULL / empty entries are
/// skipped.
fn process_strings(strings: Option<&[Option<&[u8]>]>, count: c_int, target: &[u8]) -> c_int {
    let strings = match strings {
        None => return 0,
        Some(s) => {
            if count <= 0 {
                return 0;
            }
            s
        }
    };

    let mut matches: c_int = 0;

    for idx in 0..count as usize {
        let s = match strings[idx] {
            None => continue,                      // `*i == NULL`
            Some(s) if s.first().copied().unwrap_or(0) == 0 => continue, // `**i == '\0'`
            Some(s) => s,
        };

        // strncmp(*i, target, strlen(target)) == 0
        let n = target.len();
        if s.len() >= n && &s[..n] == target {
            matches = matches.wrapping_add(1);
        }
    }

    matches
}

/// `static int safe_sum_array(int *arr, size_t size)`
fn safe_sum_array(arr: Option<&[c_int]>, size: usize) -> c_int {
    let arr = match arr {
        None => return 0,
        Some(a) => {
            if size == 0 {
                return 0;
            }
            a
        }
    };

    let mut sum: c_int = 0;
    for idx in 0..size {
        sum = sum.wrapping_add(arr[idx]);
    }
    sum
}

/// `static int interpret_as_int(unsigned char *bytes, size_t len)`
///
/// Reinterprets the first `sizeof(int)` bytes as an `int` (little-endian on the
/// target the C library is built for). Returns 0 when there are not enough
/// bytes.
fn interpret_as_int(bytes: Option<&[u8]>, len: usize) -> c_int {
    let bytes = match bytes {
        None => return 0,
        Some(b) => {
            if len < std::mem::size_of::<c_int>() {
                return 0;
            }
            b
        }
    };

    let mut raw = [0u8; 4];
    raw.copy_from_slice(&bytes[..4]);
    c_int::from_le_bytes(raw)
}

/// `static int count_occurrences(const char *text, char ch)`
fn count_occurrences(text: Option<&[u8]>, ch: u8) -> c_int {
    let text = match text {
        None => return 0,
        Some(t) => {
            if t.first().copied().unwrap_or(0) == 0 {
                return 0;
            }
            t
        }
    };

    // size_t len = strlen(text);
    let len = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    memchra(text, ch as c_int, len)
}

/// `static int complex_iteration(int *data, size_t count)`
fn complex_iteration(data: Option<&[c_int]>, count: usize) -> c_int {
    let data = match data {
        None => return -1,
        Some(d) => {
            if count == 0 {
                return -1;
            }
            d
        }
    };

    let mut result: c_int = 0;
    for idx in 0..count {
        let u = data[idx] as u32;
        result ^= (u & 0xFF) as c_int;
    }
    result
}

/// Emulates `snprintf(buffer, size, ...)` into a fixed-size C character array:
/// at most `size - 1` bytes of `text` are copied and a NUL terminator is
/// always appended (when `size != 0`).
fn snprintf_into(buffer: &mut [u8], text: &str) {
    let size = buffer.len();
    if size == 0 {
        return;
    }
    let src = text.as_bytes();
    let n = src.len().min(size - 1);
    buffer[..n].copy_from_slice(&src[..n]);
    buffer[n] = 0;
}

/// `int memchra2(int a, int b, int c, int d)` — the library's only exported
/// symbol.
#[unsafe(no_mangle)]
pub extern "C" fn memchra2(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut result: c_int = 0;

    // char buffer[64];
    // snprintf(buffer, sizeof(buffer), "test%d-%d-%d-%d", a, b, c, d);
    let mut buffer = [0u8; 64];
    let mut formatted = String::new();
    let _ = write!(formatted, "test{}-{}-{}-{}", a, b, c, d);
    snprintf_into(&mut buffer, &formatted);

    let dash_count = count_occurrences(Some(&buffer), b'-');
    result = result.wrapping_add(dash_count.wrapping_mul(10));

    let values: [c_int; 4] = [a, b, c, d];
    let sum = safe_sum_array(Some(&values), 4);
    result = result.wrapping_add(sum);

    let test_strings: [Option<&[u8]>; 4] = [
        Some(b"test1\0"),
        Some(b"test2\0"),
        Some(b"testing\0"),
        Some(b"other\0"),
    ];

    let matches = process_strings(Some(&test_strings), 4, b"test");
    result = result.wrapping_add(matches.wrapping_mul(5));

    let f = int_to_float_bits(a);
    if f > 0.0f32 && f < 1000.0f32 {
        // (int)f — truncation toward zero; f is known to be in (0, 1000) here.
        result = result.wrapping_add(f as c_int);
    }

    // int buf_sum = process_buffer(buffer, strlen(buffer));
    let buf_len = buffer.iter().position(|&x| x == 0).unwrap_or(buffer.len());
    let buf_sum = process_buffer(Some(&buffer), buf_len);
    if buf_sum > 0 {
        result = result.wrapping_add(buf_sum % 256);
    }

    let mut bytes = [0u8; 4];
    bytes[0] = (b & 0xFF) as u8;
    bytes[1] = (c & 0xFF) as u8;
    bytes[2] = (d & 0xFF) as u8;
    bytes[3] = 0;

    let interpreted = interpret_as_int(Some(&bytes), 4);
    result ^= interpreted;

    let complex_result = complex_iteration(Some(&values), 4);
    result = result.wrapping_add(complex_result);

    result
}
