// printf "%.{precision}g" formatting for f64.
// Mimics C99 semantics:
//   - Style 'f' or 'e' is chosen based on the exponent X that style 'e' would use.
//   - Let P = precision (or 1 if precision is 0).
//   - If P > X >= -4, use style f with P-(X+1) digits after the decimal point.
//   - Else use style e with P-1 digits after the decimal point.
//   - Without the '#' flag, trailing zeros after the decimal point are trimmed,
//     and the decimal point is removed if no fractional digits remain.
//   - Exponent in style e has at least 2 digits with explicit sign (e.g. e+03, e-04, e+100).

pub fn format_g(x: f64, precision: usize) -> String {
    if x.is_nan() {
        // glibc prints "-nan" when the NaN's sign bit is set.
        if x.is_sign_negative() {
            return "-nan".to_string();
        }
        return "nan".to_string();
    }
    if x.is_infinite() {
        return if x.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }

    let p = if precision == 0 { 1 } else { precision };

    if x == 0.0 {
        // Both +0.0 and -0.0 — preserve sign, no fractional part with %g (no '#').
        if x.is_sign_negative() {
            return "-0".to_string();
        }
        return "0".to_string();
    }

    // Determine the exponent X that scientific style would use, after rounding to P digits.
    // Rust's `{:.(P-1)e}` rounds half-to-even, matching glibc's default rounding.
    let sci = format!("{:.*e}", p - 1, x);
    let e_pos = sci.find('e').expect("scientific format must contain 'e'");
    let exp_str = &sci[e_pos + 1..];
    let x_exp: i32 = exp_str.parse().expect("exponent should parse");

    let p_i = p as i32;

    let (use_f, digits_after): (bool, usize) = if p_i > x_exp && x_exp >= -4 {
        // Style f with P - (X + 1) digits after the decimal point.
        let d = p_i - (x_exp + 1);
        (true, d.max(0) as usize)
    } else {
        // Style e with P - 1 digits after the decimal point.
        let d = p_i - 1;
        (false, d.max(0) as usize)
    };

    let mut result = if use_f {
        format!("{:.*}", digits_after, x)
    } else {
        let raw = format!("{:.*e}", digits_after, x);
        rust_e_to_c_e(&raw)
    };

    // Trim trailing zeros after the decimal point (no '#' flag).
    if use_f {
        if result.contains('.') {
            while result.ends_with('0') {
                result.pop();
            }
            if result.ends_with('.') {
                result.pop();
            }
        }
    } else {
        // Split mantissa and exponent at 'e'.
        let e_pos = result.find('e').expect("e-form result must contain 'e'");
        let mant = result[..e_pos].to_string();
        let exp_part = result[e_pos..].to_string();
        let mut mant = mant;
        if mant.contains('.') {
            while mant.ends_with('0') {
                mant.pop();
            }
            if mant.ends_with('.') {
                mant.pop();
            }
        }
        result = mant + &exp_part;
    }

    result
}

/// Convert Rust's `Ne±D` (no leading zeros, no '+' sign) into C-printf style
/// `Ne[+|-]DD` (sign always shown, exponent at least 2 digits).
fn rust_e_to_c_e(s: &str) -> String {
    let e_pos = s.find('e').expect("must contain 'e'");
    let mant = &s[..e_pos];
    let exp_str = &s[e_pos + 1..];
    let exp: i32 = exp_str.parse().expect("exponent should parse");
    let sign = if exp < 0 { '-' } else { '+' };
    let abs = exp.unsigned_abs();
    if abs < 10 {
        format!("{}e{}0{}", mant, sign, abs)
    } else {
        format!("{}e{}{}", mant, sign, abs)
    }
}
