// Rust translation of c_src/src/driver.c (public header: c_src/include/driver.h).
//
// Original copyright notice from the C sources:
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use core::ffi::{c_char, c_int};
use core::sync::atomic::{AtomicI32, Ordering};

unsafe extern "C" {
    /// C `printf` from libc. Used (rather than Rust's own `std::io::stdout`)
    /// so that emitted bytes *and* stdio buffering behaviour match the
    /// original C library exactly.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Print a fixed, NUL-terminated byte string through C `printf` using a `%s`
/// format, mirroring `printf("literal\n")` in the C source.
///
/// `s` must contain no interior NUL and must end with a NUL byte.
fn c_print(s: &[u8]) {
    debug_assert_eq!(s.last(), Some(&0), "string must be NUL terminated");
    // SAFETY: `s` is a NUL-terminated byte string and "%s\0" is a valid
    // format string expecting exactly one `*const c_char` argument.
    unsafe {
        printf(c"%s".as_ptr(), s.as_ptr() as *const c_char);
    }
}

/// Translation of the C file-scope `static int y = 123;`.
///
/// The C original is a plain (non-atomic) mutable global; an atomic is used
/// here to express the same mutable-global semantics without `static mut`.
/// `Relaxed` ordering is used because, as in the C code, all accesses happen
/// on the caller's thread.
static Y: AtomicI32 = AtomicI32::new(123);

/// Translation of the C `static int multi_stage(int x, int z)`.
///
/// This function is `static` in C, so it must NOT be exported from the
/// shared object; it stays a private Rust function.
///
/// The order of the validation checks and the exact messages are preserved
/// verbatim, including the C original's `goto fail` fall-through which prints
/// `"Operation failed"` only on the error paths.
fn multi_stage(x: c_int, z: c_int) -> c_int {
    let mut result: c_int = 0;

    // Mirrors the C `goto fail;` targets: each failing check sets `result`
    // and jumps to the shared failure epilogue.
    loop {
        if x != 1 {
            c_print(b"Error: x != 1\n\0");
            result = 1;
            break; // goto fail
        }

        if Y.load(Ordering::Relaxed) != 2 {
            c_print(b"Error: x == 1 but y != 2\n\0");
            result = 2;
            break; // goto fail
        }

        if z != 3 {
            c_print(b"Error: x == 1 and y == 2, but z != 3\n\0");
            result = 3;
            break; // goto fail
        }

        c_print(b"Ok!\n\0");
        return result;
    }

    // fail:
    c_print(b"Operation failed\n\0");
    result
}

/// Translation of the public C entry point:
///
/// ```c
/// void driver(int x, int local_y, int z);
/// ```
///
/// Assigns `local_y` to the file-scope `y`, runs the staged validation, then
/// prints the resulting status code.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, local_y: c_int, z: c_int) {
    Y.store(local_y, Ordering::Relaxed);
    let result = multi_stage(x, z);
    // SAFETY: "Result: %d\n\0" is a valid format string expecting exactly one
    // `c_int` argument, which is what is passed.
    unsafe {
        printf(c"Result: %d\n".as_ptr(), result);
    }
}
