use std::ffi::c_int;

unsafe extern "C" {
    fn putchar(c: c_int) -> c_int;
}

fn hex_digit(nibble: u8) -> u8 {
    match nibble {
        0..=9 => b'0' + nibble,
        _ => b'a' + (nibble - 10),
    }
}

fn print_hex(bytes: &[u8]) {
    for &byte in bytes {
        let hi = hex_digit(byte >> 4);
        let lo = hex_digit(byte & 0x0f);

        unsafe {
            putchar(c_int::from(hi));
            putchar(c_int::from(lo));
        }
    }

    unsafe {
        putchar(c_int::from(b'\n'));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let raw = x.to_ne_bytes();
    print_hex(&raw);
}
