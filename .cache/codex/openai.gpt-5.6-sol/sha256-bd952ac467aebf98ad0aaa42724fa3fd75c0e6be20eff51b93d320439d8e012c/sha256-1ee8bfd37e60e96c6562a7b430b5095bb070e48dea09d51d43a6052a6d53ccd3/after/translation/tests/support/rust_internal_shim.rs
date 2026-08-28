include!("../../src/lib.rs");

#[unsafe(no_mangle)]
pub unsafe extern "C" fn verify_process_buffer(
    buffer: *mut std::ffi::c_char,
    len: usize,
) -> std::ffi::c_int {
    if buffer.is_null() {
        return -1;
    }
    if unsafe { *buffer } == 0 {
        return -1;
    }
    if len == 0 {
        return 0;
    }

    let bytes = unsafe { std::slice::from_raw_parts(buffer.cast::<u8>(), len) };
    process_buffer(bytes)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn verify_process_strings(
    strings: *mut *mut std::ffi::c_char,
    count: std::ffi::c_int,
    target: *const std::ffi::c_char,
) -> std::ffi::c_int {
    if strings.is_null() || count <= 0 {
        return 0;
    }

    let pointers = unsafe { std::slice::from_raw_parts(strings, count as usize) };
    let converted: Vec<&[u8]> = pointers
        .iter()
        .map(|&pointer| {
            if pointer.is_null() {
                &[][..]
            } else {
                unsafe { std::ffi::CStr::from_ptr(pointer) }.to_bytes()
            }
        })
        .collect();
    let target = unsafe { std::ffi::CStr::from_ptr(target) }.to_bytes();
    process_strings(&converted, target)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn verify_safe_sum_array(
    values: *mut std::ffi::c_int,
    size: usize,
) -> std::ffi::c_int {
    if values.is_null() || size == 0 {
        return 0;
    }

    let values = unsafe { std::slice::from_raw_parts(values, size) };
    safe_sum_array(values)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn verify_interpret_as_int(bytes: *mut u8, len: usize) -> std::ffi::c_int {
    if bytes.is_null() || len < size_of::<std::ffi::c_int>() {
        return 0;
    }

    let bytes = unsafe { std::slice::from_raw_parts(bytes, len) };
    interpret_as_int(bytes)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn verify_count_occurrences(
    text: *const std::ffi::c_char,
    value: std::ffi::c_char,
) -> std::ffi::c_int {
    if text.is_null() {
        return 0;
    }

    let text = unsafe { std::ffi::CStr::from_ptr(text) }.to_bytes();
    count_occurrences(text, value as u8)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn verify_complex_iteration(
    values: *mut std::ffi::c_int,
    count: usize,
) -> std::ffi::c_int {
    if values.is_null() || count == 0 {
        return -1;
    }

    let values = unsafe { std::slice::from_raw_parts(values, count) };
    complex_iteration(values)
}
