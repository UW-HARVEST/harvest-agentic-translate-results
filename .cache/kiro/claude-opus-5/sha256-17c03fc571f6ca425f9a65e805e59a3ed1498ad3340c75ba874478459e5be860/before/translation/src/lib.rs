// Rust translation of c_src/src/lib.c
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

use std::ffi::c_int;

/// `static int memchra(const char *str, int c, size_t n)`
///
/// Counts bytes in `str[0..n]` equal to `(char)c`.
fn memchra(s: &[u8], c: c_int, n: usize) -> c_int {
    let mut count: c_int = 0;
    // The C code truncates `c` to `char` before comparing.
    let needle = c as u8;
    for i in 0..n {
        if s[i] == needle {
            count += 1;
        }
    }
    count
}

/// `static int process_buffer(char *buffer, size_t len)`
///
/// Returns -1 for a NULL or empty buffer, otherwise the sum of the signed
/// `char` values up to `len` bytes or the first NUL, whichever comes first.
fn process_buffer(buffer: &[u8], len: usize) -> c_int {
    // `buffer == NULL || *buffer == '\0'`
    if buffer.is_empty() || buffer[0] == 0 {
        return -1;
    }

    let mut result: c_int = 0;
    let mut idx = 0usize;
    while idx < len && buffer[idx] != 0 {
        // `char` is signed on the platforms this library targets, so the
        // promotion to `int` sign-extends.
        result = result.wrapping_add(buffer[idx] as i8 as c_int);
        idx += 1;
    }
    result
}

/// `static float int_to_float_bits(int value)`
///
/// Reinterprets the object representation of an `int` as a `float`.
fn int_to_float_bits(value: c_int) -> f32 {
    f32::from_bits(value as u32)
}

/// `static int process_strings(char **strings, int count, const char *target)`
///
/// Counts the entries of `strings` that begin with `target`. NULL and empty
/// entries are skipped.
fn process_strings(strings: &[&[u8]], count: c_int, target: &[u8]) -> c_int {
    // `strings == NULL || count <= 0`
    if count <= 0 {
        return 0;
    }

    let mut matches: c_int = 0;

    for i in 0..(count as usize) {
        let s = strings[i];
        // `*i == NULL || **i == '\0'`
        if s.is_empty() {
            continue;
        }

        // strncmp(*i, target, strlen(target)) == 0
        if strncmp_prefix(s, target) {
            matches += 1;
        }
    }

    matches
}

/// Emulates `strncmp(s, target, strlen(target)) == 0` for NUL-terminated
/// strings represented here as NUL-free byte slices.
fn strncmp_prefix(s: &[u8], target: &[u8]) -> bool {
    if s.len() < target.len() {
        // `s` ends before `target` does, so the terminating NUL of `s`
        // compares unequal against the corresponding byte of `target`.
        return false;
    }
    &s[..target.len()] == target
}

/// `static int safe_sum_array(int *arr, size_t size)`
fn safe_sum_array(arr: &[c_int], size: usize) -> c_int {
    // `arr == NULL || size == 0`
    if size == 0 {
        return 0;
    }

    let mut sum: c_int = 0;

    for i in 0..size {
        sum = sum.wrapping_add(arr[i]);
    }

    sum
}

/// `static int interpret_as_int(unsigned char *bytes, size_t len)`
///
/// Reinterprets the first `sizeof(int)` bytes as an `int`.
fn interpret_as_int(bytes: &[u8], len: usize) -> c_int {
    // `bytes == NULL || len < sizeof(int)`
    if bytes.is_empty() || len < core::mem::size_of::<c_int>() {
        return 0;
    }

    let mut raw = [0u8; core::mem::size_of::<c_int>()];
    raw.copy_from_slice(&bytes[..core::mem::size_of::<c_int>()]);
    // Little-endian byte order, matching the target platform.
    c_int::from_le_bytes(raw)
}

/// `static int count_occurrences(const char *text, char ch)`
fn count_occurrences(text: &[u8], ch: u8) -> c_int {
    // `text == NULL || *text == '\0'`
    if text.is_empty() {
        return 0;
    }

    let len = text.len();
    memchra(text, ch as i8 as c_int, len)
}

/// `static int complex_iteration(int *data, size_t count)`
fn complex_iteration(data: &[c_int], count: usize) -> c_int {
    // `data == NULL || count == 0`
    if count == 0 {
        return -1;
    }

    let mut result: c_int = 0;

    for i in 0..count {
        let u = data[i] as u32;
        result ^= (u & 0xFF) as c_int;
    }

    result
}

/// Emulates `snprintf(buffer, size, ...)` into a fixed-size C char buffer:
/// writes at most `size - 1` bytes of `text` followed by a NUL terminator.
fn snprintf_into(buffer: &mut [u8], text: &str) -> usize {
    let size = buffer.len();
    if size == 0 {
        return 0;
    }
    let bytes = text.as_bytes();
    let n = core::cmp::min(bytes.len(), size - 1);
    buffer[..n].copy_from_slice(&bytes[..n]);
    buffer[n] = 0;
    n
}

/// `int memchra2(int a, int b, int c, int d)`
#[unsafe(no_mangle)]
pub extern "C" fn memchra2(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut result: c_int = 0;

    let mut buffer = [0u8; 64];
    let formatted = format!("test{}-{}-{}-{}", a, b, c, d);
    let buf_len = snprintf_into(&mut buffer, &formatted);
    // The NUL-terminated contents, as `strlen` would see them.
    let buffer_str = &buffer[..buf_len];

    let dash_count = count_occurrences(buffer_str, b'-');
    result = result.wrapping_add(dash_count.wrapping_mul(10));

    let values: [c_int; 4] = [a, b, c, d];
    let sum = safe_sum_array(&values, 4);
    result = result.wrapping_add(sum);

    let test_strings: [&[u8]; 4] = [b"test1", b"test2", b"testing", b"other"];

    let matches = process_strings(&test_strings, 4, b"test");
    result = result.wrapping_add(matches.wrapping_mul(5));

    let f = int_to_float_bits(a);
    if f > 0.0f32 && f < 1000.0f32 {
        result = result.wrapping_add(f as c_int);
    }

    let buf_sum = process_buffer(buffer_str, buf_len);
    if buf_sum > 0 {
        result = result.wrapping_add(buf_sum % 256);
    }

    let mut bytes = [0u8; 4];
    bytes[0] = (b & 0xFF) as u8;
    bytes[1] = (c & 0xFF) as u8;
    bytes[2] = (d & 0xFF) as u8;
    bytes[3] = 0;

    let interpreted = interpret_as_int(&bytes, 4);
    result ^= interpreted;

    let complex_result = complex_iteration(&values, 4);
    result = result.wrapping_add(complex_result);

    result
}
