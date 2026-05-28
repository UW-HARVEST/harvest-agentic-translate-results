use std::ffi::c_char;

/// Returns a pointer to the basename portion of `path`, where the basename is
/// the substring after the last '/' or '\\' character. If neither separator is
/// present, returns the original pointer.
///
/// # Safety
/// `path` must be a valid pointer to a NUL-terminated C string, or behavior
/// matches the C implementation (which would itself invoke UB on a null/invalid
/// pointer via `strrchr`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tool_basename(path: *mut c_char) -> *mut c_char {
    // Find the last occurrence of '/' (s1) and '\\' (s2) in the C string.
    let s1 = strrchr(path, b'/' as c_char);
    let s2 = strrchr(path, b'\\' as c_char);

    let mut result = path;

    if !s1.is_null() && !s2.is_null() {
        result = if (s1 as usize) > (s2 as usize) {
            s1.add(1)
        } else {
            s2.add(1)
        };
    } else if !s1.is_null() {
        result = s1.add(1);
    } else if !s2.is_null() {
        result = s2.add(1);
    }

    result
}

/// Mimics C's `strrchr`: returns a pointer to the last occurrence of `c` in
/// the NUL-terminated C string `s`, or null if not found. The terminating NUL
/// byte is considered part of the string for matching purposes (matching C
/// semantics).
unsafe fn strrchr(s: *const c_char, c: c_char) -> *mut c_char {
    if s.is_null() {
        return std::ptr::null_mut();
    }
    let mut last: *mut c_char = std::ptr::null_mut();
    let mut p = s as *mut c_char;
    loop {
        let ch = *p;
        if ch == c {
            last = p;
        }
        if ch == 0 {
            break;
        }
        p = p.add(1);
    }
    last
}
