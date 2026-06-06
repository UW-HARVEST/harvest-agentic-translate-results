use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

fn inner(out: &mut [i32], len: usize) {
    // fma_array(out, out, out, out, len) — in C this aliases all four pointers.
    // Reproduce exactly: out[i] = out[i]*out[i] + out[i]
    for i in 0..len {
        let v = out[i];
        out[i] = v.wrapping_mul(v).wrapping_add(v);
    }
    let _ = fma_array; // silence unused warning if optimized away
    for i in 0..len {
        unsafe {
            printf(b"%d\n\0".as_ptr(), out[i] as c_int);
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
