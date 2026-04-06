use std::os::raw::c_int;

fn memchra(str_ptr: *const u8, c: c_int, n: usize) -> c_int {
    let mut count: c_int = 0;
    for i in 0..n {
        if unsafe { *str_ptr.add(i) } == c as u8 {
            count += 1;
        }
    }
    count
}

fn process_buffer(buffer: *mut u8, len: usize) -> c_int {
    if buffer.is_null() || unsafe { *buffer } == 0 {
        return -1;
    }
    let mut result: c_int = 0;
    let mut i = buffer;
    let end = unsafe { buffer.add(len) };
    while i < end && unsafe { *i } != 0 {
        result += unsafe { *i } as c_int;
        i = unsafe { i.add(1) };
    }
    result
}

fn int_to_float_bits(value: c_int) -> f32 {
    f32::from_bits(value as u32)
}

fn process_strings(strings: *mut *mut u8, count: c_int, target: *const u8) -> c_int {
    if strings.is_null() || count <= 0 {
        return 0;
    }
    let target_len = unsafe { libc_strlen(target) };
    let mut matches: c_int = 0;
    for idx in 0..count as usize {
        let s = unsafe { *strings.add(idx) };
        if s.is_null() || unsafe { *s } == 0 {
            continue;
        }
        if unsafe { libc_strncmp(s, target, target_len) } == 0 {
            matches += 1;
        }
    }
    matches
}

fn safe_sum_array(arr: *const c_int, size: usize) -> c_int {
    if arr.is_null() || size == 0 {
        return 0;
    }
    let mut sum: c_int = 0;
    for i in 0..size {
        sum = sum.wrapping_add(unsafe { *arr.add(i) });
    }
    sum
}

fn interpret_as_int(bytes: *const u8, len: usize) -> c_int {
    if bytes.is_null() || len < std::mem::size_of::<c_int>() {
        return 0;
    }
    let mut buf = [0u8; 4];
    unsafe { std::ptr::copy_nonoverlapping(bytes, buf.as_mut_ptr(), 4) };
    c_int::from_ne_bytes(buf)
}

fn count_occurrences(text: *const u8, ch: u8) -> c_int {
    if text.is_null() || unsafe { *text } == 0 {
        return 0;
    }
    let len = unsafe { libc_strlen(text) };
    memchra(text, ch as c_int, len)
}

fn complex_iteration(data: *const c_int, count: usize) -> c_int {
    if data.is_null() || count == 0 {
        return -1;
    }
    let mut result: c_int = 0;
    for i in 0..count {
        let u = unsafe { *data.add(i) } as u32;
        result ^= (u & 0xFF) as c_int;
    }
    result
}

// Minimal C-compatible helpers to avoid libc dependency
unsafe fn libc_strlen(s: *const u8) -> usize {
    let mut len = 0;
    while *s.add(len) != 0 {
        len += 1;
    }
    len
}

unsafe fn libc_strncmp(s1: *const u8, s2: *const u8, n: usize) -> c_int {
    for i in 0..n {
        let a = *s1.add(i);
        let b = *s2.add(i);
        if a != b {
            return a as c_int - b as c_int;
        }
        if a == 0 {
            return 0;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn memchra2(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut result: c_int = 0;

    // snprintf(buffer, 64, "test%d-%d-%d-%d", a, b, c, d)
    let formatted = format!("test{}-{}-{}-{}\0", a, b, c, d);
    let mut buffer = [0u8; 64];
    let copy_len = formatted.len().min(63);
    buffer[..copy_len].copy_from_slice(&formatted.as_bytes()[..copy_len]);
    buffer[copy_len] = 0; // ensure null termination (snprintf guarantees this)

    let dash_count = count_occurrences(buffer.as_ptr(), b'-');
    result += dash_count * 10;

    let values: [c_int; 4] = [a, b, c, d];
    let sum = safe_sum_array(values.as_ptr(), 4);
    result += sum;

    let mut s0 = *b"test1\0";
    let mut s1 = *b"test2\0";
    let mut s2 = *b"testing\0";
    let mut s3 = *b"other\0";
    let mut test_strings: [*mut u8; 4] = [
        s0.as_mut_ptr(),
        s1.as_mut_ptr(),
        s2.as_mut_ptr(),
        s3.as_mut_ptr(),
    ];
    let target = b"test\0";
    let matches = process_strings(test_strings.as_mut_ptr(), 4, target.as_ptr());
    result += matches * 5;

    let f = int_to_float_bits(a);
    if f > 0.0f32 && f < 1000.0f32 {
        result += f as c_int;
    }

    let buf_len = unsafe { libc_strlen(buffer.as_ptr()) };
    let buf_sum = process_buffer(buffer.as_mut_ptr(), buf_len);
    if buf_sum > 0 {
        result += buf_sum % 256;
    }

    let mut bytes = [0u8; 4];
    bytes[0] = (b & 0xFF) as u8;
    bytes[1] = (c & 0xFF) as u8;
    bytes[2] = (d & 0xFF) as u8;
    bytes[3] = 0;

    let interpreted = interpret_as_int(bytes.as_ptr(), 4);
    result ^= interpreted;

    let complex_result = complex_iteration(values.as_ptr(), 4);
    result += complex_result;

    result
}
