use std::ffi::{c_char, c_int};
use std::ptr;

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
    let mut i: c_int = 0;
    while i < len {
        let idx = i as isize;
        let lhs = unsafe { *mul1.offset(idx) };
        let rhs = unsafe { *mul2.offset(idx) };
        let addend = unsafe { *add.offset(idx) };
        unsafe {
            *out.offset(idx) = lhs.wrapping_mul(rhs).wrapping_add(addend);
        }
        i += 1;
    }
}

fn inner(out: *mut c_int, len: c_int) {
    unsafe {
        fma_array(out, out, out, out, len);
    }

    let mut i: c_int = 0;
    while i < len {
        let value = unsafe { *out.offset(i as isize) };
        unsafe {
            printf(c"%d\n".as_ptr(), value);
        }
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: *const c_int, len: c_int) {
    let count = len as usize;
    let mut out = Vec::<c_int>::with_capacity(count);
    unsafe {
        out.set_len(count);
        ptr::copy_nonoverlapping(data, out.as_mut_ptr(), count);
    }
    inner(out.as_mut_ptr(), len);
}
