use libc::{memcpy, printf};
use std::ffi::c_int;

static PRINTF_INT_LINE_FORMAT: &[u8] = b"%d\n\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    let mut i = 0;
    while i < len {
        let idx = i as isize;
        let value = (*mul1.offset(idx))
            .wrapping_mul(*mul2.offset(idx))
            .wrapping_add(*add.offset(idx));
        *out.offset(idx) = value;
        i += 1;
    }
}

unsafe fn inner(out: *mut c_int, len: c_int) {
    fma_array(out, out, out, out, len);

    let mut i = 0;
    while i < len {
        printf(PRINTF_INT_LINE_FORMAT.as_ptr().cast(), *out.offset(i as isize));
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: *const c_int, len: c_int) {
    let len_usize = len as usize;
    let mut out = vec![0 as c_int; len_usize];

    memcpy(
        out.as_mut_ptr().cast(),
        data.cast(),
        len_usize.wrapping_mul(std::mem::size_of::<c_int>()),
    );

    inner(out.as_mut_ptr(), len);
}
