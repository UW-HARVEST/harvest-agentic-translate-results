// Rust shared-library export that mirrors the C `driver` function so it can be
// loaded via `libloading` and compared byte-for-byte against the C .so build.
//
// The C implementation prints, via libc stdio, the raw little-endian bytes of
// the float as lower-case hexadecimal followed by a newline. We replicate that
// exactly using libc::printf so that captured stdout matches byte-for-byte.

use std::os::raw::{c_float, c_int, c_uchar};

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

unsafe fn print_hex(p: *const c_uchar, len: c_int) {
    let fmt_byte = b"%02x\0".as_ptr();
    for i in 0..len {
        let b: c_uchar = *p.offset(i as isize);
        printf(fmt_byte, b as c_int);
    }
    printf(b"\n\0".as_ptr());
}

#[no_mangle]
pub extern "C" fn driver(x: c_float) {
    let bytes = x.to_ne_bytes();
    unsafe {
        print_hex(bytes.as_ptr(), bytes.len() as c_int);
    }
}

extern "C" {
    fn scanf(fmt: *const u8, ...) -> c_int;
}


// Only define a `main` export when building the cdylib (not during the test
// binary, where Rust's test harness wants its own `main`).
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> c_int {
    let mut x: c_float = 0.0;
    unsafe {
        scanf(b"%f\0".as_ptr(), &mut x as *mut c_float);
    }
    driver(x);
    0
}
