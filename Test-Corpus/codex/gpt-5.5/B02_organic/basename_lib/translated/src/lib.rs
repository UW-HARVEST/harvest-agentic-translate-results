use std::ffi::c_char;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tool_basename(path: *mut c_char) -> *mut c_char {
    let mut current = path;
    let mut last_slash: *mut c_char = std::ptr::null_mut();
    let mut last_backslash: *mut c_char = std::ptr::null_mut();

    while unsafe { *current } != 0 {
        match unsafe { *current as u8 } {
            b'/' => last_slash = current,
            b'\\' => last_backslash = current,
            _ => {}
        }

        current = unsafe { current.add(1) };
    }

    if !last_slash.is_null() && !last_backslash.is_null() {
        if last_slash > last_backslash {
            unsafe { last_slash.add(1) }
        } else {
            unsafe { last_backslash.add(1) }
        }
    } else if !last_slash.is_null() {
        unsafe { last_slash.add(1) }
    } else if !last_backslash.is_null() {
        unsafe { last_backslash.add(1) }
    } else {
        path
    }
}
