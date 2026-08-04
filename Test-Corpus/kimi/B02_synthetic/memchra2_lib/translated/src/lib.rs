use std::ffi::{c_char, c_int, c_float};
use std::os::raw::{c_int as raw_c_int, c_float as raw_c_float};
use std::slice;
use std::mem;

fn memchra(str: &[u8], c: u8) -> i32 {
    str.iter().filter(|&&b| b == c).count() as i32
}

fn process_buffer(buffer: &[u8]) -> i32 {
    if buffer.is_empty() || buffer[0] == 0 {
        return -1;
    }
    buffer
        .iter()
        .take_while(|&&b| b != 0)
        .map(|&b| b as i32)
        .sum()
}

fn int_to_float_bits(value: i32) -> f32 {
    f32::from_bits(value as u32)
}

fn process_strings(strings: &[&str], target: &str) -> i32 {
    strings
        .iter()
        .filter(|s| !s.is_empty() && s.starts_with(target))
        .count() as i32
}

fn safe_sum_array(arr: &[i32]) -> i32 {
    arr.iter().sum()
}

fn interpret_as_int(bytes: &[u8]) -> i32 {
    if bytes.len() < mem::size_of::<i32>() {
        return 0;
    }
    i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn count_occurrences(text: &str, ch: u8) -> i32 {
    if text.is_empty() {
        return 0;
    }
    memchra(text.as_bytes(), ch)
}

fn complex_iteration(data: &[i32]) -> i32 {
    if data.is_empty() {
        return -1;
    }
    data.iter().fold(0, |acc, &x| acc ^ ((x as u32 & 0xFF) as i32))
}

#[unsafe(no_mangle)]
pub extern "C" fn memchra2(a: raw_c_int, b: raw_c_int, c: raw_c_int, d: raw_c_int) -> raw_c_int {
    let mut result: i32 = 0;

    let buffer = format!("test{}-{}-{}-{}", a, b, c, d);
    let buffer_bytes = buffer.as_bytes();

    let dash_count = count_occurrences(&buffer, b'-');
    result += dash_count * 10;

    let values = [a, b, c, d];
    let sum = safe_sum_array(&values);
    result += sum;

    let test_strings = ["test1", "test2", "testing", "other"];
    let matches = process_strings(&test_strings, "test");
    result += matches * 5;

    let f = int_to_float_bits(a);
    if f > 0.0 && f < 1000.0 {
        result += f as i32;
    }

    let buf_sum = process_buffer(buffer_bytes);
    if buf_sum > 0 {
        result += buf_sum % 256;
    }

    let mut bytes: [u8; 4] = [0; 4];
    bytes[0] = (b & 0xFF) as u8;
    bytes[1] = (c & 0xFF) as u8;
    bytes[2] = (d & 0xFF) as u8;
    bytes[3] = 0;

    let interpreted = interpret_as_int(&bytes);
    result ^= interpreted;

    let complex_result = complex_iteration(&values);
    result += complex_result;

    result
}
