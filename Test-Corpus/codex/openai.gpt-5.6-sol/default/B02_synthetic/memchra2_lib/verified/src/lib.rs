use std::ffi::c_int;
use std::fmt::Write;

fn memchra(bytes: &[u8], value: u8) -> c_int {
    bytes
        .iter()
        .fold(0, |count, &byte| count + c_int::from(byte == value))
}

fn process_buffer(buffer: &[u8]) -> c_int {
    if buffer.is_empty() || buffer[0] == 0 {
        return -1;
    }

    buffer
        .iter()
        .take_while(|&&byte| byte != 0)
        .fold(0, |result, &byte| result.wrapping_add(c_int::from(byte)))
}

fn int_to_float_bits(value: c_int) -> f32 {
    f32::from_bits(value as u32)
}

fn process_strings(strings: &[&[u8]], target: &[u8]) -> c_int {
    if strings.is_empty() {
        return 0;
    }

    strings
        .iter()
        .filter(|string| !string.is_empty() && string.starts_with(target))
        .count() as c_int
}

fn safe_sum_array(values: &[c_int]) -> c_int {
    values
        .iter()
        .fold(0, |sum, &value| sum.wrapping_add(value))
}

fn interpret_as_int(bytes: &[u8]) -> c_int {
    if bytes.len() < size_of::<c_int>() {
        return 0;
    }

    c_int::from_ne_bytes(bytes[..size_of::<c_int>()].try_into().unwrap())
}

fn count_occurrences(text: &[u8], value: u8) -> c_int {
    if text.is_empty() || text[0] == 0 {
        return 0;
    }

    let length = text.iter().position(|&byte| byte == 0).unwrap_or(text.len());
    memchra(&text[..length], value)
}

fn complex_iteration(values: &[c_int]) -> c_int {
    if values.is_empty() {
        return -1;
    }

    values
        .iter()
        .fold(0, |result, &value| result ^ (value as u32 & 0xff) as c_int)
}

#[unsafe(no_mangle)]
pub extern "C" fn memchra2(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut buffer = String::with_capacity(63);
    write!(&mut buffer, "test{a}-{b}-{c}-{d}").unwrap();
    let buffer = buffer.as_bytes();

    let mut result = count_occurrences(buffer, b'-').wrapping_mul(10);

    let values = [a, b, c, d];
    let sum = safe_sum_array(&values);
    result = result.wrapping_add(sum);

    let test_strings: [&[u8]; 4] = [b"test1", b"test2", b"testing", b"other"];
    let matches = process_strings(&test_strings, b"test");
    result = result.wrapping_add(matches.wrapping_mul(5));

    let float_value = int_to_float_bits(a);
    if float_value > 0.0 && float_value < 1000.0 {
        result = result.wrapping_add(float_value as c_int);
    }

    let buffer_sum = process_buffer(buffer);
    if buffer_sum > 0 {
        result = result.wrapping_add(buffer_sum % 256);
    }

    let bytes = [
        (b as u32 & 0xff) as u8,
        (c as u32 & 0xff) as u8,
        (d as u32 & 0xff) as u8,
        0,
    ];
    let interpreted = interpret_as_int(&bytes);
    result ^= interpreted;

    let complex_result = complex_iteration(&values);
    result.wrapping_add(complex_result)
}
