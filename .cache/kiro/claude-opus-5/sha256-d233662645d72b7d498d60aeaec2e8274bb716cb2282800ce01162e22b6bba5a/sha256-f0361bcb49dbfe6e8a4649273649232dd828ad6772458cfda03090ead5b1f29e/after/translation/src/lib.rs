// Rust translation of the C library in c_src/.
//
// Original C library (c_src/):
//   * c_src/include/driver.h -- public header, declares `void driver(int x);`
//   * c_src/src/driver.c     -- the single translation unit
//
// The C build (c_src/CMakeLists.txt) compiles src/driver.c into a shared
// library whose only exported (defined) dynamic symbol is `driver`. There are
// no namespace-renaming preprocessor macros in the public header, so the final
// linker symbol is plainly `driver`.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

// The C code uses `printf("%d\n", y)` from <stdio.h>. We call the very same
// libc function rather than Rust's own `print!`/`std::io::stdout`, so that the
// emitted bytes *and* the stdio buffering/flush semantics (line buffered on a
// tty, fully buffered when redirected, flushed at exit by libc) are identical
// to the original library. This also keeps output correctly interleaved with
// any stdio writes performed by a C caller that loads this shared object.
unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Translation of:
///
/// ```c
/// void driver(int x) {
///     register int y = 2*x;
///     y += 300;
///     printf("%d\n", y);
/// }
/// ```
///
/// `register` is only a (long-obsolete) storage-class hint and has no bearing
/// on observable behaviour, so it is not modelled.
///
/// The arithmetic is performed with wrapping semantics on `i32`: signed
/// overflow is undefined behaviour in C, but the C library as actually compiled
/// (two's-complement `lea`/`add` on the target) wraps, so `wrapping_*` faithfully
/// reproduces the original behaviour for every input, including `INT_MIN` and
/// values near `INT_MAX`, instead of panicking as Rust's checked arithmetic
/// would in a debug build. No bug is "fixed" here: the observable result matches
/// the C.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut y: c_int = (x as i32).wrapping_mul(2);
    y = y.wrapping_add(300);

    // "%d\n\0" -- exactly the format string used by the C source.
    unsafe {
        printf(c"%d\n".as_ptr(), y);
    }
}
