use std::ffi::c_int;
use std::io::{self, Write};

#[unsafe(no_mangle)]
pub extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    for i in 0..len as usize {
        unsafe {
            let m1 = mul1.add(i).read();
            let m2 = mul2.add(i).read();
            let a = add.add(i).read();
            out.add(i).write(m1.wrapping_mul(m2).wrapping_add(a));
        }
    }
}

fn inner(out: *mut c_int, len: c_int) {
    fma_array(out, out, out, out, len);
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    for i in 0..len as usize {
        unsafe {
            let _ = writeln!(lock, "{}", out.add(i).read());
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(data: *const c_int, len: c_int) {
    let mut buf = vec![0i32; len as usize];
    unsafe {
        std::ptr::copy_nonoverlapping(data, buf.as_mut_ptr(), len as usize);
    }
    inner(buf.as_mut_ptr(), len);
}
