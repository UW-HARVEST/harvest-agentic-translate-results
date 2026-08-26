use std::ffi::{c_char, c_int, c_uchar};

extern "C" {
    fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;
}

fn memchra(s: *const c_char, c: c_int, n: usize) -> c_int {
    let mut count: c_int = 0;
    let target = c as c_char;
    for i in 0..n {
        unsafe {
            if *s.add(i) == target {
                count = count.wrapping_add(1);
            }
        }
    }
    count
}

fn process_buffer(buffer: *mut c_char, len: usize) -> c_int {
    if buffer.is_null() || unsafe { *buffer } == 0 {
        return -1;
    }
    let mut result: c_int = 0;
    let end = unsafe { buffer.add(len) };
    let mut i = buffer;
    while i < end && unsafe { *i } != 0 {
        result = result.wrapping_add(unsafe { *i } as c_int);
        i = unsafe { i.add(1) };
    }
    result
}

fn int_to_float_bits(value: c_int) -> f32 {
    f32::from_bits(value as u32)
}

fn process_strings(strings: *mut *mut c_char, count: c_int, target: *const c_char) -> c_int {
    if strings.is_null() || count <= 0 {
        return 0;
    }
    let mut matches: c_int = 0;
    let target_len = unsafe { strlen(target) };
    for idx in 0..count as usize {
        let s = unsafe { *strings.add(idx) };
        if s.is_null() || unsafe { *s } == 0 {
            continue;
        }
        if unsafe { strncmp(s, target, target_len) } == 0 {
            matches = matches.wrapping_add(1);
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

fn interpret_as_int(bytes: *const c_uchar, len: usize) -> c_int {
    if bytes.is_null() || len < std::mem::size_of::<c_int>() {
        return 0;
    }
    unsafe { (bytes as *const c_int).read_unaligned() }
}

fn count_occurrences(text: *const c_char, ch: c_char) -> c_int {
    if text.is_null() || unsafe { *text } == 0 {
        return 0;
    }
    let len = unsafe { strlen(text) };
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memchra2(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut result: c_int = 0;

    let mut buffer: [c_char; 64] = [0; 64];
    let fmt = b"test%d-%d-%d-%d\0".as_ptr() as *const c_char;
    snprintf(buffer.as_mut_ptr(), 64, fmt, a, b, c, d);

    let dash_count = count_occurrences(buffer.as_ptr(), b'-' as c_char);
    result = result.wrapping_add(dash_count.wrapping_mul(10));

    let values: [c_int; 4] = [a, b, c, d];
    let sum = safe_sum_array(values.as_ptr(), 4);
    result = result.wrapping_add(sum);

    let mut test_strings: [*mut c_char; 4] = [
        b"test1\0".as_ptr() as *mut c_char,
        b"test2\0".as_ptr() as *mut c_char,
        b"testing\0".as_ptr() as *mut c_char,
        b"other\0".as_ptr() as *mut c_char,
    ];

    let matches = process_strings(
        test_strings.as_mut_ptr(),
        4,
        b"test\0".as_ptr() as *const c_char,
    );
    result = result.wrapping_add(matches.wrapping_mul(5));

    let f = int_to_float_bits(a);
    if f > 0.0f32 && f < 1000.0f32 {
        result = result.wrapping_add(f as c_int);
    }

    let buf_sum = process_buffer(buffer.as_mut_ptr(), strlen(buffer.as_ptr()));
    if buf_sum > 0 {
        result = result.wrapping_add(buf_sum % 256);
    }

    let mut bytes: [c_uchar; 4] = [0; 4];
    bytes[0] = (b & 0xFF) as c_uchar;
    bytes[1] = (c & 0xFF) as c_uchar;
    bytes[2] = (d & 0xFF) as c_uchar;
    bytes[3] = 0;

    let interpreted = interpret_as_int(bytes.as_ptr(), 4);
    result ^= interpreted;

    let complex_result = complex_iteration(values.as_ptr(), 4);
    result = result.wrapping_add(complex_result);

    result
}
