//! Library exposing the C-compatible `driver` symbol.
//!
//! The C implementation writes bytes of `x` (host endian) as lowercase hex,
//! followed by a newline, to stdout via `printf`. We replicate this exactly,
//! including writing through libc's `printf` so buffering and stdout handling
//! match the C version byte-for-byte.

use std::os::raw::{c_char, c_int, c_uchar};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

unsafe fn print_hex(p: *const c_uchar, len: c_int) {
    let fmt_byte = b"%02x\0".as_ptr() as *const c_char;
    let fmt_nl = b"\n\0".as_ptr() as *const c_char;
    for i in 0..len {
        let b = *p.offset(i as isize) as c_int;
        printf(fmt_byte, b);
    }
    printf(fmt_nl);
}

/// C-compatible export: replicates `void driver(int x)` from main.c exactly.
#[no_mangle]
pub unsafe extern "C" fn driver(x: c_int) {
    // C does: char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));
    //         print_hex((unsigned char*)raw, sizeof(raw));
    let raw = (x as i32).to_ne_bytes();
    print_hex(raw.as_ptr(), raw.len() as c_int);
}

extern "C" {
    fn scanf(fmt: *const c_char, ...) -> c_int;
}

/// C-compatible export: replicates `int main()` from main.c exactly.
///
/// Reads an integer from stdin via scanf("%d", &x), defaulting to 0 if no
/// conversion happens, and calls driver(x).
///
/// Only emitted when building the cdylib, not when running unit tests
/// (where the test harness needs to provide its own `main`).
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn main() -> c_int {
    let mut x: c_int = 0;
    let fmt = b"%d\0".as_ptr() as *const c_char;
    scanf(fmt, &mut x as *mut c_int);
    driver(x);
    0
}
