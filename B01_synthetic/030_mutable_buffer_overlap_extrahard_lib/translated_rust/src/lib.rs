use std::os::raw::c_int;

/// # Safety
/// All pointers must be valid for `len` elements.
/// Pointers may alias (replicates C sequential semantics).
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
            let v = (*mul1.offset(i)) * (*mul2.offset(i)) + (*add.offset(i));
            *out.offset(i) = v;
        }
    }
}

/// # Safety
/// `data` must point to at least `len` valid ints.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: *const c_int, len: c_int) {
    let n = len as usize;
    let mut out = vec![0i32; n];
    unsafe {
        std::ptr::copy_nonoverlapping(data, out.as_mut_ptr(), n);
        // Call with all four pointers the same buffer — replicates C aliasing behavior
        fma_array(
            out.as_mut_ptr(),
            out.as_ptr(),
            out.as_ptr(),
            out.as_ptr(),
            len,
        );
    }
    for i in 0..n {
        println!("{}", out[i]);
    }
}
