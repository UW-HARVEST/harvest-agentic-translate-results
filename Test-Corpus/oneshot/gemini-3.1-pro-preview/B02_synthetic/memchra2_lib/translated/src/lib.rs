use std::os::raw::{c_int, c_char};
use std::io::Write;

fn memchra(s: &[u8], c: i32) -> i32 {
    let mut count = 0;
    for &byte in s {
        if byte as c_char == c as c_char {
            count += 1;
        }
    }
    count
}

fn process_buffer(buffer: &[u8]) -> i32 {
    if buffer.is_empty() || buffer[0] == 0 {
        return -1;
    }
    let mut result = 0;
    for &byte in buffer {
        if byte == 0 {
            break;
        }
        result += (byte as c_char) as i32;
    }
    result
}

fn int_to_float_bits(value: i32) -> f32 {
    f32::from_bits(value as u32)
}

fn process_strings(strings: &[*const c_char], target: &[u8]) -> i32 {
    let mut matches = 0;
    for &s in strings {
        if s.is_null() {
            continue;
        }
        let cstr = unsafe { std::ffi::CStr::from_ptr(s) };
        let bytes = cstr.to_bytes();
        if bytes.is_empty() {
            continue;
        }
        if bytes.starts_with(target) {
            matches += 1;
        }
    }
    matches
}

fn safe_sum_array(arr: &[i32]) -> i32 {
    arr.iter().sum()
}

fn interpret_as_int(bytes: &[u8]) -> i32 {
    if bytes.len() < std::mem::size_of::<i32>() {
        return 0;
    }
    let mut arr = [0u8; 4];
    arr.copy_from_slice(&bytes[..4]);
    i32::from_ne_bytes(arr)
}

fn count_occurrences(text: &[u8], ch: u8) -> i32 {
    if text.is_empty() || text[0] == 0 {
        return 0;
    }
    let len = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    memchra(&text[..len], ch as i32)
}

fn complex_iteration(data: &[i32]) -> i32 {
    if data.is_empty() {
        return -1;
    }
    let mut result = 0;
    for &val in data {
        let u = val as u32;
        result ^= (u & 0xFF) as i32;
    }
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn memchra2(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut result = 0;

    let mut buffer = [0u8; 64];
    let mut cursor = std::io::Cursor::new(&mut buffer[..63]);
    let _ = write!(cursor, "test{}-{}-{}-{}", a, b, c, d);

    let dash_count = count_occurrences(&buffer, b'-');
    result += dash_count * 10;

    let values = [a, b, c, d];
    let sum = safe_sum_array(&values);
    result += sum;

    let test1 = b"test1\0".as_ptr() as *const c_char;
    let test2 = b"test2\0".as_ptr() as *const c_char;
    let testing = b"testing\0".as_ptr() as *const c_char;
    let other = b"other\0".as_ptr() as *const c_char;
    let test_strings = [test1, test2, testing, other];

    let matches = process_strings(&test_strings, b"test");
    result += matches * 5;

    let f = int_to_float_bits(a);
    if f > 0.0 && f < 1000.0 {
        result += f as i32;
    }

    let buf_len = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
    let buf_sum = process_buffer(&buffer[..buf_len]);
    if buf_sum > 0 {
        result += buf_sum % 256;
    }

    let mut bytes_arr = [0u8; 4];
    bytes_arr[0] = (b & 0xFF) as u8;
    bytes_arr[1] = (c & 0xFF) as u8;
    bytes_arr[2] = (d & 0xFF) as u8;
    bytes_arr[3] = 0;

    let interpreted = interpret_as_int(&bytes_arr);
    result ^= interpreted;

    let complex_result = complex_iteration(&values);
    result += complex_result;

    result
}
