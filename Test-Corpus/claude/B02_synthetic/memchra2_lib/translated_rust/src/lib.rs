use std::ffi::c_int;

fn memchra(s: &[u8], c: c_int) -> c_int {
    // (char)c — truncate to char (signed on x86_64); byte representation equals (c as u8)
    let target = c as u8;
    let mut count: c_int = 0;
    for &b in s {
        if b == target {
            count = count.wrapping_add(1);
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
        if i >= buffer.len() {
            break;
        }
        let b = buffer[i];
        if b == 0 {
            break;
        }
        // (int)(*i) where *i is signed char on x86_64 — sign-extend
        result = result.wrapping_add(b as i8 as c_int);
    }
    result
}

fn int_to_float_bits(value: c_int) -> f32 {
    // Type-punning union { int i; float f; } — reinterpret bits
    f32::from_bits(value as u32)
}

fn process_strings(strings: &[&[u8]], target: &[u8]) -> c_int {
    if strings.is_empty() {
        return 0;
    }
    let mut matches: c_int = 0;
    for s in strings {
        if s.is_empty() || s[0] == 0 {
            continue;
        }
        // strncmp(*i, target, strlen(target)) — compare exactly target.len() bytes
        if s.len() >= target.len() && &s[..target.len()] == target {
            matches = matches.wrapping_add(1);
        }
    }
    matches
}

fn safe_sum_array(arr: &[c_int]) -> c_int {
    if arr.is_empty() {
        return 0;
    }
    let mut sum: c_int = 0;
    for &x in arr {
        sum = sum.wrapping_add(x);
    }
    sum
}

fn interpret_as_int(bytes: &[u8]) -> c_int {
    if bytes.len() < std::mem::size_of::<c_int>() {
        return 0;
    }
    // Reinterpret first sizeof(int)==4 bytes as int — little-endian (x86_64)
    c_int::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn count_occurrences(text: &[u8], ch: u8) -> c_int {
    if text.is_empty() || text[0] == 0 {
        return 0;
    }
    let len = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    memchra(&text[..len], ch as c_int)
}

fn complex_iteration(data: &[c_int]) -> c_int {
    if data.is_empty() {
        return -1;
    }
    let mut result: c_int = 0;
    for &x in data {
        let u = x as u32;
        result ^= (u & 0xFF) as c_int;
    }
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn memchra2(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut result: c_int = 0;

    // snprintf(buffer, sizeof(buffer)=64, "test%d-%d-%d-%d", a, b, c, d)
    let formatted = format!("test{}-{}-{}-{}", a, b, c, d);
    let formatted_bytes = formatted.as_bytes();
    let mut buffer = [0u8; 64];
    let copy_len = formatted_bytes.len().min(63);
    buffer[..copy_len].copy_from_slice(&formatted_bytes[..copy_len]);
    // buffer[copy_len] = 0 (already zero-initialized)

    let dash_count = count_occurrences(&buffer, b'-');
    result = result.wrapping_add(dash_count.wrapping_mul(10));

    let values = [a, b, c, d];
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
    if f > 0.0f32 && f < 1000.0f32 {
        // (int)f — truncate toward zero. f is in (0,1000), so safe.
        result = result.wrapping_add(f as c_int);
    }

    // strlen(buffer)
    let buf_len = buffer.iter().position(|&x| x == 0).unwrap_or(buffer.len());
    let buf_sum = process_buffer(&buffer, buf_len);
    if buf_sum > 0 {
        // C signed % matches Rust % (truncated). buf_sum > 0 here so result is non-negative.
        result = result.wrapping_add(buf_sum % 256);
    }

    let bytes_arr: [u8; 4] = [
        (b & 0xFF) as u8,
        (c & 0xFF) as u8,
        (d & 0xFF) as u8,
        0u8,
    ];
    let interpreted = interpret_as_int(&bytes_arr);
    result ^= interpreted;

    let complex_result = complex_iteration(&values);
    result = result.wrapping_add(complex_result);

    result
}
