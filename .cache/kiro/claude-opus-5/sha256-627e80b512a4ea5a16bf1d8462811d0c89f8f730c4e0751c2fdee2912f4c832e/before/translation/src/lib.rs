// Rust translation of c_src/src/driver.c
//
// Original copyright 2025 MIT Lincoln Laboratory (MIT license, see c_src).
//
// Behavior is preserved byte-for-byte: the four bytes of the incoming `float`
// are dumped as lowercase, zero-padded, two-digit hex values followed by a
// newline. Output goes through C `printf` so that stdout buffering and
// interleaving with any C caller's own output are identical to the original.

use std::ffi::{c_char, c_float, c_int, c_uchar};

unsafe extern "C" {
    #[link_name = "printf"]
    unsafe fn c_printf(format: *const c_char, ...) -> c_int;
}

/// `static void print_hex(unsigned char *p, int len)`
///
/// Kept private, exactly as in the C source.
fn print_hex(p: &[c_uchar], len: c_int) {
    let mut i: c_int = 0;
    while i < len {
        // C: printf("%02x", p[i]);  — `unsigned char` promotes to `int`.
        unsafe {
            c_printf(c"%02x".as_ptr(), p[i as usize] as c_int);
        }
        i += 1;
    }
    unsafe {
        c_printf(c"\n".as_ptr());
    }
}

/// `void driver(float x)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(x: c_float) {
    // char raw[sizeof(x)];
    // memcpy(raw, &x, sizeof(x));
    let raw: [c_uchar; core::mem::size_of::<c_float>()] = x.to_ne_bytes();

    // print_hex((unsigned char *)raw, sizeof(raw));
    print_hex(&raw, raw.len() as c_int);
}
