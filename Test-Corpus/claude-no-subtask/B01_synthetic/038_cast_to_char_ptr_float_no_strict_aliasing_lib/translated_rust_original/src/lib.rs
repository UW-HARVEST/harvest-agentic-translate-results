use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

fn print_hex(p: &[u8]) {
    for &b in p {
        unsafe {
            printf(b"%02x\0".as_ptr(), b as c_int);
        }
    }
    unsafe {
        printf(b"\n\0".as_ptr());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: f32) {
    let raw: [u8; 4] = x.to_ne_bytes();
    print_hex(&raw);
}
