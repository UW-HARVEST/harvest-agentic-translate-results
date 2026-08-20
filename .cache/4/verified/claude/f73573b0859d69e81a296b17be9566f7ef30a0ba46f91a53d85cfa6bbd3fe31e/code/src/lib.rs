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
//
// The eight helpers below are `static` in the C translation unit, so they are
// private here as well.  They keep the C signatures (raw pointers, `size_t`
// lengths) one-for-one so that every guard the C performs -- including the NULL
// checks -- lives in exactly the same place as in the original.

use core::ffi::{c_char, c_int, c_uchar};

// ---------------------------------------------------------------------------
// libc helpers used by the C source (`strlen`, `strncmp`).
// Re-implemented instead of linking libc so the crate has no dependencies; the
// observable behaviour is identical.
// ---------------------------------------------------------------------------

/// `strlen(s)`
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n: usize = 0;
    while *s.add(n) != 0 {
        n += 1;
    }
    n
}

/// `strncmp(a, b, n)` -- compares as `unsigned char`, stops at a difference, at
/// a NUL byte, or after `n` bytes.
unsafe fn c_strncmp(a: *const c_char, b: *const c_char, n: usize) -> c_int {
    let mut i: usize = 0;
    while i < n {
        let ca = *a.add(i) as c_uchar;
        let cb = *b.add(i) as c_uchar;
        if ca != cb {
            return ca as c_int - cb as c_int;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
    0
}

// ---------------------------------------------------------------------------
// static int memchra(const char *str, int c, size_t n)
// ---------------------------------------------------------------------------

/// Counts the bytes in `str[0..n]` that equal `(char)c`.
unsafe fn memchra(str_: *const c_char, c: c_int, n: usize) -> c_int {
    let mut count: c_int = 0;
    // `(char)c` -- narrowing conversion of the int argument to a char.
    let needle = c as c_char;
    let mut i: usize = 0;
    while i < n {
        if *str_.add(i) == needle {
            count = count.wrapping_add(1);
        }
        i += 1;
    }
    count
}

// ---------------------------------------------------------------------------
// static int process_buffer(char *buffer, size_t len)
// ---------------------------------------------------------------------------

unsafe fn process_buffer(buffer: *mut c_char, len: usize) -> c_int {
    if buffer.is_null() || *buffer == 0 {
        return -1;
    }

    let mut result: c_int = 0;
    let mut i: usize = 0;
    // `for (char *i = buffer; i < buffer + len && *i != '\0'; i++)`
    while i < len && *buffer.add(i) != 0 {
        // `char` is signed on the reference platform (x86-64 Linux), so the
        // byte is sign-extended when converted to int.
        result = result.wrapping_add(*buffer.add(i) as c_int);
        i += 1;
    }
    result
}

// ---------------------------------------------------------------------------
// static float int_to_float_bits(int value)
// ---------------------------------------------------------------------------

/// Type-punning union: reinterpret the object representation of the int as a
/// float.
unsafe fn int_to_float_bits(value: c_int) -> f32 {
    f32::from_bits(value as u32)
}

// ---------------------------------------------------------------------------
// static int process_strings(char **strings, int count, const char *target)
// ---------------------------------------------------------------------------

unsafe fn process_strings(
    strings: *mut *mut c_char,
    count: c_int,
    target: *const c_char,
) -> c_int {
    if strings.is_null() || count <= 0 {
        return 0;
    }

    let mut matches: c_int = 0;

    // `for (char **i = strings; i < strings + count; i++)`
    let mut k: usize = 0;
    while k < count as usize {
        let s: *mut c_char = *strings.add(k);
        k += 1;

        // `if (*i == NULL || **i == '\0') continue;`
        if s.is_null() || *s == 0 {
            continue;
        }

        // `if (strncmp(*i, target, strlen(target)) == 0) matches++;`
        if c_strncmp(s, target, c_strlen(target)) == 0 {
            matches = matches.wrapping_add(1);
        }
    }

    matches
}

// ---------------------------------------------------------------------------
// static int safe_sum_array(int *arr, size_t size)
// ---------------------------------------------------------------------------

unsafe fn safe_sum_array(arr: *mut c_int, size: usize) -> c_int {
    if arr.is_null() || size == 0 {
        return 0;
    }

    let mut sum: c_int = 0;

    let mut i: usize = 0;
    while i < size {
        // Signed overflow is UB in C; gcc wraps two's-complement.
        sum = sum.wrapping_add(*arr.add(i));
        i += 1;
    }

    sum
}

// ---------------------------------------------------------------------------
// static int interpret_as_int(unsigned char *bytes, size_t len)
// ---------------------------------------------------------------------------

/// Reinterprets the first `sizeof(int)` bytes as an int (native byte order,
/// possibly misaligned -- x86-64 gcc emits a plain load).
unsafe fn interpret_as_int(bytes: *mut c_uchar, len: usize) -> c_int {
    if bytes.is_null() || len < core::mem::size_of::<c_int>() {
        return 0;
    }

    let int_ptr = bytes as *const c_int;
    int_ptr.read_unaligned()
}

// ---------------------------------------------------------------------------
// static int count_occurrences(const char *text, char ch)
// ---------------------------------------------------------------------------

unsafe fn count_occurrences(text: *const c_char, ch: c_char) -> c_int {
    if text.is_null() || *text == 0 {
        return 0;
    }

    let len = c_strlen(text);
    // `memchra(text, ch, len)` -- `ch` is promoted from (signed) char to int.
    memchra(text, ch as c_int, len)
}

// ---------------------------------------------------------------------------
// static int complex_iteration(int *data, size_t count)
// ---------------------------------------------------------------------------

unsafe fn complex_iteration(data: *mut c_int, count: usize) -> c_int {
    if data.is_null() || count == 0 {
        return -1;
    }

    let mut result: c_int = 0;

    let mut i: usize = 0;
    while i < count {
        let u = *data.add(i) as u32;
        result ^= (u & 0xFF) as c_int;
        i += 1;
    }

    result
}

// ---------------------------------------------------------------------------
// int memchra2(int a, int b, int c, int d)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn memchra2(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    unsafe {
        let mut result: c_int = 0;

        // char buffer[64];
        // snprintf(buffer, sizeof(buffer), "test%d-%d-%d-%d", a, b, c, d);
        let mut buffer = [0 as c_char; 64];
        snprintf_test_pattern(&mut buffer, a, b, c, d);

        let dash_count = count_occurrences(buffer.as_ptr(), b'-' as c_char);
        result = result.wrapping_add(dash_count.wrapping_mul(10));

        let mut values: [c_int; 4] = [a, b, c, d];
        let sum = safe_sum_array(values.as_mut_ptr(), 4);
        result = result.wrapping_add(sum);

        let s0 = b"test1\0";
        let s1 = b"test2\0";
        let s2 = b"testing\0";
        let s3 = b"other\0";
        let mut test_strings: [*mut c_char; 4] = [
            s0.as_ptr() as *mut c_char,
            s1.as_ptr() as *mut c_char,
            s2.as_ptr() as *mut c_char,
            s3.as_ptr() as *mut c_char,
        ];

        let target = b"test\0";
        let matches = process_strings(
            test_strings.as_mut_ptr(),
            4,
            target.as_ptr() as *const c_char,
        );
        result = result.wrapping_add(matches.wrapping_mul(5));

        let f = int_to_float_bits(a);
        if f > 0.0f32 && f < 1000.0f32 {
            // `(int)f` -- f is known to be in (0, 1000) here, so the C
            // truncation is well defined and matches Rust's saturating `as`.
            result = result.wrapping_add(f as c_int);
        }

        let buf_sum = process_buffer(buffer.as_mut_ptr(), c_strlen(buffer.as_ptr()));
        if buf_sum > 0 {
            result = result.wrapping_add(buf_sum % 256);
        }

        // unsigned char bytes[4];
        let mut bytes = [0 as c_uchar; 4];
        bytes[0] = (b as u32 & 0xFF) as c_uchar;
        bytes[1] = (c as u32 & 0xFF) as c_uchar;
        bytes[2] = (d as u32 & 0xFF) as c_uchar;
        bytes[3] = 0;

        let interpreted = interpret_as_int(bytes.as_mut_ptr(), 4);
        result ^= interpreted;

        let complex_result = complex_iteration(values.as_mut_ptr(), 4);
        result = result.wrapping_add(complex_result);

        result
    }
}

// ---------------------------------------------------------------------------
// snprintf(buffer, sizeof(buffer), "test%d-%d-%d-%d", a, b, c, d)
// ---------------------------------------------------------------------------

/// Reproduces the single `snprintf` call site, including its truncation
/// behaviour and NUL termination.  (The formatted output is at most 51 bytes,
/// so truncation is unreachable for this format string, but it is implemented
/// anyway.)
fn snprintf_test_pattern(buffer: &mut [c_char; 64], a: c_int, b: c_int, c: c_int, d: c_int) {
    let mut out = [0u8; 96];
    let mut n: usize = 0;

    for &byte in b"test" {
        out[n] = byte;
        n += 1;
    }
    n = fmt_int(&mut out, n, a);
    out[n] = b'-';
    n += 1;
    n = fmt_int(&mut out, n, b);
    out[n] = b'-';
    n += 1;
    n = fmt_int(&mut out, n, c);
    out[n] = b'-';
    n += 1;
    n = fmt_int(&mut out, n, d);

    // snprintf writes at most `size - 1` characters and always NUL-terminates.
    let cap = buffer.len() - 1;
    let written = if n < cap { n } else { cap };
    let mut i: usize = 0;
    while i < written {
        buffer[i] = out[i] as c_char;
        i += 1;
    }
    buffer[written] = 0;
}

/// Formats a C `int` exactly like the `%d` conversion specifier, appending to
/// `out` at offset `at`; returns the new offset.
fn fmt_int(out: &mut [u8; 96], at: usize, value: c_int) -> usize {
    let mut tmp = [0u8; 10];
    let negative = value < 0;
    // Use the unsigned magnitude so that INT_MIN is handled correctly.
    let mut mag: u32 = if negative {
        (value as i64).unsigned_abs() as u32
    } else {
        value as u32
    };

    let mut ndigits: usize = 0;
    if mag == 0 {
        tmp[0] = b'0';
        ndigits = 1;
    } else {
        while mag > 0 {
            tmp[ndigits] = b'0' + (mag % 10) as u8;
            mag /= 10;
            ndigits += 1;
        }
    }

    let mut n = at;
    if negative {
        out[n] = b'-';
        n += 1;
    }
    let mut i = ndigits;
    while i > 0 {
        i -= 1;
        out[n] = tmp[i];
        n += 1;
    }
    n
}

// ---------------------------------------------------------------------------
// Optional test-only exports (feature `internal_test_api`, off by default).
//
// The C helpers are `static`, so they cannot be reached across the `.so`
// boundary.  These wrappers expose them under stable C names purely so the
// differential test-suite can drive the low-level entry points directly; with
// the feature disabled the `.so` exports exactly `memchra2`, matching the C
// library's public surface byte for byte.
// ---------------------------------------------------------------------------

#[cfg(feature = "internal_test_api")]
mod internal_test_api {
    use super::*;

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn itest_memchra(str_: *const c_char, c: c_int, n: usize) -> c_int {
        memchra(str_, c, n)
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn itest_process_buffer(buffer: *mut c_char, len: usize) -> c_int {
        process_buffer(buffer, len)
    }

    /// The `float` return value is handed back as raw bits so the comparison is
    /// exact (NaN payloads included).
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn itest_int_to_float_bits(value: c_int) -> f32 {
        int_to_float_bits(value)
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn itest_process_strings(
        strings: *mut *mut c_char,
        count: c_int,
        target: *const c_char,
    ) -> c_int {
        process_strings(strings, count, target)
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn itest_safe_sum_array(arr: *mut c_int, size: usize) -> c_int {
        safe_sum_array(arr, size)
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn itest_interpret_as_int(bytes: *mut c_uchar, len: usize) -> c_int {
        interpret_as_int(bytes, len)
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn itest_count_occurrences(text: *const c_char, ch: c_char) -> c_int {
        count_occurrences(text, ch)
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn itest_complex_iteration(data: *mut c_int, count: usize) -> c_int {
        complex_iteration(data, count)
    }

    /// Mirrors the `snprintf` call site of `memchra2` so that the `%d`
    /// formatting emulation can be compared against glibc directly.  Copies the
    /// formatted string plus its NUL, exactly like the C shim.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn itest_format_buffer(
        a: c_int,
        b: c_int,
        c: c_int,
        d: c_int,
        out: *mut c_char,
        outlen: usize,
    ) {
        let mut buffer = [0 as c_char; 64];
        snprintf_test_pattern(&mut buffer, a, b, c, d);
        let mut n = c_strlen(buffer.as_ptr()) + 1;
        if n > outlen {
            n = outlen;
        }
        let mut i: usize = 0;
        while i < n {
            *out.add(i) = buffer[i];
            i += 1;
        }
    }
}
