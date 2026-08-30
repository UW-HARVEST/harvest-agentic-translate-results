//! C `printf("%.Ng", value)` emulation for `double` arguments.

pub fn format_g(v: f64, prec: usize) -> String {
    let prec = if prec == 0 { 1 } else { prec };

    if v.is_nan() {
        return if v.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if v.is_infinite() {
        return if v < 0.0 {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }

    // Round to `prec` significant digits using scientific notation to learn the
    // decimal exponent that %g uses to pick its style.
    let sci = format!("{:.*e}", prec - 1, v);
    let (mant, exp_str) = sci.split_once('e').expect("rust e-format has exponent");
    let exp: i32 = exp_str.parse().expect("valid exponent");

    if exp < -4 || exp >= prec as i32 {
        let mant = strip_trailing_zeros(mant);
        let (sign, digits) = if exp < 0 {
            ('-', (-(exp as i64)) as u64)
        } else {
            ('+', exp as u64)
        };
        format!("{}e{}{:02}", mant, sign, digits)
    } else {
        let frac_digits = (prec as i32 - 1 - exp) as usize;
        let s = format!("{:.*}", frac_digits, v);
        strip_trailing_zeros(&s)
    }
}

fn strip_trailing_zeros(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    let t = s.trim_end_matches('0');
    let t = t.strip_suffix('.').unwrap_or(t);
    t.to_string()
}
