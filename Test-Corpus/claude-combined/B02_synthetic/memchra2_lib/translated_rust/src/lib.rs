// Rust translation of c_src/src/lib.c
//
// Preserves the exact behavior of the C reference (including any quirks),
// matching the public symbol `memchra2` exposed by the C shared library.

use std::ffi::c_char;
use std::ffi::c_int;
use std::os::raw::c_uchar;

/// Count occurrences of byte `(char)c` in `str[0..n]`.
///
/// Mirrors the C `memchra` helper. Note: in C, comparing
/// `str[i] == (char)c` uses signed-char semantics on Linux/x86_64.
fn memchra(s: &[u8], c: c_int, n: usize) -> c_int {
    let target = c as u8; // truncation matches `(char)c`
    let mut count: c_int = 0;
    for i in 0..n {
        if s[i] == target {
            count += 1;
        }
    }
    count
}

/// Sum the bytes of `buffer` interpreted as `signed char` (Linux default),
/// stopping at the first NUL byte or after `len` bytes (whichever comes
/// first). Returns -1 if the input pointer is null or starts with NUL.
unsafe fn process_buffer(buffer: *mut c_char, len: usize) -> c_int {
    if buffer.is_null() || unsafe { *buffer } == 0 {
        return -1;
    }
    let mut result: c_int = 0;
    let mut i: usize = 0;
    while i < len {
        let ch = unsafe { *buffer.add(i) };
        if ch == 0 {
            break;
        }
        // C signed-char semantics: cast to int preserves sign.
        result += ch as c_int;
        i += 1;
    }
    result
}

/// Bit-reinterpret an `int` as `float`. Matches the C union punning trick.
fn int_to_float_bits(value: c_int) -> f32 {
    f32::from_bits(value as u32)
}

/// Count how many of the `count` strings start with `target`.
/// Returns 0 on null array or `count <= 0`. Skips null/empty entries.
unsafe fn process_strings(
    strings: *const *const c_char,
    count: c_int,
    target: *const c_char,
) -> c_int {
    if strings.is_null() || count <= 0 {
        return 0;
    }

    // strlen(target)
    let target_len = unsafe {
        let mut n: usize = 0;
        while *target.add(n) != 0 {
            n += 1;
        }
        n
    };

    let mut matches: c_int = 0;
    for idx in 0..count as usize {
        let s = unsafe { *strings.add(idx) };
        if s.is_null() || unsafe { *s } == 0 {
            continue;
        }
        // strncmp behaviour: compare up to `target_len` bytes; equal if all
        // bytes match (including a possible NUL inside both strings).
        let mut equal = true;
        for j in 0..target_len {
            let a = unsafe { *s.add(j) } as u8;
            let b = unsafe { *target.add(j) } as u8;
            if a != b {
                equal = false;
                break;
            }
            if a == 0 {
                // both reached NUL simultaneously -> equal
                break;
            }
        }
        if equal {
            matches += 1;
        }
    }
    matches
}

/// Sum the elements of an `int` array. Returns 0 on null pointer / zero size.
unsafe fn safe_sum_array(arr: *const c_int, size: usize) -> c_int {
    if arr.is_null() || size == 0 {
        return 0;
    }
    let mut sum: c_int = 0;
    for i in 0..size {
        // Wrapping addition matches C signed overflow as compiled with the
        // typical 2's-complement semantics used here.
        sum = sum.wrapping_add(unsafe { *arr.add(i) });
    }
    sum
}

/// Reinterpret the first `sizeof(int)` bytes of `bytes` as a host-endian
/// `int`. Returns 0 if `bytes` is null or `len < sizeof(int)`.
unsafe fn interpret_as_int(bytes: *const c_uchar, len: usize) -> c_int {
    if bytes.is_null() || len < std::mem::size_of::<c_int>() {
        return 0;
    }
    let mut buf = [0u8; 4];
    for i in 0..4 {
        buf[i] = unsafe { *bytes.add(i) };
    }
    // Native-endian to match the C `*(int*)bytes` read.
    c_int::from_ne_bytes(buf)
}

/// Count occurrences of `ch` in the C string `text`.
unsafe fn count_occurrences(text: *const c_char, ch: c_char) -> c_int {
    if text.is_null() || unsafe { *text } == 0 {
        return 0;
    }
    let mut len: usize = 0;
    while unsafe { *text.add(len) } != 0 {
        len += 1;
    }
    let slice = unsafe { std::slice::from_raw_parts(text as *const u8, len) };
    memchra(slice, ch as c_int, len)
}

/// XOR-fold the low byte of each `int` in `data`.
unsafe fn complex_iteration(data: *const c_int, count: usize) -> c_int {
    if data.is_null() || count == 0 {
        return -1;
    }
    let mut result: c_int = 0;
    for i in 0..count {
        let v = unsafe { *data.add(i) };
        let u = v as u32;
        result ^= (u & 0xFF) as c_int;
    }
    result
}

/// Public C-compatible entry point. Behavior must match `memchra2` in lib.c
/// byte-for-byte.
#[unsafe(no_mangle)]
pub extern "C" fn memchra2(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut result: c_int = 0;

    // snprintf(buffer, 64, "test%d-%d-%d-%d", a, b, c, d)
    let formatted = format!("test{}-{}-{}-{}", a, b, c, d);
    let mut buffer = [0u8; 64];
    let bytes = formatted.as_bytes();
    let copy_len = bytes.len().min(63); // leave room for trailing NUL
    buffer[..copy_len].copy_from_slice(&bytes[..copy_len]);
    // remaining bytes already zero-initialised, NUL-terminating the string

    let buffer_ptr = buffer.as_mut_ptr() as *mut c_char;

    let dash_count = unsafe { count_occurrences(buffer_ptr, b'-' as c_char) };
    result = result.wrapping_add(dash_count.wrapping_mul(10));

    let values: [c_int; 4] = [a, b, c, d];
    let sum = unsafe { safe_sum_array(values.as_ptr(), 4) };
    result = result.wrapping_add(sum);

    // Static C strings used for process_strings.
    let s_test1 = b"test1\0";
    let s_test2 = b"test2\0";
    let s_testing = b"testing\0";
    let s_other = b"other\0";
    let s_target = b"test\0";

    let test_strings: [*const c_char; 4] = [
        s_test1.as_ptr() as *const c_char,
        s_test2.as_ptr() as *const c_char,
        s_testing.as_ptr() as *const c_char,
        s_other.as_ptr() as *const c_char,
    ];

    let matches = unsafe {
        process_strings(
            test_strings.as_ptr(),
            4,
            s_target.as_ptr() as *const c_char,
        )
    };
    result = result.wrapping_add(matches.wrapping_mul(5));

    let f = int_to_float_bits(a);
    if f > 0.0f32 && f < 1000.0f32 {
        // C cast: float -> int truncates toward zero.
        result = result.wrapping_add(f as c_int);
    }

    // process_buffer(buffer, strlen(buffer))
    let buf_strlen = {
        let mut n: usize = 0;
        while n < buffer.len() && buffer[n] != 0 {
            n += 1;
        }
        n
    };
    let buf_sum = unsafe { process_buffer(buffer_ptr, buf_strlen) };
    if buf_sum > 0 {
        result = result.wrapping_add(buf_sum % 256);
    }

    // unsigned char bytes[4]; bytes[0..3] from b,c,d; bytes[3] = 0
    let mut bytes_arr: [c_uchar; 4] = [0; 4];
    bytes_arr[0] = (b & 0xFF) as c_uchar;
    bytes_arr[1] = (c & 0xFF) as c_uchar;
    bytes_arr[2] = (d & 0xFF) as c_uchar;
    bytes_arr[3] = 0;

    let interpreted = unsafe { interpret_as_int(bytes_arr.as_ptr(), 4) };
    result ^= interpreted;

    let complex_result = unsafe { complex_iteration(values.as_ptr(), 4) };
    result = result.wrapping_add(complex_result);

    result
}
