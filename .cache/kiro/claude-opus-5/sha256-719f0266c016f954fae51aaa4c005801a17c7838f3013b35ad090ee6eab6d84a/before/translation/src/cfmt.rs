//! C `printf` compatible formatting helpers.

/// Format a `double` the way glibc's `printf("%.2f", v)` does.
pub fn format_f2(v: f64) -> String {
    if v.is_nan() {
        // glibc prints the sign of a NaN.
        if v.is_sign_negative() {
            return "-nan".to_string();
        }
        return "nan".to_string();
    }
    if v.is_infinite() {
        if v.is_sign_negative() {
            return "-inf".to_string();
        }
        return "inf".to_string();
    }
    // Rust's fixed-precision formatting is exact and rounds half to even,
    // matching glibc in the default rounding mode.
    format!("{:.2}", v)
}
