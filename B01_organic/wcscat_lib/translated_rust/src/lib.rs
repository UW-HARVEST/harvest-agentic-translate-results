use std::ffi::c_int;

// wchar_t is i32 on Linux
type WcharT = i32;

#[unsafe(no_mangle)]
pub extern "C" fn wcscat(dst: *mut WcharT, num_elem: usize, src: *const WcharT) -> c_int {
    if dst.is_null() || num_elem == 0 {
        return 22;
    }
    if src.is_null() {
        unsafe { *dst = 0; }
        return 22;
    }

    let dst_end = unsafe { dst.add(num_elem) };
    let mut ptr = dst;

    // Advance ptr to end of existing string in dst
    while ptr < dst_end && unsafe { *ptr } != 0 {
        ptr = unsafe { ptr.add(1) };
    }

    // Copy src into dst starting at ptr
    let mut s = src;
    while ptr < dst_end {
        let ch = unsafe { *s };
        unsafe { *ptr = ch; }
        ptr = unsafe { ptr.add(1) };
        s = unsafe { s.add(1) };
        if ch == 0 {
            return 0;
        }
    }

    // Overflow: null-terminate dst[0] and return ERANGE
    unsafe { *dst = 0; }
    34
}
