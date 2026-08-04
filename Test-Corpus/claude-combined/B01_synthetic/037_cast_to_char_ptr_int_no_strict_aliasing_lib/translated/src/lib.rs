use std::ffi::c_int;
use std::os::raw::c_uchar;

fn print_hex(p: *const c_uchar, len: c_int) {
    let fmt = b"%02x\0".as_ptr() as *const libc::c_char;
    let nl = b"\n\0".as_ptr() as *const libc::c_char;
    for i in 0..len {
        unsafe {
            libc::printf(fmt, *p.offset(i as isize) as c_int);
        }
    }
    unsafe {
        libc::printf(nl);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    // char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));
    let raw: [u8; std::mem::size_of::<c_int>()] = x.to_ne_bytes();
    print_hex(raw.as_ptr(), raw.len() as c_int);
}
