use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    for i in 0..len {
        let offset = i as isize;
        // Preserve the C loop's per-element read-before-write order.
        let lhs = unsafe { *mul1.offset(offset) };
        let rhs = unsafe { *mul2.offset(offset) };
        let addend = unsafe { *add.offset(offset) };
        unsafe {
            *out.offset(offset) = lhs.wrapping_mul(rhs).wrapping_add(addend);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: *const c_int, len: c_int) {
    if len <= 0 {
        return;
    }

    let len = len as usize;
    let mut out = vec![0; len];
    unsafe {
        std::ptr::copy_nonoverlapping(data, out.as_mut_ptr(), len);
        fma_array(
            out.as_mut_ptr(),
            out.as_ptr(),
            out.as_ptr(),
            out.as_ptr(),
            len as c_int,
        );
    }

    const INTEGER_LINE: &[u8] = b"%d\n\0";
    for value in out {
        unsafe {
            printf(INTEGER_LINE.as_ptr().cast(), value);
        }
    }
}
