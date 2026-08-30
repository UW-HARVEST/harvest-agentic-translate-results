//! C `printf("%.9g", value)` for a `double` (the `float` result is promoted by
//! the C variadic call, so the formatting is done on the widened value).
//!
//! `%g` with precision P = 9: use `%e` style when the decimal exponent X
//! satisfies X < -4 or X >= P, otherwise `%f` style with precision P-1-X.
//! Trailing zeros are removed from the fractional part (no `#` flag), and a
//! trailing decimal point is dropped.

pub fn format_g9(v: f64) -> String {
    let sign = if v.is_sign_negative() { "-" } else { "" };

    if v.is_nan() {
        return format!("{}nan", sign);
    }
    if v.is_infinite() {
        return format!("{}inf", sign);
    }
    if v == 0.0 {
        return format!("{}0", sign);
    }

    // 9 significant digits, correctly rounded, plus the decimal exponent.
    let s = format!("{:.8e}", v.abs());
    let (mant, exp) = match s.split_once('e') {
        Some(pair) => pair,
        None => (s.as_str(), "0"),
    };
    let x: i32 = exp.parse().unwrap_or(0);
    let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    debug_assert_eq!(digits.len(), 9);

    const P: i32 = 9;
    if x < -4 || x >= P {
        let frac = digits[1..].trim_end_matches('0');
        let mut out = String::new();
        out.push_str(sign);
        out.push_str(&digits[..1]);
        if !frac.is_empty() {
            out.push('.');
            out.push_str(frac);
        }
        out.push('e');
        out.push(if x < 0 { '-' } else { '+' });
        let a = (x as i64).abs();
        if a < 10 {
            out.push('0');
        }
        out.push_str(&a.to_string());
        out
    } else {
        let (int_part, frac_part) = if x >= 0 {
            let k = (x + 1) as usize;
            (digits[..k].to_string(), digits[k..].to_string())
        } else {
            let zeros = "0".repeat((-x - 1) as usize);
            ("0".to_string(), format!("{}{}", zeros, digits))
        };
        let frac = frac_part.trim_end_matches('0');
        if frac.is_empty() {
            format!("{}{}", sign, int_part)
        } else {
            format!("{}{}.{}", sign, int_part, frac)
        }
    }
}
