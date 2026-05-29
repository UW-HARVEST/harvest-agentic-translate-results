//! Pure-Rust port of the snprintf helpers used by xopt's error reporting.
//!
//! The original C file implements a complete printf-family library. The Rust
//! port here exposes the same function surface with simplified, safe
//! implementations that cover the subset of behaviour exercised by the rest of
//! the project (xopt error formatting and tests).

/// A small subset of vsnprintf-like formatting: substitutes successive `%s`,
/// `%c`, `%d`, `%x`, etc. occurrences in `format` with corresponding entries
/// from `args`. Returns the number of characters written.
pub fn rpl_vsnprintf(
    s: &mut String,
    n: usize,
    format: &str,
    args: &[&str],
) -> i32 {
    let mut out = String::new();
    let bytes = format.as_bytes();
    let mut i = 0usize;
    let mut argi = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'%' && i + 1 < bytes.len() {
            // Skip flags/width/precision/length chars and consume conversion
            let mut j = i + 1;
            // flags
            while j < bytes.len() && b"-+ #0'".contains(&bytes[j]) {
                j += 1;
            }
            // width digits or '*'
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            // precision
            if j < bytes.len() && bytes[j] == b'.' {
                j += 1;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
            }
            // length modifiers
            while j < bytes.len() && b"hlLjtz".contains(&bytes[j]) {
                j += 1;
            }
            if j >= bytes.len() {
                out.push('%');
                i += 1;
                continue;
            }
            let conv = bytes[j];
            match conv {
                b'%' => {
                    out.push('%');
                }
                b's' | b'd' | b'i' | b'u' | b'x' | b'X' | b'o' | b'c' | b'f' | b'g' | b'e' | b'p' => {
                    if argi < args.len() {
                        out.push_str(args[argi]);
                        argi += 1;
                    }
                }
                _ => {
                    out.push('%');
                    out.push(conv as char);
                }
            }
            i = j + 1;
        } else {
            out.push(b as char);
            i += 1;
        }
    }

    // Truncate to `n` characters if a finite limit was given (n == 0 means no
    // limit in our simplified API).
    if n != 0 && out.len() > n {
        out.truncate(n);
    }

    *s = out;
    s.len() as i32
}

/// Append `value` (a pre-formatted string) to `s`, optionally padded.
pub fn fmtstr(
    s: &mut String,
    size: usize,
    value: &str,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let _ = (size, flags);
    let take = if precision == 0 { value.len() } else { precision.min(value.len()) };
    let slice = &value[..take];
    let padlen = if width > slice.len() { width - slice.len() } else { 0 };
    if (flags & 1) != 0 {
        // PRINT_F_MINUS — left-justify, pad on the right
        s.push_str(slice);
        for _ in 0..padlen {
            s.push(' ');
        }
    } else {
        for _ in 0..padlen {
            s.push(' ');
        }
        s.push_str(slice);
    }
}

/// Append a formatted integer to `s`.
pub fn fmtint(
    s: &mut String,
    size: usize,
    value: i32,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let _ = (size, flags);
    let mut text = if precision == 0 {
        format!("{}", value)
    } else {
        let abs = value.unsigned_abs() as u64;
        let digits = format!("{:0>width$}", abs, width = precision);
        if value < 0 {
            format!("-{}", digits)
        } else {
            digits
        }
    };
    if width > text.len() {
        let pad = width - text.len();
        text = format!("{}{}", " ".repeat(pad), text);
    }
    s.push_str(&text);
}

/// Append a formatted float to `s`.
pub fn fmtflt(
    s: &mut String,
    size: usize,
    value: f64,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let _ = (size, flags);
    let prec = if precision == 0 { 6 } else { precision };
    let mut text = format!("{:.*}", prec, value);
    if width > text.len() {
        let pad = width - text.len();
        text = format!("{}{}", " ".repeat(pad), text);
    }
    s.push_str(&text);
}

/// Append a thousands separator (locale's grouping char).
pub fn printsep(s: &mut String, size: usize) {
    let _ = size;
    s.push(',');
}

/// Returns the number of digit-group separators that appear within `digits`
/// digits (i.e. one separator every three digits except for the most
/// significant group).
pub fn getnumsep(digits: i32) -> i32 {
    let mut sep = (digits - 1) / 3;
    if sep < 0 {
        sep = 0;
    }
    sep
}

/// Returns the decimal exponent of `value` — i.e. the integer `e` such that
/// `value` lies in `[10^e, 10^(e+1))` (or the negative analogue).
pub fn getexponent(value: f64) -> i32 {
    if value == 0.0 || !value.is_finite() {
        return 0;
    }
    value.abs().log10().floor() as i32
}

/// Convert `value` (an unsigned integer) to a string in `base`, optionally
/// using upper-case digits. Writes least-significant digit first.
pub fn convert(value: usize, buf: &mut String, base: usize, caps: usize) {
    buf.clear();
    if value == 0 {
        buf.push('0');
        return;
    }
    let digits_lower = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let digits_upper = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let table = if caps != 0 { digits_upper } else { digits_lower };
    let mut v = value;
    while v != 0 {
        let d = v % base;
        v /= base;
        buf.push(table[d] as char);
    }
}

/// Truncate a floating-point value to its integer part as `i32`.
pub fn cast(value: f64) -> i32 {
    value.trunc() as i32
}

/// Returns `10^exponent` as a `f64`.
pub fn mypow10(exponent: i32) -> f64 {
    10f64.powi(exponent)
}

/// Formats `format` using `args`, allocating the result into `s`.
pub fn rpl_vasprintf(mut s: Vec<String>, format: &str, args: &[&str]) -> i32 {
    let mut buf = String::new();
    let n = rpl_vsnprintf(&mut buf, 0, format, args);
    s.push(buf);
    n
}

/// Formats `format` using `args`, writing the result into `s`.
pub fn rpl_asprintf(s: &mut String, format: &str, args: &[&str]) -> i32 {
    rpl_vsnprintf(s, 0, format, args)
}

pub fn main() {
    // Stand-in for the C file's optional self-test entrypoint. Intentionally
    // empty in the Rust port — the real test coverage lives in `tests/`.
}
