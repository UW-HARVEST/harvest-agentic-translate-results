// Rust translation of c_src (MIT Lincoln Laboratory `driver` library).
//
// Original C:
//     static void print_hex(unsigned char *p, int len);
//     void driver(float x);
//
// The C implementation writes to stdout via `printf`, so this translation calls
// the very same libc `printf` in order to produce byte-identical output with
// identical stream-buffering behavior.

use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_uchar;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Translation of the C `static void print_hex(unsigned char *p, int len)`.
///
/// Not exported (it is `static` in the C source).
unsafe fn print_hex(p: *const c_uchar, len: c_int) {
    let mut i: c_int = 0;
    while i < len {
        // printf("%02x", p[i]);
        // `unsigned char` is promoted to `int` when passed as a variadic arg.
        printf(
            b"%02x\0".as_ptr() as *const c_char,
            *p.offset(i as isize) as c_int,
        );
        i += 1;
    }
    // printf("\n");
    printf(b"\n\0".as_ptr() as *const c_char);
}

/// Translation of the C `void driver(float x)`.
///
/// Dumps the raw object representation of the `float` argument as lowercase
/// hexadecimal, one byte at a time, followed by a newline.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: f32) {
    // print_hex((unsigned char *)&x, sizeof(x));
    let x = x;
    unsafe {
        print_hex(
            &x as *const f32 as *const c_uchar,
            core::mem::size_of::<f32>() as c_int,
        );
    }
}
