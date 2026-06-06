// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust.

use std::ffi::c_int;

fn memchra(s: &[u8], c: c_int) -> c_int {
    // Mirrors: str[i] == (char)c. After integer promotion, both sides become int.
    // (char)c truncates and (potentially) sign-extends. Comparing equality is
    // equivalent to comparing the low 8 bits, i.e., comparing as u8.
    let target: u8 = c as u8;
    let mut count: c_int = 0;
    for &byte in s {
        if byte == target {
            count = count.wrapping_add(1);
        }
    }
    count
}

fn process_buffer(buffer: &[u8]) -> c_int {
    if buffer.is_empty() || buffer[0] == 0 {
        return -1;
    }
    let mut result: c_int = 0;
    for &ch in buffer {
        if ch == 0 {
            break;
        }
        // (int)(*i) where *i is `char`. On most platforms `char` is signed.
        // Sign extend through i8 to c_int.
        result = result.wrapping_add(ch as i8 as c_int);
    }
    result
}

fn int_to_float_bits(value: c_int) -> f32 {
    // Union-based reinterpretation of the int's bits as a float.
    f32::from_bits(value as u32)
}

fn process_strings(strings: &[&[u8]], target: &[u8]) -> c_int {
    if strings.is_empty() {
        return 0;
    }
    let mut matches: c_int = 0;
    for s in strings {
        if s.is_empty() || s[0] == 0 {
            continue;
        }
        // strncmp(*i, target, strlen(target)) == 0  =>  *i starts with target.
        if s.starts_with(target) {
            matches = matches.wrapping_add(1);
        }
    }
    matches
}

fn safe_sum_array(arr: &[c_int]) -> c_int {
    if arr.is_empty() {
        return 0;
    }
    let mut sum: c_int = 0;
    for &v in arr {
        sum = sum.wrapping_add(v);
    }
    sum
}

fn interpret_as_int(bytes: &[u8]) -> c_int {
    // *(int*)bytes - little-endian read on x86/x86_64.
    if bytes.len() < std::mem::size_of::<c_int>() {
        return 0;
    }
    c_int::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn count_occurrences(text: &[u8], ch: u8) -> c_int {
    if text.is_empty() || text[0] == 0 {
        return 0;
    }
    let len = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    memchra(&text[..len], ch as c_int)
}

fn complex_iteration(data: &[c_int]) -> c_int {
    if data.is_empty() {
        return -1;
    }
    let mut result: c_int = 0;
    for &v in data {
        let u = v as u32;
        result ^= (u & 0xFF) as c_int;
    }
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn memchra2(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut result: c_int = 0;

    // snprintf(buffer, sizeof(buffer), "test%d-%d-%d-%d", a, b, c, d) with size 64.
    let formatted = format!("test{}-{}-{}-{}", a, b, c, d);
    let formatted_bytes = formatted.as_bytes();
    let mut buffer = [0u8; 64];
    let copy_len = core::cmp::min(formatted_bytes.len(), 63);
    buffer[..copy_len].copy_from_slice(&formatted_bytes[..copy_len]);
    // buffer[copy_len] is already 0 (null-terminator) due to zero init.

    let dash_count = count_occurrences(&buffer, b'-');
    result = result.wrapping_add(dash_count.wrapping_mul(10));

    let values = [a, b, c, d];
    let sum = safe_sum_array(&values);
    result = result.wrapping_add(sum);

    let test_strings: [&[u8]; 4] = [
        b"test1",
        b"test2",
        b"testing",
        b"other",
    ];

    let matches = process_strings(&test_strings, b"test");
    result = result.wrapping_add(matches.wrapping_mul(5));

    let f = int_to_float_bits(a);
    if f > 0.0f32 && f < 1000.0f32 {
        result = result.wrapping_add(f as c_int);
    }

    // process_buffer(buffer, strlen(buffer)) — strlen excludes the null terminator.
    let strlen = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
    let buf_sum = process_buffer(&buffer[..strlen]);
    if buf_sum > 0 {
        result = result.wrapping_add(buf_sum % 256);
    }

    let mut bytes_arr = [0u8; 4];
    bytes_arr[0] = (b & 0xFF) as u8;
    bytes_arr[1] = (c & 0xFF) as u8;
    bytes_arr[2] = (d & 0xFF) as u8;
    bytes_arr[3] = 0;

    let interpreted = interpret_as_int(&bytes_arr);
    result ^= interpreted;

    let complex_result = complex_iteration(&values);
    result = result.wrapping_add(complex_result);

    result
}
