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

//! Rust translation of `c_src/src/main.c`.
//!
//! Takes two arguments, a base and an exponent, and prints base^exponent.
//!
//! The C original leans on three pieces of libc behavior that have no faithful
//! pure-Rust equivalent, so those are reached through FFI:
//!
//! * `strtod` - accepts leading whitespace, `+`/`-`, hex floats (`0x10`),
//!   `inf`/`infinity`/`nan(...)`, reports the first unconsumed character via
//!   `endptr`, and sets `ERANGE` on overflow *and* on gradual underflow.
//!   `str::parse::<f64>()` matches none of that.
//! * `pow` - the `EDOM` / `ERANGE` reporting the C code branches on is a libm
//!   side effect, not a property of the returned value.
//! * `errno` - read after each of the above, in the same order.
//!
//! Everything else (argument handling, formatting, output) is safe Rust.

use std::ffi::CString;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;

// Linux/glibc values from <errno.h>. The C code compares `errno` against these
// after `strtod` and `pow`.
const EDOM: i32 = 33;
const ERANGE: i32 = 34;

#[link(name = "m")]
extern "C" {
    fn strtod(nptr: *const std::os::raw::c_char, endptr: *mut *mut std::os::raw::c_char) -> f64;
    fn pow(x: f64, y: f64) -> f64;
    /// glibc's `errno` is a macro expanding to `*__errno_location()`.
    fn __errno_location() -> *mut i32;
}

fn errno_get() -> i32 {
    // SAFETY: `__errno_location` always returns a valid pointer to the calling
    // thread's `errno` slot.
    unsafe { *__errno_location() }
}

fn errno_clear() {
    // SAFETY: as above; writing 0 is what `errno = 0;` compiles to.
    unsafe { *__errno_location() = 0 }
}

/// Result of a C `strtod(s, &endptr)` call.
struct StrtodResult {
    value: f64,
    /// True when `*endptr != '\0'`, i.e. trailing characters were left over.
    /// A string that fails to convert entirely leaves `endptr == nptr`, so an
    /// empty input yields `false` here and a value of `0.0` - the C program
    /// silently accepts `""` as zero, and so does this.
    has_trailing: bool,
    errno: i32,
}

/// Calls libc `strtod` on `bytes` with `errno` cleared first, mirroring
///
/// ```c
/// errno = 0;
/// double v = strtod(arg, &endptr);
/// ```
fn c_strtod(bytes: &[u8]) -> StrtodResult {
    // Arguments delivered through `execve` cannot contain interior NULs, but if
    // one somehow does, C would only ever see the bytes up to it.
    let truncated = match bytes.iter().position(|&b| b == 0) {
        Some(i) => &bytes[..i],
        None => bytes,
    };
    let cstr = CString::new(truncated).expect("NUL bytes already stripped");
    let nptr = cstr.as_ptr();
    let mut endptr: *mut std::os::raw::c_char = std::ptr::null_mut();

    errno_clear();
    // SAFETY: `nptr` is a valid NUL-terminated string that outlives the call,
    // and `endptr` is a valid writable slot for the out-parameter.
    let value = unsafe { strtod(nptr, &mut endptr) };
    let errno = errno_get();

    // `*endptr != '\0'` without dereferencing a raw pointer: `endptr` always
    // lands within `cstr` (inclusive of its terminator), so the byte offset
    // indexes the buffer we still own.
    let offset = (endptr as usize) - (nptr as usize);
    let has_trailing = cstr.as_bytes().get(offset).is_some_and(|&b| b != 0);

    StrtodResult {
        value,
        has_trailing,
        errno,
    }
}

/// Calls libc `pow` with `errno` cleared first, returning `(result, errno)`.
fn c_pow(base: f64, exponent: f64) -> (f64, i32) {
    errno_clear();
    // SAFETY: `pow` is a pure computation over two `double`s.
    let result = unsafe { pow(base, exponent) };
    (result, errno_get())
}

/// Renders `v` the way glibc's `printf("%.2f", v)` does.
///
/// Rust's `{:.2}` agrees with glibc on every finite value and on the
/// infinities; it differs only for NaN, which Rust prints as `NaN` and glibc
/// prints as `nan` / `-nan` depending on the sign bit.
fn format_f2(v: f64) -> String {
    if v.is_nan() {
        if v.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        }
    } else {
        format!("{:.2}", v)
    }
}

fn stderr_write(bytes: &[u8]) {
    let mut err = std::io::stderr();
    // C's `fprintf` to unbuffered stderr ignores write failures too.
    let _ = err.write_all(bytes);
    let _ = err.flush();
}

fn stdout_write(bytes: &[u8]) {
    let mut out = std::io::stdout();
    let _ = out.write_all(bytes);
    let _ = out.flush();
}

fn main() {
    // Argument bytes, not `String`s: C's `argv` is arbitrary bytes and the
    // error messages echo them back verbatim via `%s`.
    let argv: Vec<Vec<u8>> = std::env::args_os()
        .map(|a| a.as_bytes().to_vec())
        .collect();
    let argc = argv.len();

    if argc != 3 {
        // `printf("%s", NULL)` renders as `(null)` on glibc, which is what a
        // zero-length `argv` would produce here.
        let prog: &[u8] = argv.first().map_or(&b"(null)"[..], |a| a.as_slice());
        let mut msg = b"Usage: ".to_vec();
        msg.extend_from_slice(prog);
        msg.extend_from_slice(b" base exponent\n");
        stderr_write(&msg);
        std::process::exit(1);
    }

    // Convert base
    let base_arg = &argv[1];
    let base_conv = c_strtod(base_arg);
    if base_conv.errno == ERANGE {
        let mut msg = b"Range error while converting base '".to_vec();
        msg.extend_from_slice(base_arg);
        msg.extend_from_slice(b"'\n");
        stderr_write(&msg);
        std::process::exit(1);
    } else if base_conv.has_trailing {
        let mut msg = b"Invalid numeric input for base: '".to_vec();
        msg.extend_from_slice(base_arg);
        msg.extend_from_slice(b"'\n");
        stderr_write(&msg);
        std::process::exit(1);
    }
    let base = base_conv.value;

    // Convert exponent
    let exponent_arg = &argv[2];
    let exponent_conv = c_strtod(exponent_arg);
    if exponent_conv.errno == ERANGE {
        let mut msg = b"Range error while converting exponent '".to_vec();
        msg.extend_from_slice(exponent_arg);
        msg.extend_from_slice(b"'\n");
        stderr_write(&msg);
        std::process::exit(1);
    } else if exponent_conv.has_trailing {
        let mut msg = b"Invalid numeric input for exponent: '".to_vec();
        msg.extend_from_slice(exponent_arg);
        msg.extend_from_slice(b"'\n");
        stderr_write(&msg);
        std::process::exit(1);
    }
    let exponent = exponent_conv.value;

    // Calculate power
    let (result, pow_errno) = c_pow(base, exponent);
    if pow_errno == EDOM {
        let msg = format!(
            "Domain error: pow({}, {}) is undefined in the real number domain.\n",
            format_f2(base),
            format_f2(exponent)
        );
        stderr_write(msg.as_bytes());
        std::process::exit(1);
    } else if pow_errno == ERANGE {
        let msg = format!(
            "Range error: pow({}, {}) caused overflow or underflow.\n",
            format_f2(base),
            format_f2(exponent)
        );
        stderr_write(msg.as_bytes());
        std::process::exit(1);
    }

    stdout_write(format!("Result: {}\n", format_f2(result)).as_bytes());
    std::process::exit(0);
}
