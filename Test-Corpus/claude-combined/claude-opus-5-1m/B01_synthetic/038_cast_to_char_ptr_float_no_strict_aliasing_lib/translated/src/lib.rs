use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

fn print_hex(p: &[u8]) {
    let fmt = b"%02x\0";
    let nl = b"\n\0";
    for &byte in p {
        unsafe {
            printf(fmt.as_ptr(), byte as c_int);
        }
    }
    unsafe {
        printf(nl.as_ptr());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: f32) {
    let raw: [u8; 4] = x.to_ne_bytes();
    print_hex(&raw);
}
