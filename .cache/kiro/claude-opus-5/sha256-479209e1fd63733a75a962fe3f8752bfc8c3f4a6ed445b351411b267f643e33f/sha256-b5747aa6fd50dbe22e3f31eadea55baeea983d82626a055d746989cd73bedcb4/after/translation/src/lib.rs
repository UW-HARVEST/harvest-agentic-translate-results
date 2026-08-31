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

use std::ffi::{CString, c_char, c_int};

unsafe extern "C" {
    /// C `printf`, used so that the library shares glibc's `stdout` stream and
    /// therefore its buffering / interleaving behaviour, exactly like the C
    /// original did.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

const SIGN_MASK: u64 = 0x8000_0000_0000_0000;
const EXP_MASK: u64 = 0x7ff0_0000_0000_0000;
const MANT_MASK: u64 = 0x000f_ffff_ffff_ffff;

/// Reproduce glibc's `%a` conversion for a `double`.
///
/// glibc emits `[-]0xh[.hhh...]p{+,-}d`, where the leading hex digit is `1` for
/// normal values and `0` for subnormals (whose exponent stays pinned at
/// `-1022`), zero is printed as `0x0p+0`, trailing zeros of the fraction are
/// dropped, and the radix point is omitted when nothing follows it.
fn format_hex_float(bits: u64) -> String {
    let sign = if bits & SIGN_MASK != 0 { "-" } else { "" };
    let exp_field = ((bits & EXP_MASK) >> 52) as i32;
    let mantissa = bits & MANT_MASK;

    // Infinities and NaNs: glibc ignores the payload entirely.
    if exp_field == 0x7ff {
        let kind = if mantissa == 0 { "inf" } else { "nan" };
        return format!("{sign}{kind}");
    }

    // Both zeros are printed with a +0 exponent rather than -1022.
    if bits & !SIGN_MASK == 0 {
        return format!("{sign}0x0p+0");
    }

    let (leading_digit, exponent) = if exp_field == 0 {
        (0u8, -1022i32) // subnormal
    } else {
        (1u8, exp_field - 1023) // normal
    };

    // 52 mantissa bits == 13 hex digits; trailing zeros are not printed.
    let digits = format!("{mantissa:013x}");
    let fraction = digits.trim_end_matches('0');
    let radix_part = if fraction.is_empty() {
        String::new()
    } else {
        format!(".{fraction}")
    };

    let (exp_sign, exp_abs) = if exponent < 0 {
        ('-', -(exponent as i64))
    } else {
        ('+', exponent as i64)
    };

    format!("{sign}0x{leading_digit}{radix_part}p{exp_sign}{exp_abs}")
}

/// Reproduce glibc's `%.4f` conversion for a `double`.
///
/// Rust's `{:.4}` already produces the exact, correctly rounded (round-half-to-
/// even) decimal expansion that glibc produces, including the sign of negative
/// zero; only the non-finite spellings differ (`NaN` vs `nan`/`-nan`).
fn format_fixed_4(f: f64) -> String {
    if f.is_nan() {
        return if f.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if f.is_infinite() {
        return if f.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    format!("{f:.4}")
}

/// `void driver(double f)` — prints `"%llx %a %.4f\n"` for the raw bits of `f`
/// and `f` itself.
#[unsafe(no_mangle)]
pub extern "C" fn driver(f: f64) {
    // The C code type-puns the double through a union to read its raw bits.
    let raw: u64 = f.to_bits();

    let line = format!(
        "{:x} {} {}\n",
        raw,
        format_hex_float(raw),
        format_fixed_4(f)
    );

    // Safety: `line` never contains an interior NUL, so the CString conversion
    // cannot fail, and we hand a valid NUL-terminated pointer to printf.
    let c_line = match CString::new(line) {
        Ok(s) => s,
        Err(_) => return,
    };
    unsafe {
        printf(c"%s".as_ptr(), c_line.as_ptr());
    }
}
