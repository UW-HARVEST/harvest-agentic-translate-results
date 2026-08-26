// Translated from C source in c_src/

use std::os::raw::c_int;

fn memchra(s: &[u8], c: c_int, n: usize) -> c_int {
    let target = c as u8 as i8;
    let mut count: c_int = 0;
    let upper = n.min(s.len());
    for i in 0..upper {
        if s[i] as i8 == target {
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
    let upper = len.min(buffer.len());
    for i in 0..upper {
        let b = buffer[i];
        if b == 0 {
            break;
        }
        // Match C's `int` from `char` semantics: char is signed on most
        // common platforms (e.g., x86 Linux), so sign-extend through i8.
        result = result.wrapping_add((b as i8) as c_int);
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
    let mut matches: c_int = 0;
    let upper = (count as usize).min(strings.len());
    let target_bytes = target.as_bytes();
    for i in 0..upper {
        let s = strings[i];
        let bytes = s.as_bytes();
        if bytes.is_empty() {
            continue;
        }
        if bytes.len() >= target_bytes.len() && &bytes[..target_bytes.len()] == target_bytes {
            matches += 1;
        }
    }
    matches
}

fn safe_sum_array(arr: &[c_int], size: usize) -> c_int {
    if arr.is_empty() || size == 0 {
        return 0;
    }
    let mut sum: c_int = 0;
    let upper = size.min(arr.len());
    for i in 0..upper {
        sum = sum.wrapping_add(arr[i]);
    }
    sum
}

fn interpret_as_int(bytes: &[u8], len: usize) -> c_int {
    let int_size = std::mem::size_of::<c_int>();
    if bytes.is_empty() || len < int_size {
        return 0;
    }
    if bytes.len() < int_size {
        return 0;
    }
    // Reinterpret first sizeof(int) bytes as a c_int using native endianness,
    // matching the C cast `*(int *)bytes`.
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&bytes[..int_size]);
    c_int::from_ne_bytes(buf)
}

fn count_occurrences(text: &[u8], ch: u8) -> c_int {
    if text.is_empty() || text[0] == 0 {
        return 0;
    }
    // Compute strlen-equivalent length up to the first null byte.
    let len = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    memchra(&text[..len], ch as c_int, len)
}

fn complex_iteration(data: &[c_int], count: usize) -> c_int {
    if data.is_empty() || count == 0 {
        return -1;
    }
    let mut result: c_int = 0;
    let upper = count.min(data.len());
    for i in 0..upper {
        let u = data[i] as u32;
        result ^= (u & 0xFF) as c_int;
    }
    result
}

#[no_mangle]
pub extern "C" fn memchra2(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut result: c_int = 0;

    // snprintf into 64-byte buffer
    let mut buffer = [0u8; 64];
    let formatted = format!("test{}-{}-{}-{}", a, b, c, d);
    let bytes = formatted.as_bytes();
    let copy_len = bytes.len().min(buffer.len() - 1);
    buffer[..copy_len].copy_from_slice(&bytes[..copy_len]);
    // buffer[copy_len] is already 0 (null-terminator)

    let dash_count = count_occurrences(&buffer, b'-');
    result = result.wrapping_add(dash_count.wrapping_mul(10));

    let values = [a, b, c, d];
    let sum = safe_sum_array(&values, 4);
    result = result.wrapping_add(sum);

    let test_strings: [&str; 4] = ["test1", "test2", "testing", "other"];
    let matches = process_strings(&test_strings, 4, "test");
    result = result.wrapping_add(matches.wrapping_mul(5));

    let f = int_to_float_bits(a);
    if f > 0.0f32 && f < 1000.0f32 {
        // C cast `(int)f` truncates toward zero; Rust's `as i32` does the same
        // and is saturating, which is safe for the bounded range here.
        result = result.wrapping_add(f as c_int);
    }

    // strlen of buffer
    let buf_strlen = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
    let buf_sum = process_buffer(&buffer, buf_strlen);
    if buf_sum > 0 {
        result = result.wrapping_add(buf_sum.rem_euclid(256));
    }

    let mut bytes_buf = [0u8; 4];
    bytes_buf[0] = (b & 0xFF) as u8;
    bytes_buf[1] = (c & 0xFF) as u8;
    bytes_buf[2] = (d & 0xFF) as u8;
    bytes_buf[3] = 0;

    let interpreted = interpret_as_int(&bytes_buf, 4);
    result ^= interpreted;

    let complex_result = complex_iteration(&values, 4);
    result = result.wrapping_add(complex_result);

    result
}
