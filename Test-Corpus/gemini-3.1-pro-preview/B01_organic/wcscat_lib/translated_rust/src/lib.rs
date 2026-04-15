use libc::wchar_t;
use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn wcscat(dst: *mut wchar_t, num_elem: usize, src: *const wchar_t) -> c_int {
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
    unsafe {
        while ptr < end && *ptr != 0 {
            ptr = ptr.add(1);
        }
        let mut src_ptr = src;
        while ptr < end {
            let val = *src_ptr;
            *ptr = val;
            ptr = ptr.add(1);
            src_ptr = src_ptr.add(1);
            if val == 0 {
                return 0;
            }
        }
        *dst = 0;
    }
    34
}
