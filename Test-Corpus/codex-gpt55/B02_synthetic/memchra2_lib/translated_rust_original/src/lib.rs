use std::ffi::c_int;

fn memchra(bytes: &[u8], c: i32) -> c_int {
    let target = c as u8;
    bytes.iter().filter(|&&byte| byte == target).count() as c_int
}

fn process_buffer(buffer: &[u8], len: usize) -> c_int {
    if buffer.is_empty() || buffer[0] == 0 {
        return -1;
    }

    let mut result: c_int = 0;
    let limit = len.min(buffer.len());
    for &byte in &buffer[..limit] {
        if byte == 0 {
            break;
        }
        result = result.wrapping_add(byte as i8 as c_int);
    }
    result
}

fn int_to_float_bits(value: c_int) -> f32 {
    f32::from_bits(value as u32)
}

fn process_strings(strings: &[Option<&[u8]>], count: c_int, target: &[u8]) -> c_int {
    if strings.is_empty() || count <= 0 {
        return 0;
    }

    let mut matches: c_int = 0;
    let limit = (count as usize).min(strings.len());
    for string in &strings[..limit] {
        let Some(string) = string else {
            continue;
        };
        if string.is_empty() || string[0] == 0 {
            continue;
        }

        if string.len() >= target.len() && &string[..target.len()] == target {
            matches = matches.wrapping_add(1);
        }
    }

    matches
}

fn safe_sum_array(arr: &[c_int], size: usize) -> c_int {
    if arr.is_empty() || size == 0 {
        return 0;
    }

    let mut sum: c_int = 0;
    let limit = size.min(arr.len());
    for &value in &arr[..limit] {
        sum = sum.wrapping_add(value);
    }

    sum
}

fn interpret_as_int(bytes: &[u8], len: usize) -> c_int {
    if bytes.len() < 4 || len < size_of::<c_int>() {
        return 0;
    }

    c_int::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn count_occurrences(text: &[u8], ch: u8) -> c_int {
    if text.is_empty() || text[0] == 0 {
        return 0;
    }

    let len = text.iter().position(|&byte| byte == 0).unwrap_or(text.len());
    memchra(&text[..len], ch as i32)
}

fn complex_iteration(data: &[c_int], count: usize) -> c_int {
    if data.is_empty() || count == 0 {
        return -1;
    }

    let mut result: c_int = 0;
    let limit = count.min(data.len());
    for &value in &data[..limit] {
        let u = value as u32;
        result ^= (u & 0xff) as c_int;
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn memchra2(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut result: c_int = 0;

    let buffer = format!("test{}-{}-{}-{}", a, b, c, d);

    let dash_count = count_occurrences(buffer.as_bytes(), b'-');
    result = result.wrapping_add(dash_count.wrapping_mul(10));

    let values = [a, b, c, d];
    let sum = safe_sum_array(&values, 4);
    result = result.wrapping_add(sum);

    let test_strings: [Option<&[u8]>; 4] = [
        Some(b"test1"),
        Some(b"test2"),
        Some(b"testing"),
        Some(b"other"),
    ];

    let matches = process_strings(&test_strings, 4, b"test");
    result = result.wrapping_add(matches.wrapping_mul(5));

    let f = int_to_float_bits(a);
    if f > 0.0 && f < 1000.0 {
        result = result.wrapping_add(f as c_int);
    }

    let buf_sum = process_buffer(buffer.as_bytes(), buffer.len());
    if buf_sum > 0 {
        result = result.wrapping_add(buf_sum % 256);
    }

    let bytes = [
        (b & 0xff) as u8,
        (c & 0xff) as u8,
        (d & 0xff) as u8,
        0,
    ];

    let interpreted = interpret_as_int(&bytes, 4);
    result ^= interpreted;

    let complex_result = complex_iteration(&values, 4);
    result = result.wrapping_add(complex_result);

    result
}
