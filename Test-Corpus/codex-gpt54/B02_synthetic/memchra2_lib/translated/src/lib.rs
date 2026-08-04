use std::ffi::c_int;

fn memchra(bytes: &[u8], c: c_int, n: usize) -> c_int {
    let needle = c as u8;
    let mut count: c_int = 0;

    for &byte in bytes.iter().take(n) {
        if byte == needle {
            count += 1;
        }
    }

    count
}

fn process_buffer(buffer: &[u8], len: usize) -> c_int {
    if buffer.is_empty() || buffer[0] == b'\0' {
        return -1;
    }

    let mut result: c_int = 0;

    for &byte in buffer.iter().take(len) {
        if byte == b'\0' {
            break;
        }
        result += byte as i8 as c_int;
    }

    result
}

fn int_to_float_bits(value: c_int) -> f32 {
    f32::from_bits(value as u32)
}

fn process_strings(strings: &[Option<&'static [u8]>], count: c_int, target: &[u8]) -> c_int {
    if count <= 0 {
        return 0;
    }

    let mut matches: c_int = 0;
    let target_len = target.len();

    for maybe_bytes in strings.iter().take(count as usize) {
        let Some(bytes) = maybe_bytes else {
            continue;
        };

        if bytes.is_empty() || bytes[0] == b'\0' {
            continue;
        }

        if bytes.len() >= target_len && &bytes[..target_len] == target {
            matches += 1;
        }
    }

    matches
}

fn safe_sum_array(arr: &[c_int], size: usize) -> c_int {
    if size == 0 {
        return 0;
    }

    let mut sum: c_int = 0;

    for &value in arr.iter().take(size) {
        sum += value;
    }

    sum
}

fn interpret_as_int(bytes: &[u8], len: usize) -> c_int {
    if len < std::mem::size_of::<c_int>() {
        return 0;
    }

    let mut int_bytes = [0_u8; std::mem::size_of::<c_int>()];
    int_bytes.copy_from_slice(&bytes[..std::mem::size_of::<c_int>()]);
    c_int::from_ne_bytes(int_bytes)
}

fn count_occurrences(text: &[u8], ch: u8) -> c_int {
    if text.is_empty() || text[0] == b'\0' {
        return 0;
    }

    memchra(text, ch as c_int, text.len())
}

fn complex_iteration(data: &[c_int], count: usize) -> c_int {
    if count == 0 {
        return -1;
    }

    let mut result: c_int = 0;

    for &value in data.iter().take(count) {
        let u = value as u32;
        result ^= (u & 0xFF) as c_int;
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn memchra2(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut result: c_int = 0;

    let buffer = format!("test{}-{}-{}-{}", a, b, c, d).into_bytes();

    let dash_count = count_occurrences(&buffer, b'-');
    result += dash_count * 10;

    let values = [a, b, c, d];
    let sum = safe_sum_array(&values, 4);
    result += sum;

    let test_strings = [
        Some(&b"test1"[..]),
        Some(&b"test2"[..]),
        Some(&b"testing"[..]),
        Some(&b"other"[..]),
    ];

    let matches = process_strings(&test_strings, 4, b"test");
    result += matches * 5;

    let f = int_to_float_bits(a);
    if f > 0.0_f32 && f < 1000.0_f32 {
        result += f as c_int;
    }

    let buf_sum = process_buffer(&buffer, buffer.len());
    if buf_sum > 0 {
        result += buf_sum % 256;
    }

    let bytes = [
        (b & 0xFF) as u8,
        (c & 0xFF) as u8,
        (d & 0xFF) as u8,
        0_u8,
    ];

    let interpreted = interpret_as_int(&bytes, 4);
    result ^= interpreted;

    let complex_result = complex_iteration(&values, 4);
    result += complex_result;

    result
}
