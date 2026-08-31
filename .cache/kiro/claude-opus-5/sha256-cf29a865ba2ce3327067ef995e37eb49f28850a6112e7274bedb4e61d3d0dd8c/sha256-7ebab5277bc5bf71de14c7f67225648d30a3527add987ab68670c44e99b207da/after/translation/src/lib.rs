// Rust translation of c_src/src/driver.c
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

use std::ffi::{c_char, c_int, c_ulonglong};

unsafe extern "C" {
    /// C `printf`. Using the C library's own implementation — rather than
    /// reimplementing its conversions in Rust — is what makes this translation
    /// byte-identical to the original:
    ///
    /// * the library shares glibc's `stdout`, so buffering and interleaving with
    ///   the rest of the host process match the C version exactly;
    /// * `%a` and `%f` emit the decimal point taken from the host's `LC_NUMERIC`
    ///   locale (`1,5000` rather than `1.5000` under, say, `de_DE`), which the
    ///   C code inherits from whatever called it;
    /// * `%f` rounds according to the current floating-point rounding direction
    ///   (`fesetround`), not unconditionally to nearest-even;
    /// * non-finite values use glibc's spellings (`inf`, `-inf`, `nan`, `-nan`).
    ///
    /// Reproducing all of that independently would mean reimplementing glibc's
    /// `vfprintf`, so the conversions are delegated to it verbatim.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// The C translation unit type-puns the `double` through a union to read its
/// raw bits:
///
/// ```c
/// typedef union {
///     uint64_t x;
///     double f;
/// } raw_double_t;
/// ```
///
/// `f64::to_bits` is the exact equivalent for IEEE-754 `binary64`, which is the
/// representation of `double` on every target this library builds for.
#[inline]
fn raw_bits(f: f64) -> u64 {
    f.to_bits()
}

/// `void driver(double f)` — prints `"%llx %a %.4f\n"` with the raw bits of `f`
/// followed by `f` twice.
#[unsafe(no_mangle)]
pub extern "C" fn driver(f: f64) {
    let raw: u64 = raw_bits(f);

    // Safety: the format string is a NUL-terminated literal and the variadic
    // arguments match its conversions exactly — `%llx` takes an
    // `unsigned long long`, and both `%a` and `%.4f` take a `double` (which is
    // how a `f64` is passed through C's default argument promotions).
    unsafe {
        printf(
            c"%llx %a %.4f\n".as_ptr(),
            raw as c_ulonglong,
            f as core::ffi::c_double,
            f as core::ffi::c_double,
        );
    }
}
