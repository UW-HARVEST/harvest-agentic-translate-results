use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    for i in 0..len as isize {
        unsafe {
            let m1 = *mul1.offset(i);
            let m2 = *mul2.offset(i);
            let a = *add.offset(i);
            *out.offset(i) = m1.wrapping_mul(m2).wrapping_add(a);
        }
    }
}

fn inner(out: *mut c_int, len: c_int) {
    fma_array(out, out, out, out, len);
    for i in 0..len as isize {
        unsafe {
            println!("{}", *out.offset(i));
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(data: *const c_int, len: c_int) {
    if len <= 0 {
        return;
    }
    let mut out = vec![0; len as usize];
    unsafe {
        std::ptr::copy_nonoverlapping(data, out.as_mut_ptr(), len as usize);
    }
    inner(out.as_mut_ptr(), len);
}
