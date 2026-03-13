use std::os::raw::c_char;

/// # Safety
/// `path` must point to a valid null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tool_basename(path: *mut c_char) -> *mut c_char {
    unsafe {
        let s1 = libc::strrchr(path, b'/' as i32);
        let s2 = libc::strrchr(path, b'\\' as i32);

        if !s1.is_null() && !s2.is_null() {
            if s1 > s2 { s1.add(1) } else { s2.add(1) }
        } else if !s1.is_null() {
            s1.add(1)
        } else if !s2.is_null() {
            s2.add(1)
        } else {
            path
        }
    }
}
