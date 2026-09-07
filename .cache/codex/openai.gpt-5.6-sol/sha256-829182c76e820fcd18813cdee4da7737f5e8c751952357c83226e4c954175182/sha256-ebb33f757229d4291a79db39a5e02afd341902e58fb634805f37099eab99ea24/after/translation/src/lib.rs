use std::ffi::c_int;

/// Appends a NUL-terminated wide string within a fixed-size destination buffer.
///
/// The ABI uses `c_int` because `wchar_t` is a signed 32-bit integer on the
/// target platform.
///
/// # Safety
///
/// Non-null pointers must refer to memory satisfying the same requirements as
/// the original C function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wcscat(dst: *mut c_int, num_elem: usize, src: *const c_int) -> c_int {
    let mut ptr = dst;

    if dst.is_null() || num_elem == 0 {
        return 22;
    }

    if src.is_null() {
        unsafe {
            *dst = 0;
        }
        return 22;
    }

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
