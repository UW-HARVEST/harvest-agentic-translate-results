use std::ffi::c_int;

/// out[i] = mul1[i] * mul2[i] + add[i]
///
/// # Safety
/// All pointers must be valid for `len` reads/writes of `c_int`.
/// `out` must not alias `mul1`, `mul2`, or `add` (matches C `restrict`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    for i in 0..len {
        let i = i as isize;
        unsafe {
            *out.offset(i) = (*mul1.offset(i))
                .wrapping_mul(*mul2.offset(i))
                .wrapping_add(*add.offset(i));
        }
    }
}

/// Replicates the original C implementation, including its dead stores.
///
/// # Safety
/// `data` must be valid for `len` reads of `c_int`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn call_fma(data: *const c_int, len: c_int) -> c_int {
    if len == 0 {
        return 0;
    }

    let n = len as usize;
    let mut out: Vec<c_int> = vec![0; n];
    let mut ones: Vec<c_int> = vec![0; n];
    let mut zeros: Vec<c_int> = vec![0; n];

    // Mirror C: out[0] = 0; (dead store, will be overwritten)
    out[0] = 0;
    for i in 0..n {
        ones[i] = 1;
        zeros[i] = 0;
    }

    unsafe {
        fma_array(
            out.as_mut_ptr(),
            ones.as_ptr(),
            data,
            zeros.as_ptr(),
            len,
        );
    }

    out[n - 1]
}
