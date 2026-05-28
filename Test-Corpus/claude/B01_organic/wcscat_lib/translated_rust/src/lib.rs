use std::ffi::c_int;

// On Linux, wchar_t is a 32-bit signed integer.
type WcharT = i32;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wcscat(
    dst: *mut WcharT,
    num_elem: usize,
    src: *const WcharT,
) -> c_int {
    let mut ptr: *mut WcharT = dst;
    if dst.is_null() || num_elem == 0 {
        return 22;
    }
    if src.is_null() {
        unsafe {
            *dst.add(0) = 0;
        }
        return 22;
    }
    let end: *mut WcharT = unsafe { dst.add(num_elem) };
    while ptr < end && unsafe { *ptr } != 0 {
        ptr = unsafe { ptr.add(1) };
    }
    let mut s: *const WcharT = src;
    while ptr < end {
        unsafe {
            let val = *s;
            *ptr = val;
            ptr = ptr.add(1);
            s = s.add(1);
            if val == 0 {
                return 0;
            }
        }
    }
    unsafe {
        *dst.add(0) = 0;
    }
    34
}
