use std::ffi::c_int;
use std::fmt::{self, Write};

struct StackBuffer {
    bytes: [u8; 64],
    len: usize,
}

impl StackBuffer {
    fn new() -> Self {
        Self {
            bytes: [0; 64],
            len: 0,
        }
    }

    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

impl Write for StackBuffer {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let end = self.len.checked_add(value.len()).ok_or(fmt::Error)?;
        let destination = self.bytes.get_mut(self.len..end).ok_or(fmt::Error)?;
        destination.copy_from_slice(value.as_bytes());
        self.len = end;
        Ok(())
    }
}

fn memchra(bytes: &[u8], byte: u8) -> c_int {
    bytes
        .iter()
        .fold(0_i32, |count, &item| count + i32::from(item == byte))
}

fn process_buffer(buffer: &[u8]) -> c_int {
    if buffer.first().is_none_or(|&byte| byte == 0) {
        return -1;
    }

    buffer
        .iter()
        .take_while(|&&byte| byte != 0)
        .fold(0_i32, |sum, &byte| sum.wrapping_add(byte as i8 as i32))
}

fn int_to_float_bits(value: c_int) -> f32 {
    f32::from_bits(value as u32)
}

fn process_strings(strings: &[&[u8]], target: &[u8]) -> c_int {
    strings.iter().fold(0_i32, |matches, string| {
        matches + i32::from(!string.is_empty() && string.starts_with(target))
    })
}

fn safe_sum_array(values: &[c_int]) -> c_int {
    values
        .iter()
        .fold(0_i32, |sum, &value| sum.wrapping_add(value))
}

fn interpret_as_int(bytes: [u8; 4]) -> c_int {
    i32::from_ne_bytes(bytes)
}

fn count_occurrences(text: &[u8], byte: u8) -> c_int {
    if text.first().is_none_or(|&item| item == 0) {
        0
    } else {
        memchra(text, byte)
    }
}

fn complex_iteration(values: &[c_int]) -> c_int {
    if values.is_empty() {
        return -1;
    }

    values.iter().fold(0_i32, |result, &value| {
        result ^ (value as u32 & 0xff) as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn memchra2(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut result = 0_i32;

    let mut buffer = StackBuffer::new();
    write!(&mut buffer, "test{a}-{b}-{c}-{d}")
        .expect("all four decimal integers fit in the 64-byte C buffer");
    let buffer = buffer.as_bytes();

    let dash_count = count_occurrences(buffer, b'-');
    result = result.wrapping_add(dash_count.wrapping_mul(10));

    let values = [a, b, c, d];
    let sum = safe_sum_array(&values);
    result = result.wrapping_add(sum);

    let test_strings: [&[u8]; 4] = [b"test1", b"test2", b"testing", b"other"];
    let matches = process_strings(&test_strings, b"test");
    result = result.wrapping_add(matches.wrapping_mul(5));

    let float = int_to_float_bits(a);
    if float > 0.0 && float < 1000.0 {
        result = result.wrapping_add(float as c_int);
    }

    let buffer_sum = process_buffer(buffer);
    if buffer_sum > 0 {
        result = result.wrapping_add(buffer_sum % 256);
    }

    let bytes = [b as u8, c as u8, d as u8, 0];
    let interpreted = interpret_as_int(bytes);
    result ^= interpreted;

    let complex_result = complex_iteration(&values);
    result.wrapping_add(complex_result)
}
