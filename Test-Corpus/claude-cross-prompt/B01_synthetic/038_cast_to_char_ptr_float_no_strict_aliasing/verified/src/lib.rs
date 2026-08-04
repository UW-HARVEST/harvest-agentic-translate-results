use std::ffi::c_char;
use std::os::raw::{c_float, c_int, c_uchar};

fn print_hex(p: *const c_uchar, len: c_int) {
    let fmt = b"%02x\0".as_ptr() as *const c_char;
    let nl = b"\n\0".as_ptr() as *const c_char;
    unsafe {
        for i in 0..len {
            let byte = *p.offset(i as isize) as c_int;
            libc::printf(fmt, byte);
        }
        libc::printf(nl);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_float) {
    let mut raw: [u8; 4] = [0; 4];
    let bytes = x.to_ne_bytes();
    raw.copy_from_slice(&bytes);
    print_hex(raw.as_ptr() as *const c_uchar, raw.len() as c_int);
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> c_int {
    let mut x: c_float = 0.0;
    let fmt = b"%f\0".as_ptr() as *const c_char;
    unsafe {
        libc::scanf(fmt, &mut x as *mut c_float);
    }
    driver(x);
    0
}
