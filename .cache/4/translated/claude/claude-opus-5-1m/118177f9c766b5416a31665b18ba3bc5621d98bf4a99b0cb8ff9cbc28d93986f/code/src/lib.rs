// Rust translation of the C library in c_src/ (MIT Lincoln Laboratory `driver`).
//
// Public ABI exported by the C shared library (from `nm -D libdriver.so`):
//   T driver
//
// Everything else in the C source (`print_hex`) is `static`, i.e. private, and
// is therefore translated as a private Rust function.

use std::ffi::c_char;
use std::ffi::c_int;

unsafe extern "C" {
    // Use libc's printf so that stdout buffering, formatting and interleaving
    // with any other C output are byte-for-byte identical to the C library.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Translation of:
/// ```c
/// static void print_hex(unsigned char *p, int len) {
///     for (int i = 0; i < len; i++) {
///         printf("%02x", p[i]);
///     }
///     printf("\n");
/// }
/// ```
fn print_hex(p: *const u8, len: c_int) {
    let mut i: c_int = 0;
    while i < len {
        // `p[i]` is an `unsigned char` promoted to `int` for the varargs call.
        let byte = unsafe { *p.offset(i as isize) };
        unsafe {
            printf(c"%02x".as_ptr(), byte as c_int);
        }
        i += 1;
    }
    unsafe {
        printf(c"\n".as_ptr());
    }
}

/// Translation of:
/// ```c
/// void driver(float x) {
///     print_hex((unsigned char *)&x, sizeof(x));
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: f32) {
    // The C code reinterprets the bytes of the (stack-stored) `float`
    // parameter, i.e. its native-endian object representation.
    let bytes: [u8; 4] = x.to_ne_bytes();
    print_hex(bytes.as_ptr(), core::mem::size_of::<f32>() as c_int);
}
