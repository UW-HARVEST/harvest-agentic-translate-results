use std::ffi::c_int;

// On Linux, wchar_t is a 32-bit signed integer (4 bytes).
// We use i32 here to match glibc's wchar_t representation.
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
    unsafe {
        while ptr < end && *ptr != 0 {
            ptr = ptr.add(1);
        }
    }
    let mut s: *const WcharT = src;
    unsafe {
        while ptr < end {
            let val = *s;
            *ptr = val;
            ptr = ptr.add(1);
            s = s.add(1);
            if val == 0 {
                return 0;
            }
        }
        *dst.add(0) = 0;
    }
    34
}
