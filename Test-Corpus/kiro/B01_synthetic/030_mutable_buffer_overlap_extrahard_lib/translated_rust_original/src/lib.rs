use std::os::raw::c_int;

/// # Safety
/// All pointers must be valid for `len` elements.
/// Pointers may alias (matching C semantics where reads see prior writes).
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

/// # Safety
/// `data` must be valid for `len` elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: *const c_int, len: c_int) {
    unsafe {
        let n = len as usize;
        let mut out = vec![0i32; n];
        std::ptr::copy_nonoverlapping(data, out.as_mut_ptr(), n);
        // inner: fma_array with all pointers aliasing out, then print
        fma_array(
            out.as_mut_ptr(),
            out.as_ptr(),
            out.as_ptr(),
            out.as_ptr(),
            len,
        );
        for i in 0..n {
            libc::printf(b"%d\n\0".as_ptr() as *const libc::c_char, out[i]);
        }
    }
}
