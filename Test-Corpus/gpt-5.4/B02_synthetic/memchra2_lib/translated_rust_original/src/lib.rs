use std::ffi::c_int;
use std::mem;
use std::slice;

fn memchra(bytes: &[u8], c: u8, n: usize) -> c_int {
    bytes.iter().take(n).filter(|&&b| b == c).count() as c_int
}

fn process_buffer(buffer: &[u8], len: usize) -> c_int {
    if buffer.is_empty() || buffer[0] == 0 {
        return -1;
    }

    let mut result: c_int = 0;
    for &b in buffer.iter().take(len) {
        if b == 0 {
            break;
        }
        result += b as c_int;
    }
    result
}

fn int_to_float_bits(value: c_int) -> f32 {
    f32::from_bits(value as u32)
}

fn process_strings(strings: &[&str], count: c_int, target: &str) -> c_int {
    if count <= 0 {
        return 0;
    }

    let limit = (count as usize).min(strings.len());
    let mut matches: c_int = 0;

    for s in strings.iter().take(limit) {
        if s.is_empty() {
            continue;
        }
        if s.starts_with(target) {
            matches += 1;
        }
    }

    matches
}

fn safe_sum_array(arr: &[c_int], size: usize) -> c_int {
    if size == 0 || arr.is_empty() {
        return 0;
    }

    arr.iter().take(size).copied().sum()
}

fn interpret_as_int(bytes: &[u8], len: usize) -> c_int {
    if len < mem::size_of::<c_int>() || bytes.len() < mem::size_of::<c_int>() {
        return 0;
    }

    let arr: [u8; mem::size_of::<c_int>()] = bytes[..mem::size_of::<c_int>()].try_into().unwrap();
    c_int::from_ne_bytes(arr)
}

fn count_occurrences(text: &[u8], ch: u8) -> c_int {
    if text.is_empty() || text[0] == 0 {
        return 0;
    }

    let len = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    memchra(text, ch, len)
}

fn complex_iteration(data: &[c_int], count: usize) -> c_int {
    if data.is_empty() || count == 0 {
        return -1;
    }

    let mut result: c_int = 0;
    for &v in data.iter().take(count) {
        let u = v as u32;
        result ^= (u & 0xFF) as c_int;
    }
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn memchra2(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut result: c_int = 0;

    let buffer_string = format!("test{}-{}-{}-{}", a, b, c, d);
    let mut buffer = Vec::with_capacity(64);
    buffer.extend_from_slice(buffer_string.as_bytes());
    if buffer.len() >= 64 {
        buffer.truncate(63);
    }
    buffer.push(0);

    let dash_count = count_occurrences(&buffer, b'-');
    result += dash_count * 10;

    let values = [a, b, c, d];
    let sum = safe_sum_array(&values, 4);
    result += sum;

    let test_strings = ["test1", "test2", "testing", "other"];
    let matches = process_strings(&test_strings, 4, "test");
    result += matches * 5;

    let f = int_to_float_bits(a);
    if f > 0.0 && f < 1000.0 {
        result += f as c_int;
    }

    let buf_len = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
    let buf_sum = process_buffer(&buffer, buf_len);
    if buf_sum > 0 {
        result += buf_sum % 256;
    }

    let bytes = [
        (b & 0xFF) as u8,
        (c & 0xFF) as u8,
        (d & 0xFF) as u8,
        0,
    ];

    let interpreted = interpret_as_int(&bytes, 4);
    result ^= interpreted;

    let complex_result = complex_iteration(&values, 4);
    result += complex_result;

    result
}
