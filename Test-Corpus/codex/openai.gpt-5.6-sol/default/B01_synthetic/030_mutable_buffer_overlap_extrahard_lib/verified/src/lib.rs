use std::ffi::{c_char, c_int};
use std::ptr;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const DECIMAL_LINE_FORMAT: &[u8] = b"%d\n\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    for i in 0..len {
        let offset = i as usize;
        let product = unsafe { ptr::read(mul1.add(offset)) }
            .wrapping_mul(unsafe { ptr::read(mul2.add(offset)) });
        let value = product.wrapping_add(unsafe { ptr::read(add.add(offset)) });
        unsafe { ptr::write(out.add(offset), value) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: *const c_int, len: c_int) {
    if len <= 0 {
        return;
    }

    let len = len as usize;
    let mut out = Vec::<c_int>::with_capacity(len);
    unsafe {
        ptr::copy_nonoverlapping(data, out.as_mut_ptr(), len);
        out.set_len(len);
        fma_array(
            out.as_mut_ptr(),
            out.as_ptr(),
            out.as_ptr(),
            out.as_ptr(),
            len as c_int,
        );
    }

    for value in out {
        unsafe { printf(DECIMAL_LINE_FORMAT.as_ptr().cast(), value) };
    }
}
