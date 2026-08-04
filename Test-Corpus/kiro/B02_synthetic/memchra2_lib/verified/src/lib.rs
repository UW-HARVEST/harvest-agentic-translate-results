use std::ffi::c_int;

fn memchra(str_: &[u8], c: c_int, n: usize) -> c_int {
    let mut count: c_int = 0;
    for i in 0..n {
        if str_[i] == c as u8 {
            count += 1;
        }
    }
    count
}

fn process_buffer(buffer: &[u8], len: usize) -> c_int {
    if buffer.is_empty() || buffer[0] == 0 {
        return -1;
    }
    let mut result: c_int = 0;
    for i in 0..len {
        if buffer[i] == 0 {
            break;
        }
        result = result.wrapping_add(buffer[i] as c_int);
    }
    result
}

fn int_to_float_bits(value: c_int) -> f32 {
    f32::from_bits(value as u32)
}

fn process_strings(strings: &[&[u8]], count: c_int, target: &[u8]) -> c_int {
    if count <= 0 {
        return 0;
    }
    let mut matches: c_int = 0;
    let target_len = target.len();
    for i in 0..count as usize {
        let s = strings[i];
        if s.is_empty() || s[0] == 0 {
            continue;
        }
        if s.len() >= target_len && &s[..target_len] == target {
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
    for i in 0..size {
        sum = sum.wrapping_add(arr[i]);
    }
    sum
}

fn interpret_as_int(bytes: &[u8; 4]) -> c_int {
    c_int::from_ne_bytes(*bytes)
}

fn count_occurrences(text: &[u8]) -> c_int {
    let len = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    if len == 0 {
        return 0;
    }
    memchra(text, b'-' as c_int, len)
}

fn complex_iteration(data: &[c_int], count: usize) -> c_int {
    if count == 0 {
        return -1;
    }
    let mut result: c_int = 0;
    for i in 0..count {
        let u = data[i] as u32;
        result ^= (u & 0xFF) as c_int;
    }
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn memchra2(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut result: c_int = 0;

    // snprintf(buffer, 64, "test%d-%d-%d-%d", a, b, c, d)
    let formatted = format!("test{}-{}-{}-{}", a, b, c, d);
    let mut buffer = [0u8; 64];
    let copy_len = formatted.len().min(63);
    buffer[..copy_len].copy_from_slice(&formatted.as_bytes()[..copy_len]);
    buffer[copy_len] = 0;

    let dash_count = count_occurrences(&buffer);
    result = result.wrapping_add(dash_count.wrapping_mul(10));

    let values = [a, b, c, d];
    let sum = safe_sum_array(&values, 4);
    result = result.wrapping_add(sum);

    let test_strings: &[&[u8]] = &[b"test1", b"test2", b"testing", b"other"];
    let matches = process_strings(test_strings, 4, b"test");
    result = result.wrapping_add(matches.wrapping_mul(5));

    let f = int_to_float_bits(a);
    if f > 0.0f32 && f < 1000.0f32 {
        result = result.wrapping_add(f as c_int);
    }

    let buf_len = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
    let buf_sum = process_buffer(&buffer, buf_len);
    if buf_sum > 0 {
        result = result.wrapping_add(buf_sum % 256);
    }

    let mut bytes = [0u8; 4];
    bytes[0] = (b & 0xFF) as u8;
    bytes[1] = (c & 0xFF) as u8;
    bytes[2] = (d & 0xFF) as u8;
    bytes[3] = 0;

    let interpreted = interpret_as_int(&bytes);
    result ^= interpreted;

    let complex_result = complex_iteration(&values, 4);
    result = result.wrapping_add(complex_result);

    result
}
