use std::ffi::c_int;

unsafe extern "C" {
    fn putchar(c: c_int) -> c_int;
}

fn print_hex(bytes: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    for &byte in bytes {
        let hi = HEX[((byte >> 4) & 0x0f) as usize] as c_int;
        let lo = HEX[(byte & 0x0f) as usize] as c_int;

        unsafe {
            putchar(hi);
            putchar(lo);
        }
    }

    unsafe {
        putchar(b'\n' as c_int);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: f32) {
    let raw = x.to_ne_bytes();
    print_hex(&raw);
}
