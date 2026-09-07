// Rust translation of c_src/src/lib.c
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

/// static int memchra(const char *str, int c, size_t n)
///
/// Counts the bytes of `str[0..n]` equal to `(char)c`. On the reference
/// platform `char` is signed, so the comparison is performed on the
/// truncated low byte of `c`.
fn memchra(s: &[u8], c: c_int, n: usize) -> c_int {
    let mut count: c_int = 0;
    let target = c as u8; // (char)c
    for i in 0..n {
        if s[i] == target {
            count = count.wrapping_add(1);
        }
    }
    count
}

/// static int process_buffer(char *buffer, size_t len)
///
/// `buffer` is modeled as the whole backing array; a NULL pointer cannot be
/// produced by the single caller, so only the "empty string" early return is
/// reachable. Bytes are accumulated as *signed* char values, matching the
/// `(int)(*i)` conversion of the C code on the reference platform.
fn process_buffer(buffer: &[u8], len: usize) -> c_int {
    // if (buffer == NULL || *buffer == '\0') return -1;
    if buffer.is_empty() || buffer[0] == 0 {
        return -1;
    }

    let mut result: c_int = 0;
    let mut i = 0usize;
    while i < len && buffer[i] != 0 {
        result = result.wrapping_add((buffer[i] as i8) as c_int);
        i += 1;
    }
    result
}

/// static float int_to_float_bits(int value)
///
/// Type punning through a union: reinterpret the object representation of the
/// int as a float.
fn int_to_float_bits(value: c_int) -> f32 {
    f32::from_bits(value as u32)
}

/// static int process_strings(char **strings, int count, const char *target)
///
/// Counts the non-NULL, non-empty entries of `strings` that start with
/// `target` (`strncmp(*i, target, strlen(target)) == 0`).
fn process_strings(strings: &[&[u8]], count: c_int, target: &[u8]) -> c_int {
    // if (strings == NULL || count <= 0) return 0;
    if strings.is_empty() || count <= 0 {
        return 0;
    }

    let mut matches: c_int = 0;

    for entry in strings.iter().take(count as usize) {
        // if (*i == NULL || **i == '\0') continue;
        if entry.is_empty() || entry[0] == 0 {
            continue;
        }

        // strncmp(*i, target, strlen(target)) == 0
        if strncmp_prefix(entry, target) {
            matches = matches.wrapping_add(1);
        }
    }

    matches
}

/// `strncmp(a, b, b.len()) == 0` where both operands are NUL-terminated C
/// strings represented here without their terminator.
fn strncmp_prefix(a: &[u8], b: &[u8]) -> bool {
    for i in 0..b.len() {
        let ac = if i < a.len() { a[i] } else { 0 };
        let bc = b[i];
        if ac != bc {
            return false;
        }
        if ac == 0 {
            return true;
        }
    }
    true
}

/// static int safe_sum_array(int *arr, size_t size)
fn safe_sum_array(arr: &[c_int], size: usize) -> c_int {
    // if (arr == NULL || size == 0) return 0;
    if arr.is_empty() || size == 0 {
        return 0;
    }

    let mut sum: c_int = 0;

    for i in 0..size {
        sum = sum.wrapping_add(arr[i]);
    }

    sum
}

/// static int interpret_as_int(unsigned char *bytes, size_t len)
///
/// Reinterprets the first `sizeof(int)` bytes as an `int`; the reference
/// platform is little-endian.
fn interpret_as_int(bytes: &[u8], len: usize) -> c_int {
    // if (bytes == NULL || len < sizeof(int)) return 0;
    if bytes.is_empty() || len < core::mem::size_of::<c_int>() {
        return 0;
    }

    c_int::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// static int count_occurrences(const char *text, char ch)
fn count_occurrences(text: &[u8], ch: u8) -> c_int {
    // if (text == NULL || *text == '\0') return 0;
    if text.is_empty() || text[0] == 0 {
        return 0;
    }

    let len = c_strlen(text);
    memchra(text, ch as i8 as c_int, len)
}

/// static int complex_iteration(int *data, size_t count)
fn complex_iteration(data: &[c_int], count: usize) -> c_int {
    // if (data == NULL || count == 0) return -1;
    if data.is_empty() || count == 0 {
        return -1;
    }

    let mut result: c_int = 0;

    for i in 0..count {
        let u = data[i] as u32;
        result ^= (u & 0xFF) as c_int;
    }

    result
}

/// `strlen` over a byte buffer holding a NUL-terminated string.
fn c_strlen(buf: &[u8]) -> usize {
    match buf.iter().position(|&b| b == 0) {
        Some(p) => p,
        None => buf.len(),
    }
}

/// Emulates `snprintf(buffer, size, ...)` into a fixed-size `char` array:
/// at most `size - 1` bytes of `text` are copied, followed by a NUL byte.
fn snprintf_into(buffer: &mut [u8], text: &[u8]) {
    if buffer.is_empty() {
        return;
    }
    let max = buffer.len() - 1;
    let n = if text.len() < max { text.len() } else { max };
    buffer[..n].copy_from_slice(&text[..n]);
    buffer[n] = 0;
}

#[unsafe(no_mangle)]
pub extern "C" fn memchra2(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut result: c_int = 0;

    // char buffer[64];
    // snprintf(buffer, sizeof(buffer), "test%d-%d-%d-%d", a, b, c, d);
    let mut buffer = [0u8; 64];
    let formatted = format!("test{}-{}-{}-{}", a, b, c, d);
    snprintf_into(&mut buffer, formatted.as_bytes());

    let dash_count = count_occurrences(&buffer, b'-');
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

    let buf_len = c_strlen(&buffer);
    let buf_sum = process_buffer(&buffer, buf_len);
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
