use std::ffi::c_int;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    for i in 0..len as isize {
        unsafe {
            *out.offset(i) = *mul1.offset(i) * *mul2.offset(i) + *add.offset(i);
        }
    }
}

unsafe fn inner(out: *mut c_int, len: c_int) {
    fma_array(out, out, out, out, len);
    for i in 0..len as isize {
        unsafe {
            libc::printf(b"%d\n\0".as_ptr() as *const libc::c_char, *out.offset(i));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: *const c_int, len: c_int) {
    let size = len as usize;
    let mut out = vec![0i32; size];
    unsafe {
        std::ptr::copy_nonoverlapping(data, out.as_mut_ptr(), size);
        inner(out.as_mut_ptr(), len);
    }
}
