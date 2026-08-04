use std::ffi::c_int;

fn inner(out: &mut [c_int], len: usize) {
    // C: fma_array(out, out, out, out, len) — all four pointers alias.
    // Each iteration reads out[i] then writes out[i] = out[i]*out[i] + out[i].
    for i in 0..len {
        let v = out[i];
        out[i] = v.wrapping_mul(v).wrapping_add(v);
    }
    let fmt = b"%d\n\0".as_ptr() as *const std::ffi::c_char;
    for i in 0..len {
        unsafe {
            libc::printf(fmt, out[i]);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: *const c_int, len: c_int) {
    let len_usize = len as usize;
    let src = unsafe { std::slice::from_raw_parts(data, len_usize) };
    let mut out: Vec<c_int> = src.to_vec();
    inner(&mut out, len_usize);
}

// fma_array has external linkage in the C source, so expose it as well.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    let n = len as usize;
    for i in 0..n {
        unsafe {
            let m1 = *mul1.add(i);
            let m2 = *mul2.add(i);
            let a = *add.add(i);
            *out.add(i) = m1.wrapping_mul(m2).wrapping_add(a);
        }
    }
}
