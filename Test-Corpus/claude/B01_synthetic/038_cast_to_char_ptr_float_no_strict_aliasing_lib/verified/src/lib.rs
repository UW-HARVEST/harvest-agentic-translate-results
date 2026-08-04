use std::ffi::c_int;

fn print_hex(p: &[u8]) {
    // Use libc::printf to match C stdio behavior exactly
    let fmt_byte = b"%02x\0".as_ptr() as *const i8;
    let fmt_nl = b"\n\0".as_ptr() as *const i8;
    for &b in p {
        unsafe {
            libc::printf(fmt_byte, b as c_int);
        }
    }
    unsafe {
        libc::printf(fmt_nl);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: f32) {
    let raw: [u8; 4] = x.to_ne_bytes();
    print_hex(&raw);
}
