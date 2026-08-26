use std::ffi::{c_char, c_int, c_uchar};
use std::fmt::{self, Write};
use std::ptr;

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

fn memchra_impl(bytes: &[u8], byte: u8) -> c_int {
    bytes
        .iter()
        .fold(0_i32, |count, &item| count + i32::from(item == byte))
}

fn process_buffer_impl(buffer: &[u8]) -> c_int {
    if buffer.first().is_none_or(|&byte| byte == 0) {
        return -1;
    }

    buffer
        .iter()
        .take_while(|&&byte| byte != 0)
        .fold(0_i32, |sum, &byte| sum.wrapping_add(byte as i8 as i32))
}

fn int_to_float_bits_impl(value: c_int) -> f32 {
    f32::from_bits(value as u32)
}

fn process_strings_impl(strings: &[&[u8]], target: &[u8]) -> c_int {
    strings.iter().fold(0_i32, |matches, string| {
        matches + i32::from(!string.is_empty() && string.starts_with(target))
    })
}

fn safe_sum_array_impl(values: &[c_int]) -> c_int {
    values
        .iter()
        .fold(0_i32, |sum, &value| sum.wrapping_add(value))
}

fn interpret_as_int_impl(bytes: [u8; 4]) -> c_int {
    i32::from_ne_bytes(bytes)
}

fn count_occurrences_impl(text: &[u8], byte: u8) -> c_int {
    if text.first().is_none_or(|&item| item == 0) {
        0
    } else {
        memchra_impl(text, byte)
    }
}

fn complex_iteration_impl(values: &[c_int]) -> c_int {
    if values.is_empty() {
        return -1;
    }

    values.iter().fold(0_i32, |result, &value| {
        result ^ (value as u32 & 0xff) as i32
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memchra(str: *const c_char, c: c_int, n: usize) -> c_int {
    let mut count = 0_i32;
    for index in 0..n {
        let item = unsafe { *str.add(index) };
        count = count.wrapping_add(i32::from(item == c as c_char));
    }
    count
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_buffer(buffer: *mut c_char, len: usize) -> c_int {
    if buffer.is_null() || unsafe { *buffer } == 0 {
        return -1;
    }

    let mut result = 0_i32;
    for index in 0..len {
        let item = unsafe { *buffer.add(index) };
        if item == 0 {
            break;
        }
        result = result.wrapping_add(item as c_int);
    }
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn int_to_float_bits(value: c_int) -> f32 {
    int_to_float_bits_impl(value)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_strings(
    strings: *mut *mut c_char,
    count: c_int,
    target: *const c_char,
) -> c_int {
    if strings.is_null() || count <= 0 {
        return 0;
    }

    let target_len = unsafe { libc_strlen(target) };
    let mut matches = 0_i32;
    for index in 0..count as usize {
        let string = unsafe { *strings.add(index) };
        if string.is_null() || unsafe { *string } == 0 {
            continue;
        }

        let mut equal = true;
        for offset in 0..target_len {
            if unsafe { *string.add(offset) != *target.add(offset) } {
                equal = false;
                break;
            }
        }
        matches = matches.wrapping_add(i32::from(equal));
    }
    matches
}

unsafe fn libc_strlen(value: *const c_char) -> usize {
    let mut len = 0;
    while unsafe { *value.add(len) } != 0 {
        len += 1;
    }
    len
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn safe_sum_array(arr: *mut c_int, size: usize) -> c_int {
    if arr.is_null() || size == 0 {
        return 0;
    }

    let mut sum = 0_i32;
    for index in 0..size {
        sum = sum.wrapping_add(unsafe { *arr.add(index) });
    }
    sum
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn interpret_as_int(bytes: *mut c_uchar, len: usize) -> c_int {
    if bytes.is_null() || len < size_of::<c_int>() {
        return 0;
    }

    unsafe { ptr::read_unaligned(bytes.cast::<c_int>()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn count_occurrences(text: *const c_char, ch: c_char) -> c_int {
    if text.is_null() || unsafe { *text } == 0 {
        return 0;
    }

    let len = unsafe { libc_strlen(text) };
    unsafe { memchra(text, ch as c_int, len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn complex_iteration(data: *mut c_int, count: usize) -> c_int {
    if data.is_null() || count == 0 {
        return -1;
    }

    let mut result = 0_i32;
    for index in 0..count {
        let value = unsafe { *data.add(index) };
        result ^= (value as u32 & 0xff) as i32;
    }
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn memchra2(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut result = 0_i32;

    let mut buffer = StackBuffer::new();
    write!(&mut buffer, "test{a}-{b}-{c}-{d}")
        .expect("all four decimal integers fit in the 64-byte C buffer");
    let buffer = buffer.as_bytes();

    let dash_count = count_occurrences_impl(buffer, b'-');
    result = result.wrapping_add(dash_count.wrapping_mul(10));

    let values = [a, b, c, d];
    let sum = safe_sum_array_impl(&values);
    result = result.wrapping_add(sum);

    let test_strings: [&[u8]; 4] = [b"test1", b"test2", b"testing", b"other"];
    let matches = process_strings_impl(&test_strings, b"test");
    result = result.wrapping_add(matches.wrapping_mul(5));

    let float = int_to_float_bits_impl(a);
    if float > 0.0 && float < 1000.0 {
        result = result.wrapping_add(float as c_int);
    }

    let buffer_sum = process_buffer_impl(buffer);
    if buffer_sum > 0 {
        result = result.wrapping_add(buffer_sum % 256);
    }

    let bytes = [b as u8, c as u8, d as u8, 0];
    let interpreted = interpret_as_int_impl(bytes);
    result ^= interpreted;

    let complex_result = complex_iteration_impl(&values);
    result.wrapping_add(complex_result)
}
