// Rust translation of c_src/src/driver.c
//
// Original C:
//     static void print_hex(unsigned char *p, int len) {
//         for (int i = 0; i < len; i++) {
//             printf("%02x", p[i]);
//         }
//         printf("\n");
//     }
//
//     void driver(float x) {
//         print_hex((unsigned char *)&x, sizeof(x));
//     }
//
// The output is the raw in-memory byte representation of the float argument,
// printed as lowercase, zero-padded, two-digit hex, followed by a newline.
// We call the platform's C `printf` directly so that stdout buffering and
// formatting behavior are identical to the original library.

use std::ffi::{c_char, c_int, c_uchar, c_uint};

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Format string `"%02x"` as a NUL-terminated C string.
const FMT_HEX: &[u8; 5] = b"%02x\0";
/// Format string `"\n"` as a NUL-terminated C string.
const FMT_NL: &[u8; 2] = b"\n\0";

/// Equivalent of the C file-local `print_hex`.
///
/// # Safety
/// `p` must point to at least `len` readable bytes when `len > 0`.
unsafe fn print_hex(p: *const c_uchar, len: c_int) {
    let mut i: c_int = 0;
    while i < len {
        // SAFETY: caller guarantees `p[0..len]` is readable.
        let byte = unsafe { *p.offset(i as isize) };
        unsafe {
            printf(FMT_HEX.as_ptr() as *const c_char, byte as c_uint);
        }
        i += 1;
    }
    unsafe {
        printf(FMT_NL.as_ptr() as *const c_char);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: f32) {
    // `sizeof(float)` bytes of the value's object representation.
    let bytes: [u8; std::mem::size_of::<f32>()] = x.to_ne_bytes();
    unsafe {
        print_hex(bytes.as_ptr() as *const c_uchar, bytes.len() as c_int);
    }
}
