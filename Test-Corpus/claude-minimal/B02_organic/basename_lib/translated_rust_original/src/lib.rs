use std::os::raw::c_char;

/// Returns a pointer to the basename portion of `path`.
///
/// This mirrors the C function:
/// ```c
/// char *tool_basename(char *path);
/// ```
///
/// It locates the last `/` and the last `\\` in the string and returns a
/// pointer to the character following whichever separator occurs later in the
/// string. If neither separator is found, the original pointer is returned.
///
/// # Safety
///
/// `path` must be a valid pointer to a NUL-terminated C string. The returned
/// pointer is either `path` itself or points into the same buffer; the caller
/// must ensure the buffer remains valid for the duration of use.
#[no_mangle]
pub unsafe extern "C" fn tool_basename(path: *mut c_char) -> *mut c_char {
    if path.is_null() {
        return path;
    }

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

/// Equivalent of C's `strrchr`: finds the last occurrence of byte `c` in the
/// NUL-terminated C string starting at `s`.
unsafe fn strrchr(s: *mut c_char, c: c_char) -> *mut c_char {
    let mut last: *mut c_char = std::ptr::null_mut();
    let mut p = s;
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
