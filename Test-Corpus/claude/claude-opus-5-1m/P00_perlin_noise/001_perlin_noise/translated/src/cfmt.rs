//! Minimal re-implementation of the C `printf` conversions used by the program.

/// Formats `value` the way glibc's `printf("%.<precision>g", value)` does.
///
/// `%g` picks `%e` style when the decimal exponent is `< -4` or `>= precision`,
/// and `%f` style otherwise; in both cases trailing zeros (and a trailing
/// decimal point) are removed because the `#` flag is not used.
pub fn format_g(value: f64, precision: usize) -> String {
    if value.is_nan() {
        // glibc prints the sign of a NaN.
        return if value.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }

    // C: "if the precision is zero, it is taken as 1".
    let p = if precision == 0 { 1 } else { precision };

    // Rust's `{:.*e}` rounds exactly like glibc's `%e` (round-half-to-even on
    // exact decimal ties) and already normalises the exponent after a carry.
    let sci = format!("{:.*e}", p - 1, value);
    let (mantissa, exponent) = match sci.split_once('e') {
        Some(parts) => parts,
        None => (sci.as_str(), "0"),
    };
    let exp: i32 = exponent.parse().unwrap_or(0);

    if exp < -4 || exp >= p as i32 {
        let digits = trim_trailing_zeros(mantissa);
        let sign = if exp < 0 { '-' } else { '+' };
        format!("{}e{}{:02}", digits, sign, exp.unsigned_abs())
    } else {
        let frac_digits = (p as i32 - 1 - exp) as usize;
        let fixed = format!("{:.*}", frac_digits, value);
        trim_trailing_zeros(&fixed)
    }
}

/// Drops trailing zeros of a fractional part, then a bare trailing '.'.
fn trim_trailing_zeros(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    let trimmed = s.trim_end_matches('0');
    trimmed.trim_end_matches('.').to_string()
}
