use std::ffi::c_int;

type WChar = i32;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wcscat(dst: *mut WChar, num_elem: usize, src: *const WChar) -> c_int {
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

    while ptr < end && unsafe { *ptr } != 0 {
        ptr = unsafe { ptr.add(1) };
    }

    let mut src_ptr = src;
    while ptr < end {
        let value = unsafe { *src_ptr };
        unsafe {
            *ptr = value;
        }
        ptr = unsafe { ptr.add(1) };
        src_ptr = unsafe { src_ptr.add(1) };

        if value == 0 {
            return 0;
        }
    }

    unsafe {
        *dst = 0;
    }
    34
}
