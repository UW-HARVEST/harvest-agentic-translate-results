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
/// Counts the bytes in `str[0..n]` that equal `(char)c`.
fn memchra(str_: &[u8], c: c_int, n: usize) -> c_int {
    let mut count: c_int = 0;
    // `(char)c` -- narrowing conversion of the int argument to a char.
    let needle = (c as u32 & 0xFF) as u8;
    let mut i: usize = 0;
    while i < n {
        if str_[i] == needle {
            count = count.wrapping_add(1);
        }
        i += 1;
    }
    count
}

/// static int process_buffer(char *buffer, size_t len)
///
/// `buffer` is never NULL in the Rust translation (it is always a real slice),
/// so only the empty-string check remains observable.
fn process_buffer(buffer: &[u8], len: usize) -> c_int {
    if buffer.is_empty() || buffer[0] == b'\0' {
        return -1;
    }

    let mut result: c_int = 0;
    let mut i: usize = 0;
    while i < len && buffer[i] != b'\0' {
        // `char` is signed on the reference platform (x86-64 Linux), so the
        // byte is sign-extended when converted to int.
        result = result.wrapping_add(buffer[i] as i8 as c_int);
        i += 1;
    }
    result
}

/// static float int_to_float_bits(int value)
///
/// Type-punning union: reinterpret the object representation of the int as a
/// float.
fn int_to_float_bits(value: c_int) -> f32 {
    f32::from_bits(value as u32)
}

/// static int process_strings(char **strings, int count, const char *target)
///
/// Counts how many of the `count` strings start with `target`.
fn process_strings(strings: &[&[u8]], count: c_int, target: &[u8]) -> c_int {
    if count <= 0 {
        return 0;
    }

    let mut matches: c_int = 0;

    for i in 0..(count as usize) {
        let s = strings[i];

        // `*i == NULL || **i == '\0'`
        if s.is_empty() || s[0] == b'\0' {
            continue;
        }

        // strncmp(*i, target, strlen(target)) == 0
        if strncmp_prefix(s, target) {
            matches = matches.wrapping_add(1);
        }
    }

    matches
}

/// Emulates `strncmp(a, target, strlen(target)) == 0`, i.e. "does `a` begin
/// with `target`" for NUL-terminated C strings.
fn strncmp_prefix(a: &[u8], target: &[u8]) -> bool {
    let n = cstr_len(target);
    let ta = &target[..n];
    let alen = cstr_len(a);
    if alen < n {
        return false;
    }
    &a[..n] == ta
}

/// strlen() over a byte slice that is expected to be NUL-terminated (or to be
/// entirely payload).
fn cstr_len(s: &[u8]) -> usize {
    match s.iter().position(|&b| b == b'\0') {
        Some(p) => p,
        None => s.len(),
    }
}

/// static int safe_sum_array(int *arr, size_t size)
fn safe_sum_array(arr: &[c_int], size: usize) -> c_int {
    if size == 0 {
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
/// Reinterprets the first `sizeof(int)` bytes as an int (little endian on the
/// reference platform).
fn interpret_as_int(bytes: &[u8], len: usize) -> c_int {
    if len < core::mem::size_of::<c_int>() {
        return 0;
    }

    let mut raw = [0u8; 4];
    raw.copy_from_slice(&bytes[..4]);
    c_int::from_le_bytes(raw)
}

/// static int count_occurrences(const char *text, char ch)
fn count_occurrences(text: &[u8], ch: u8) -> c_int {
    if text.is_empty() || text[0] == b'\0' {
        return 0;
    }

    let len = cstr_len(text);
    memchra(text, ch as i8 as c_int, len)
}

/// static int complex_iteration(int *data, size_t count)
fn complex_iteration(data: &[c_int], count: usize) -> c_int {
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

/// int memchra2(int a, int b, int c, int d)
#[unsafe(no_mangle)]
pub extern "C" fn memchra2(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut result: c_int = 0;

    // char buffer[64];
    // snprintf(buffer, sizeof(buffer), "test%d-%d-%d-%d", a, b, c, d);
    let mut buffer = [0u8; 64];
    snprintf_buffer(&mut buffer, a, b, c, d);

    let dash_count = count_occurrences(&buffer, b'-');
    result = result.wrapping_add(dash_count.wrapping_mul(10));

    let values: [c_int; 4] = [a, b, c, d];
    let sum = safe_sum_array(&values, 4);
    result = result.wrapping_add(sum);

    let test_strings: [&[u8]; 4] = [b"test1\0", b"test2\0", b"testing\0", b"other\0"];

    let matches = process_strings(&test_strings, 4, b"test\0");
    result = result.wrapping_add(matches.wrapping_mul(5));

    let f = int_to_float_bits(a);
    if f > 0.0f32 && f < 1000.0f32 {
        result = result.wrapping_add(f as c_int);
    }

    let buf_len = cstr_len(&buffer);
    let buf_sum = process_buffer(&buffer, buf_len);
    if buf_sum > 0 {
        result = result.wrapping_add(buf_sum % 256);
    }

    // unsigned char bytes[4];
    let mut bytes = [0u8; 4];
    bytes[0] = (b as u32 & 0xFF) as u8;
    bytes[1] = (c as u32 & 0xFF) as u8;
    bytes[2] = (d as u32 & 0xFF) as u8;
    bytes[3] = 0;

    let interpreted = interpret_as_int(&bytes, 4);
    result ^= interpreted;

    let complex_result = complex_iteration(&values, 4);
    result = result.wrapping_add(complex_result);

    result
}

/// Reproduces `snprintf(buffer, sizeof(buffer), "test%d-%d-%d-%d", a, b, c, d)`
/// including truncation behaviour and NUL termination.
fn snprintf_buffer(buffer: &mut [u8; 64], a: c_int, b: c_int, c: c_int, d: c_int) {
    let mut out: Vec<u8> = Vec::with_capacity(64);
    out.extend_from_slice(b"test");
    fmt_int(&mut out, a);
    out.push(b'-');
    fmt_int(&mut out, b);
    out.push(b'-');
    fmt_int(&mut out, c);
    out.push(b'-');
    fmt_int(&mut out, d);

    // snprintf writes at most sizeof(buffer) - 1 characters plus a NUL.
    let cap = buffer.len() - 1;
    let n = if out.len() < cap { out.len() } else { cap };
    buffer[..n].copy_from_slice(&out[..n]);
    buffer[n] = 0;
    for slot in buffer[n + 1..].iter_mut() {
        *slot = 0;
    }
}

/// Formats a C `int` exactly like the `%d` conversion specifier.
fn fmt_int(out: &mut Vec<u8>, value: c_int) {
    let mut tmp = [0u8; 11];
    let negative = value < 0;
    // Use the unsigned magnitude so that INT_MIN is handled correctly.
    let mut mag: u32 = if negative {
        (value as i64).unsigned_abs() as u32
    } else {
        value as u32
    };

    let mut idx = tmp.len();
    if mag == 0 {
        idx -= 1;
        tmp[idx] = b'0';
    } else {
        while mag > 0 {
            idx -= 1;
            tmp[idx] = b'0' + (mag % 10) as u8;
            mag /= 10;
        }
    }

    if negative {
        out.push(b'-');
    }
    out.extend_from_slice(&tmp[idx..]);
}
