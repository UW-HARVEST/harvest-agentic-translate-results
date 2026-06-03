use std::ffi::c_int;

fn memchra(s: &[u8], c: i32) -> i32 {
    let target = c as u8;
    let mut count: i32 = 0;
    for &b in s {
        if b == target {
            count = count.wrapping_add(1);
        }
    }
    count
}

fn process_buffer(buffer: &[u8]) -> i32 {
    if buffer.is_empty() || buffer[0] == 0 {
        return -1;
    }
    let mut result: i32 = 0;
    for &b in buffer {
        if b == 0 {
            break;
        }
        // In C, plain `char` is signed on x86_64 Linux; (int)(*i) sign-extends.
        result = result.wrapping_add(b as i8 as i32);
    }
    result
}

fn int_to_float_bits(value: i32) -> f32 {
    // Reinterpret the int's bits as a float (matches the C union trick).
    f32::from_bits(value as u32)
}

fn process_strings(strings: &[&[u8]], target: &[u8]) -> i32 {
    if strings.is_empty() {
        return 0;
    }
    let mut matches: i32 = 0;
    let target_len = target.len();
    for s in strings {
        if s.is_empty() {
            continue;
        }
        if s.len() >= target_len && &s[..target_len] == target {
            matches = matches.wrapping_add(1);
        }
    }
    matches
}

fn safe_sum_array(arr: &[i32]) -> i32 {
    if arr.is_empty() {
        return 0;
    }
    let mut sum: i32 = 0;
    for &v in arr {
        sum = sum.wrapping_add(v);
    }
    sum
}

fn interpret_as_int(bytes: &[u8]) -> i32 {
    if bytes.len() < std::mem::size_of::<i32>() {
        return 0;
    }
    // C does `*(int *)bytes` which reads 4 bytes in native byte order.
    // Target platform is little-endian (x86_64 Linux).
    i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
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
    let mut result: i32 = 0;
    for &v in data {
        let u = v as u32;
        result ^= (u & 0xFF) as i32;
    }
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn memchra2(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut result: i32 = 0;

    // snprintf(buffer, sizeof(buffer), "test%d-%d-%d-%d", a, b, c, d);
    // Buffer is 64 bytes; snprintf truncates to fit and always NUL-terminates.
    let formatted = format!("test{}-{}-{}-{}", a, b, c, d);
    let mut buffer = [0u8; 64];
    let src = formatted.as_bytes();
    let n = src.len().min(buffer.len() - 1);
    buffer[..n].copy_from_slice(&src[..n]);
    // buffer[n] is already 0

    let dash_count = count_occurrences(&buffer, b'-');
    result = result.wrapping_add(dash_count.wrapping_mul(10));

    let values: [i32; 4] = [a, b, c, d];
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
    if f > 0.0_f32 && f < 1000.0_f32 {
        // Float is guaranteed in (0, 1000) here, so `as i32` matches C's
        // truncation-toward-zero conversion.
        result = result.wrapping_add(f as i32);
    }

    // strlen(buffer) — length up to first NUL
    let buf_len = buffer.iter().position(|&x| x == 0).unwrap_or(buffer.len());
    let buf_sum = process_buffer(&buffer[..buf_len]);
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

    result as c_int
}
