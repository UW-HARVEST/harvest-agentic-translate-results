//! Translation of `c_src/src/hello.c` (public header: `c_src/include/hello.h`).
//!
//! ```c
//! int helloworld() {
//!     printf("Hello World!\n");
//!     return 0;
//! }
//! ```

use core::ffi::{c_char, c_int};

unsafe extern "C" {
    /// C standard library `printf`, used so that output goes through exactly the
    /// same `stdout` FILE stream (and buffering behaviour) as the original C
    /// library did.  This keeps the emitted bytes, and their interleaving with
    /// any other C stdio output, byte-for-byte identical.
    #[link_name = "printf"]
    unsafe fn c_printf(format: *const c_char, ...) -> c_int;
}

/// `int helloworld()` — prints `Hello World!\n` to stdout and returns 0.
///
/// The C original ignores `printf`'s return value and unconditionally returns 0.
#[unsafe(no_mangle)]
pub extern "C" fn helloworld() -> c_int {
    // "Hello World!\n\0" as a NUL-terminated C string literal.
    const FORMAT: &[u8; 14] = b"Hello World!\n\0";

    unsafe {
        c_printf(FORMAT.as_ptr() as *const c_char);
    }

    0
}
