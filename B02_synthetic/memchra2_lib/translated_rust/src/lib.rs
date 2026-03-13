use std::ffi::c_int;

fn memchra(str_ptr: *const u8, c: c_int, n: usize) -> c_int {
    let mut count: c_int = 0;
    for i in 0..n {
        if unsafe { *str_ptr.add(i) } == c as u8 {
            count += 1;
        }
    }
    count
}

fn process_buffer(buffer: *const u8) -> c_int {
    if buffer.is_null() || unsafe { *buffer } == 0 {
        return -1;
    }
    let mut result: c_int = 0;
    let mut i = buffer;
    unsafe {
        while *i != 0 {
            result += *i as c_int;
            i = i.add(1);
        }
    }
    result
}

fn int_to_float_bits(value: c_int) -> f32 {
    f32::from_bits(value as u32)
}

fn safe_sum_array(arr: &[c_int]) -> c_int {
    if arr.is_empty() {
        return 0;
    }
    let mut sum: c_int = 0;
    for v in arr {
        sum += v;
    }
    sum
}

fn count_occurrences_in(text: &[u8]) -> c_int {
    // count '-' occurrences
    // The C code calls strlen then memchra with ch='-'
    // but text here is already the valid slice up to NUL
    memchra(text.as_ptr(), b'-' as c_int, text.len())
}

fn complex_iteration(data: &[c_int]) -> c_int {
    if data.is_empty() {
        return -1;
    }
    let mut result: c_int = 0;
    for v in data {
        let u = *v as u32;
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

    // dash_count = count_occurrences(buffer, '-')
    let dash_count = count_occurrences_in(&buffer[..copy_len]);
    result += dash_count * 10;

    // safe_sum_array
    let values = [a, b, c, d];
    let sum = safe_sum_array(&values);
    result += sum;

    // process_strings: count how many of {"test1","test2","testing","other"} start with "test"
    // "test1" starts with "test" -> yes
    // "test2" starts with "test" -> yes
    // "testing" starts with "test" -> yes
    // "other" starts with "test" -> no
    // matches = 3
    let matches: c_int = 3;
    result += matches * 5;

    // int_to_float_bits(a)
    let f = int_to_float_bits(a);
    if f > 0.0f32 && f < 1000.0f32 {
        result += f as c_int;
    }

    // process_buffer(buffer, strlen(buffer))
    let buf_sum = process_buffer(buffer.as_ptr());
    if buf_sum > 0 {
        result += buf_sum % 256;
    }

    // interpret_as_int: reinterpret 4 bytes as int (little-endian on x86)
    let bytes: [u8; 4] = [
        (b & 0xFF) as u8,
        (c & 0xFF) as u8,
        (d & 0xFF) as u8,
        0,
    ];
    let interpreted = c_int::from_ne_bytes(bytes);
    result ^= interpreted;

    // complex_iteration
    let complex_result = complex_iteration(&values);
    result += complex_result;

    result
}
