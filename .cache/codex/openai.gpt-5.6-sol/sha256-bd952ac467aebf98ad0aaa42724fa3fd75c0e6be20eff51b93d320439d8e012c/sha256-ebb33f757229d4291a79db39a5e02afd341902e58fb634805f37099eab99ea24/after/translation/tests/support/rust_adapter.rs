include!("../../src/lib.rs");

#[unsafe(no_mangle)]
pub extern "C" fn verification_process_buffer(
    buffer: *mut std::ffi::c_char,
    len: usize,
) -> std::ffi::c_int {
    if buffer.is_null() {
        return -1;
    }
    let bytes = unsafe { std::slice::from_raw_parts(buffer.cast::<u8>(), len) };
    process_buffer(bytes)
}

#[unsafe(no_mangle)]
pub extern "C" fn verification_process_strings(
    strings: *mut *mut std::ffi::c_char,
    count: std::ffi::c_int,
    target: *const std::ffi::c_char,
) -> std::ffi::c_int {
    if strings.is_null() || count <= 0 {
        return 0;
    }

    let mut converted = Vec::with_capacity(count as usize);
    for index in 0..count as usize {
        let string = unsafe { *strings.add(index) };
        if string.is_null() {
            converted.push(&[][..]);
        } else {
            converted.push(unsafe { std::ffi::CStr::from_ptr(string) }.to_bytes());
        }
    }
    let target = unsafe { std::ffi::CStr::from_ptr(target) }.to_bytes();
    process_strings(&converted, target)
}

#[unsafe(no_mangle)]
pub extern "C" fn verification_safe_sum_array(
    array: *mut std::ffi::c_int,
    size: usize,
) -> std::ffi::c_int {
    if array.is_null() || size == 0 {
        return 0;
    }
    safe_sum_array(unsafe { std::slice::from_raw_parts(array, size) })
}

#[unsafe(no_mangle)]
pub extern "C" fn verification_interpret_as_int(bytes: *mut u8, len: usize) -> std::ffi::c_int {
    if bytes.is_null() || len < size_of::<std::ffi::c_int>() {
        return 0;
    }
    interpret_as_int(unsafe { std::slice::from_raw_parts(bytes, len) })
}

#[unsafe(no_mangle)]
pub extern "C" fn verification_count_occurrences(
    text: *const std::ffi::c_char,
    ch: std::ffi::c_char,
) -> std::ffi::c_int {
    if text.is_null() {
        return 0;
    }
    count_occurrences(
        unsafe { std::ffi::CStr::from_ptr(text) }.to_bytes(),
        ch as u8,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn verification_complex_iteration(
    data: *mut std::ffi::c_int,
    count: usize,
) -> std::ffi::c_int {
    if data.is_null() || count == 0 {
        return -1;
    }
    complex_iteration(unsafe { std::slice::from_raw_parts(data, count) })
}
