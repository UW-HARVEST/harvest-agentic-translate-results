// Rust translation of the C library in c_src/.
//
// Public ABI (as exported by the C shared library, verified with `nm -D`):
//     T driver
//
// Source notes -- the original C is written with digraphs and the
// <iso646.h> alternative operator spellings:
//     `%:` == `#`,  `<%` == `{`,  `%>` == `}`
//     `bitor` == `|`,  `compl` == `~`
//
// so `src/driver.c` is, after translation of those spellings:
//
//     void driver(int x, int y) {
//         int result = x | ~y;
//         printf("%d", result);
//         puts("");
//     }

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

// Use the platform C library's stdio directly rather than Rust's `std::io`
// wrapper. This keeps the emitted bytes *and* the stream buffering behaviour
// identical to the C original (same `stdout` FILE object, same flush points),
// which matters when a host program mixes its own C stdio output with calls
// into this library.
extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn puts(s: *const c_char) -> c_int;
}

/// `void driver(int x, int y)`
///
/// Prints `x | ~y` as a decimal integer, followed by a newline.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    // `x bitor compl y` => `x | ~y`. Bitwise ops on a 32-bit two's-complement
    // `int` cannot overflow, so Rust's `i32` semantics match C exactly.
    let result: c_int = x | !y;

    unsafe {
        // printf("%d", result);
        printf(c"%d".as_ptr(), result);
        // puts("");  -- emits just the newline.
        puts(c"".as_ptr());
    }
}
