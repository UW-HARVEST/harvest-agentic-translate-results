use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    let len_usize = len as usize;
    for i in 0..len_usize {
        let m1 = unsafe { *mul1.add(i) };
        let m2 = unsafe { *mul2.add(i) };
        let a = unsafe { *add.add(i) };
        // Reproduce C's signed integer arithmetic with wrapping semantics
        let prod = m1.wrapping_mul(m2);
        let result = prod.wrapping_add(a);
        unsafe { *out.add(i) = result };
    }
}

fn inner(out: &mut [c_int]) {
    let len = out.len() as c_int;
    // fma_array(out, out, out, out, len);
    // Equivalent to: out[i] = out[i] * out[i] + out[i];
    unsafe {
        fma_array(
            out.as_mut_ptr(),
            out.as_ptr(),
            out.as_ptr(),
            out.as_ptr(),
            len,
        );
    }
    let fmt = b"%d\n\0".as_ptr() as *const std::os::raw::c_char;
    for &val in out.iter() {
        unsafe {
            libc::printf(fmt, val);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: *const c_int, len: c_int) {
    let len_usize = len as usize;
    let mut out: Vec<c_int> = Vec::with_capacity(len_usize);
    for i in 0..len_usize {
        out.push(unsafe { *data.add(i) });
    }
    inner(&mut out);
}
