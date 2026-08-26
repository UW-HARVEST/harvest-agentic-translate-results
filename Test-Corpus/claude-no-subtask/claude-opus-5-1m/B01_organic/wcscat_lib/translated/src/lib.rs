use std::ffi::c_int;

// On Linux/macOS, wchar_t is a 32-bit signed integer.
#[allow(non_camel_case_types)]
type wchar_t = i32;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wcscat(
    dst: *mut wchar_t,
    num_elem: usize,
    src: *const wchar_t,
) -> c_int {
    let mut ptr: *mut wchar_t = dst;
    if dst.is_null() || num_elem == 0 {
        return 22;
    }
    if src.is_null() {
        unsafe { *dst.add(0) = 0 };
        return 22;
    }
    let end: *mut wchar_t = unsafe { dst.add(num_elem) };
    while ptr < end && unsafe { *ptr } != 0 {
        ptr = unsafe { ptr.add(1) };
    }
    let mut s = src;
    while ptr < end {
        let val = unsafe { *s };
        unsafe { *ptr = val };
        ptr = unsafe { ptr.add(1) };
        s = unsafe { s.add(1) };
        if val == 0 {
            return 0;
        }
    }
    unsafe { *dst.add(0) = 0 };
    34
}
