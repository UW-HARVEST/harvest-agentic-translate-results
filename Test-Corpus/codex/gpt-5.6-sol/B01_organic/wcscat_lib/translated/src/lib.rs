use std::ffi::c_int;

/// Appends `src` to `dst` within a buffer of `num_elem` wide characters.
///
/// This intentionally preserves the source library's behavior, including
/// clearing the first destination element when the source does not fit.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wcscat(dst: *mut c_int, num_elem: usize, src: *const c_int) -> c_int {
    if dst.is_null() || num_elem == 0 {
        return 22;
    }

    if src.is_null() {
        unsafe {
            *dst = 0;
        }
        return 22;
    }

    let mut ptr = dst;
    let end = unsafe { dst.add(num_elem) };

    while ptr < end && unsafe { *ptr != 0 } {
        ptr = unsafe { ptr.add(1) };
    }

    let mut src_ptr = src;
    while ptr < end {
        let value = unsafe { *src_ptr };
        unsafe {
            *ptr = value;
            ptr = ptr.add(1);
            src_ptr = src_ptr.add(1);
        }
        if value == 0 {
            return 0;
        }
    }

    unsafe {
        *dst = 0;
    }
    34
}
