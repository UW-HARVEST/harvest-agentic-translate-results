use std::ffi::c_int;

/// # Safety
/// Caller must ensure `dst` points to a valid wchar_t buffer of at least `num_elem` elements,
/// and `src` (if non-null) points to a null-terminated wchar_t string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wcscat(dst: *mut libc::wchar_t, num_elem: usize, src: *const libc::wchar_t) -> c_int {
    if dst.is_null() || num_elem == 0 {
        return 22;
    }
    if src.is_null() {
        unsafe { *dst = 0; }
        return 22;
    }

    let mut ptr = dst;
    let end = unsafe { dst.add(num_elem) };

    // Advance ptr to end of existing string in dst
    unsafe {
        while ptr < end && *ptr != 0 {
            ptr = ptr.add(1);
        }
    }

    // Copy src into dst starting at ptr
    let mut s = src;
    unsafe {
        while ptr < end {
            let ch = *s;
            *ptr = ch;
            ptr = ptr.add(1);
            s = s.add(1);
            if ch == 0 {
                return 0;
            }
        }
    }

    // Buffer overflow: null-terminate at dst[0] and return ERANGE (34)
    unsafe { *dst = 0; }
    34
}
