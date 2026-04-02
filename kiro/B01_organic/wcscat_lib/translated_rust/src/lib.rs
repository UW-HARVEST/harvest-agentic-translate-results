use std::ffi::c_int;

// wchar_t is i32 on Linux (4 bytes)
type WcharT = i32;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wcscat(dst: *mut WcharT, num_elem: usize, src: *const WcharT) -> c_int {
    if dst.is_null() || num_elem == 0 {
        return 22;
    }
    if src.is_null() {
        unsafe { *dst = 0; }
        return 22;
    }

    let mut ptr = dst;
    let end = unsafe { dst.add(num_elem) };

    while ptr < end && unsafe { *ptr } != 0 {
        ptr = unsafe { ptr.add(1) };
    }

    let mut s = src;
    while ptr < end {
        let ch = unsafe { *s };
        unsafe { *ptr = ch; }
        ptr = unsafe { ptr.add(1) };
        s = unsafe { s.add(1) };
        if ch == 0 {
            return 0;
        }
    }

    unsafe { *dst = 0; }
    34
}
