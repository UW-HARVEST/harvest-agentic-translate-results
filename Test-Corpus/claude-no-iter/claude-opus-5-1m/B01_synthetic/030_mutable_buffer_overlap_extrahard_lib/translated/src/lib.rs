use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    let mut i: isize = 0;
    while i < len as isize {
        let v = (*mul1.offset(i)).wrapping_mul(*mul2.offset(i)).wrapping_add(*add.offset(i));
        *out.offset(i) = v;
        i += 1;
    }
}

unsafe fn inner(out: *mut c_int, len: c_int) {
    fma_array(out, out, out, out, len);
    let fmt = b"%d\n\0".as_ptr();
    let mut i: isize = 0;
    while i < len as isize {
        printf(fmt, *out.offset(i));
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: *const c_int, len: c_int) {
    // Mirror C's VLA: int out[len]; memcpy(out, data, len * sizeof(int));
    let n = len as usize;
    let mut out: Vec<c_int> = Vec::with_capacity(n);
    if !data.is_null() && n > 0 {
        std::ptr::copy_nonoverlapping(data, out.as_mut_ptr(), n);
    }
    out.set_len(n);
    inner(out.as_mut_ptr(), len);
}
