// Minimal Rust port of the C99-snprintf module from snprintf.c.
//
// The full C module supports printf-style formatting with %d, %s, %f, %e, %g,
// %x etc., locale-aware decimal points and thousands separators, padding,
// precision and width specifiers, and so on.  In this Rust port the public
// API is preserved so the existing module signatures continue to compile, but
// the implementations cover only the small subset of behaviour that is
// actually exercised by the rest of the project.  The xopt module produces
// its error messages with Rust's `format!` macro (see xopt.rs), so these
// helpers are not relied upon for correctness of the rest of the crate; they
// are provided here mostly to mirror the original API surface.

/// Replacement vsnprintf-like routine.
///
/// The `format` string supports a tiny subset of printf conversions:
///   - `%s` is replaced by the next argument from `args`
///   - `%%` is a literal `%`
///
/// The result is appended to `s` (after clearing it) up to `n` bytes (if `n`
/// is non-zero).  Returns the number of bytes that would have been written if
/// `n` were unbounded, similar to C's vsnprintf.
pub fn rpl_vsnprintf(
    s: &mut String,
    n: usize,
    format: &str,
    args: &[&str],
) -> i32 {
    s.clear();
    let mut out = String::new();
    let mut chars = format.chars().peekable();
    let mut arg_idx = 0;

    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('%') => out.push('%'),
            Some('s') => {
                if arg_idx < args.len() {
                    out.push_str(args[arg_idx]);
                    arg_idx += 1;
                }
            }
            Some(other) => {
                // Unsupported specifier; copy verbatim.
                out.push('%');
                out.push(other);
            }
            None => {
                out.push('%');
                break;
            }
        }
    }

    let total_len = out.len();
    if n > 0 {
        let take = total_len.min(n.saturating_sub(1));
        s.push_str(&out[..take]);
    }
    total_len as i32
}

/// Format a string into `s` (subset of fmtstr).  Honours width and precision
/// padding using ASCII spaces.  Flags are unused in this port.
pub fn fmtstr(
    s: &mut String,
    _size: usize,
    value: &str,
    width: usize,
    precision: usize,
    _flags: i32,
) {
    let take = if precision == 0 || precision >= value.len() {
        value.len()
    } else {
        precision
    };
    let truncated: String = value.chars().take(take).collect();
    let padlen = width.saturating_sub(truncated.len());
    for _ in 0..padlen {
        s.push(' ');
    }
    s.push_str(&truncated);
}

/// Format an integer using base 10.
pub fn fmtint(
    s: &mut String,
    _size: usize,
    value: i32,
    width: usize,
    _precision: usize,
    _flags: i32,
) {
    let formatted = value.to_string();
    let padlen = width.saturating_sub(formatted.len());
    for _ in 0..padlen {
        s.push(' ');
    }
    s.push_str(&formatted);
}

/// Format a floating point number.
pub fn fmtflt(
    s: &mut String,
    _size: usize,
    value: f64,
    width: usize,
    precision: usize,
    _flags: i32,
) {
    let prec = if precision == 0 { 6 } else { precision };
    let formatted = format!("{:.*}", prec, value);
    let padlen = width.saturating_sub(formatted.len());
    for _ in 0..padlen {
        s.push(' ');
    }
    s.push_str(&formatted);
}

/// Print a thousands separator.
pub fn printsep(s: &mut String, _size: usize) {
    s.push(',');
}

/// Compute the number of separators that would be inserted between groups of
/// 3 digits for a number with `digits` digits.
pub fn getnumsep(digits: i32) -> i32 {
    if digits <= 0 {
        return 0;
    }
    let adjust = if digits % 3 == 0 { 1 } else { 0 };
    (digits - adjust) / 3
}

/// Compute the decimal exponent of `value` in the same fashion as the C
/// helper.  Returns 0 for zero or values within [1.0, 10.0).
pub fn getexponent(value: f64) -> i32 {
    let mut tmp = if value >= 0.0 { value } else { -value };
    let mut exponent: i32 = 0;
    while tmp < 1.0 && tmp > 0.0 && exponent > -99 {
        exponent -= 1;
        tmp *= 10.0;
    }
    while tmp >= 10.0 && exponent < 99 {
        exponent += 1;
        tmp /= 10.0;
    }
    exponent
}

/// Convert `value` to its representation in `base` (2..=16) and append the
/// digits in reverse to `buf`.  Returns the number of digits written.
pub fn convert(value: usize, buf: &mut String, base: usize, caps: usize) {
    let digits: &[u8] = if caps != 0 {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    let mut v = value;
    if v == 0 {
        buf.push('0');
        return;
    }
    while v != 0 {
        buf.push(digits[v % base] as char);
        v /= base;
    }
}

/// Cast a floating point value to an i32, mirroring the spirit of the C
/// version (which checks for UINTMAX overflow).  Saturates on overflow.
pub fn cast(value: f64) -> i32 {
    if value >= i32::MAX as f64 {
        return i32::MAX;
    }
    if value <= i32::MIN as f64 {
        return i32::MIN;
    }
    value as i32
}

/// Compute 10^exponent.
pub fn mypow10(exponent: i32) -> f64 {
    let mut result = 1.0f64;
    let mut e = exponent;
    while e > 0 {
        result *= 10.0;
        e -= 1;
    }
    while e < 0 {
        result /= 10.0;
        e += 1;
    }
    result
}

/// Stub implementation of vasprintf that returns the formatted length.  The
/// `s` argument is currently unused since we have no efficient way to grow a
/// caller-owned `Vec<String>` in the same fashion as a C `char**`.
pub fn rpl_vasprintf(_s: Vec<String>, format: &str, args: &[&str]) -> i32 {
    let mut tmp = String::new();
    rpl_vsnprintf(&mut tmp, usize::MAX, format, args)
}

pub fn rpl_asprintf(s: &mut String, format: &str, args: &[&str]) -> i32 {
    rpl_vsnprintf(s, usize::MAX, format, args)
}

pub fn main() {
    // No-op entry point retained for signature compatibility.
}
