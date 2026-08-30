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

//! Rust translation of `c_src/src/driver.c`.
//!
//! The C original is:
//!
//! ```c
//! typedef union { uint64_t x; double f; } raw_double_t;
//!
//! void driver(double f) {
//!     raw_double_t u = {.f = f};
//!     printf("%llx %a %.4f\n", u.x, f, f);
//! }
//! ```
//!
//! Output has to be byte-identical to glibc's `printf`, so the three
//! conversions are reproduced explicitly:
//!
//! * `%llx`  - the raw bits of the double, lowercase hex, no padding.
//! * `%a`    - glibc's hex float form: `[-]0xh[.hhh]p{+,-}d`, exact mantissa
//!             with trailing zeros stripped; subnormals keep the leading `0`
//!             digit and a `p-1022` exponent; zero prints as `0x0p+0`.
//! * `%.4f`  - fixed notation, exact decimal expansion, ties-to-even
//!             (matching the default FE_TONEAREST rounding glibc uses).
//!
//! Non-finite values print as `inf` / `-inf` / `nan` / `-nan` for both float
//! conversions, following glibc (the sign bit of a NaN is honoured).

use std::ffi::{c_char, c_double, c_int};
use std::fmt::Write as _;

unsafe extern "C" {
    /// glibc `printf`. Used so the output goes through exactly the same
    /// `FILE *stdout` (and therefore the same buffering) as the C original.
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
}

const SIGN_MASK: u64 = 0x8000_0000_0000_0000;
const EXP_MASK: u64 = 0x7ff0_0000_0000_0000;
const MANT_MASK: u64 = 0x000f_ffff_ffff_ffff;
/// Number of hex digits in a double's 52-bit trailing significand.
const MANT_HEX_DIGITS: usize = 13;
/// Unbiased exponent used by glibc when printing subnormals with `%a`.
const SUBNORMAL_EXP: i32 = -1022;
const EXP_BIAS: i32 = 1023;

/// `"-"` when the sign bit is set, `""` otherwise. Note this is the raw sign
/// bit, so `-0.0` and negative NaNs get a sign, just like in glibc.
fn sign_str(bits: u64) -> &'static str {
    if bits & SIGN_MASK != 0 { "-" } else { "" }
}

/// glibc's spelling of a non-finite value for `%a` and `%f`.
/// Returns `None` for finite values.
fn non_finite_str(bits: u64) -> Option<&'static str> {
    if bits & EXP_MASK != EXP_MASK {
        return None;
    }
    let neg = bits & SIGN_MASK != 0;
    let is_nan = bits & MANT_MASK != 0;
    Some(match (is_nan, neg) {
        (false, false) => "inf",
        (false, true) => "-inf",
        (true, false) => "nan",
        (true, true) => "-nan",
    })
}

/// Renders `bits` the way glibc's `%a` conversion does (no precision given, so
/// the shortest exact hex significand is used).
fn format_hex_float(bits: u64) -> String {
    if let Some(s) = non_finite_str(bits) {
        return s.to_string();
    }

    let sign = sign_str(bits);
    let biased_exp = ((bits & EXP_MASK) >> 52) as i32;
    let mantissa = bits & MANT_MASK;

    // Zero is special-cased: glibc prints `0x0p+0`, not `0x0p-1022`.
    if biased_exp == 0 && mantissa == 0 {
        return format!("{sign}0x0p+0");
    }

    // Subnormals are *not* normalised by glibc: the leading digit stays 0 and
    // the exponent is pinned at -1022.
    let (leading_digit, exponent) = if biased_exp == 0 {
        (0u32, SUBNORMAL_EXP)
    } else {
        (1u32, biased_exp - EXP_BIAS)
    };

    // Exact significand, trailing zero nibbles dropped.
    let mut frac = format!("{mantissa:0width$x}", width = MANT_HEX_DIGITS);
    while frac.ends_with('0') {
        frac.pop();
    }

    let mut out = String::with_capacity(24);
    let _ = write!(out, "{sign}0x{leading_digit}");
    if !frac.is_empty() {
        let _ = write!(out, ".{frac}");
    }
    let _ = write!(out, "p{exponent:+}");
    out
}

/// Renders `f` the way glibc's `%.4f` conversion does.
///
/// Rust's `{:.4}` formatting is exact and breaks ties to even, which is what
/// glibc produces under the default rounding mode, so it can be reused for all
/// finite values. Only the non-finite spellings differ (`NaN` vs `nan`, and
/// Rust drops the sign of a negative NaN).
fn format_fixed4(f: c_double) -> String {
    match non_finite_str(f.to_bits()) {
        Some(s) => s.to_string(),
        None => format!("{f:.4}"),
    }
}

/// Translation of the C `driver` function.
///
/// The C code punned the double through a union to read its raw bits; here
/// `f64::to_bits` does the same thing.
///
/// # Safety
/// Callable from C with the signature `void driver(double)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(f: c_double) {
    let bits = f.to_bits();

    let line = format!(
        "{:x} {} {}\n",
        bits,
        format_hex_float(bits),
        format_fixed4(f)
    );

    // `%s` keeps the payload out of printf's format parser.
    let mut buf: Vec<u8> = Vec::with_capacity(line.len() + 1);
    buf.extend_from_slice(line.as_bytes());
    buf.push(0);
    unsafe {
        c_printf(c"%s".as_ptr(), buf.as_ptr() as *const c_char);
    }
}
