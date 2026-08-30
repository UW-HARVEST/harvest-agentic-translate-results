// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
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
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Rust translation of `c_src/src/pow.c`.
//!
//! The C code relies on three pieces of platform behaviour that must be
//! reproduced exactly in order to get byte-identical output:
//!
//! * the value *and* the `errno` side effect of the platform `pow(3)` from
//!   libm (results are not recomputed in Rust, libm is called directly);
//! * the `errno` slot itself (`__errno_location()` on glibc);
//! * `fprintf(stderr, ...)` with `%.2f` conversions, including stream
//!   buffering and glibc's exact float formatting (`inf`, `nan`, ...).
//!
//! Because of that, the error paths are emitted through libc's `fprintf`
//! rather than Rust's formatting machinery.

use core::ffi::{c_char, c_int, c_void};

// <errno.h> values on Linux.
const EDOM: c_int = 33;
const ERANGE: c_int = 34;

unsafe extern "C" {
    /// Platform `pow(3)`. Declared as a plain `extern "C"` function (instead of
    /// using `f64::powf`) so that the call is *not* lowered to the
    /// `llvm.pow.f64` intrinsic, which is treated as side-effect free and
    /// would therefore be allowed to drop the `errno` write.
    fn pow(base: f64, exponent: f64) -> f64;

    /// glibc's per-thread `errno` slot.
    fn __errno_location() -> *mut c_int;

    /// `FILE *stderr;` from glibc.
    static stderr: *mut c_void;

    #[allow(clashing_extern_declarations)]
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
}

#[inline]
fn errno_get() -> c_int {
    unsafe { *__errno_location() }
}

#[inline]
fn errno_set(value: c_int) {
    unsafe { *__errno_location() = value }
}

/// Takes two arguments, a base and an exponent, and returns base^exponent
#[unsafe(no_mangle)]
pub extern "C" fn my_pow(base: f64, exponent: f64) -> f64 {
    // Calculate power
    errno_set(0);
    let result = unsafe { pow(base, exponent) };
    // Keep the `errno` read strictly after the libm call.
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);

    let err = errno_get();
    if err == EDOM {
        unsafe {
            fprintf(
                stderr,
                c"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n"
                    .as_ptr(),
                base,
                exponent,
            );
        }
        return -1.0;
    } else if err == ERANGE {
        unsafe {
            fprintf(
                stderr,
                c"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n".as_ptr(),
                base,
                exponent,
            );
        }
        return -1.0;
    }

    result
}
