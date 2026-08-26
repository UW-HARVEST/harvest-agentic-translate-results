//! Translation of `c_src/src/hello.c`.
//!
//! ```c
//! #include <stdio.h>
//!
//! #include "hello.h"
//!
//! int helloworld() {
//!     printf("Hello World!\n");
//!     return 0;
//! }
//! ```

use core::ffi::c_int;

use crate::cstdio::printf;

/// The single format/message literal from `hello.c`, NUL terminated exactly as
/// the C string literal is.
const HELLO_WORLD: &[u8; 14] = b"Hello World!\n\0";

/// `int helloworld();`
///
/// Prints `Hello World!` followed by a newline to `stdout` and returns `0`.
///
/// The C declaration in `hello.h` uses an empty (unprototyped) parameter list,
/// so callers pass no arguments; the ABI is a plain no-argument
/// `extern "C" fn() -> int`.
#[unsafe(no_mangle)]
pub extern "C" fn helloworld() -> c_int {
    // SAFETY: `HELLO_WORLD` is a NUL-terminated byte string with no conversion
    // specifiers, so `printf` reads no variadic arguments.
    unsafe {
        printf(HELLO_WORLD.as_ptr().cast());
    }
    0
}
