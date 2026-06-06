use std::ffi::c_int;

// On Linux, wchar_t is a 32-bit signed integer.
#[allow(non_camel_case_types)]
type wchar_t = i32;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wcscat(
    dst: *mut wchar_t,
    num_elem: usize,
    src: *const wchar_t,
) -> c_int {
    let mut ptr = dst;
    if dst.is_null() || num_elem == 0 {
        return 22;
    }
    if src.is_null() {
        unsafe {
            *dst.add(0) = 0;
        }
        return 22;
    }
    let end = unsafe { dst.add(num_elem) };
    unsafe {
        while ptr < end && *ptr != 0 {
            ptr = ptr.add(1);
        }
    }
    let mut s = src;
    unsafe {
        while ptr < end {
            let v = *s;
            *ptr = v;
            ptr = ptr.add(1);
            s = s.add(1);
            if v == 0 {
                return 0;
            }
        }
        *dst.add(0) = 0;
    }
    34
}
