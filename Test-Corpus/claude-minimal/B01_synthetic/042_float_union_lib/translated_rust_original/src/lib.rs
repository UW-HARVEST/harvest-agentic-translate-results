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

/// Formats a `f64` value using C's `%a` hexadecimal floating-point format.
///
/// This mirrors the behavior of glibc's `printf("%a", f)`, producing strings
/// such as `0x1.999999999999ap-4` for `0.1` and `0x1p+0` for `1.0`.
fn format_hex_float(f: f64) -> String {
    let bits = f.to_bits();
    let sign = bits >> 63;
    let exp_bits = ((bits >> 52) & 0x7ff) as i64;
    let mantissa = bits & 0x000f_ffff_ffff_ffff;

    let sign_str = if sign == 1 { "-" } else { "" };

    // Special values: infinity and NaN.
    if exp_bits == 0x7ff {
        if mantissa == 0 {
            return format!("{}inf", sign_str);
        } else {
            return "nan".to_string();
        }
    }

    let leading: char;
    let exp: i64;
    if exp_bits == 0 {
        if mantissa == 0 {
            // Signed zero.
            return format!("{}0x0p+0", sign_str);
        }
        // Subnormal numbers.
        leading = '0';
        exp = -1022;
    } else {
        // Normal numbers.
        leading = '1';
        exp = exp_bits - 1023;
    }

    // Format mantissa as 13 lowercase hex digits, then strip trailing zeros.
    let mut mantissa_str = format!("{:013x}", mantissa);
    while mantissa_str.ends_with('0') {
        mantissa_str.pop();
    }

    let exp_str = if exp >= 0 {
        format!("+{}", exp)
    } else {
        format!("{}", exp)
    };

    if mantissa_str.is_empty() {
        format!("{}0x{}p{}", sign_str, leading, exp_str)
    } else {
        format!("{}0x{}.{}p{}", sign_str, leading, mantissa_str, exp_str)
    }
}

/// Mirrors the C `driver` function: prints the raw bit pattern, hexadecimal
/// floating-point representation, and four-decimal value of the input.
pub fn driver(f: f64) {
    let x = f.to_bits();
    println!("{:x} {} {:.4}", x, format_hex_float(f), f);
}

/// C-callable shim so this library can be used as a `cdylib` drop-in.
#[no_mangle]
pub extern "C" fn driver_c(f: f64) {
    driver(f);
}
