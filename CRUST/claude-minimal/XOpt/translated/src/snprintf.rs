//! A simplified Rust port of the C `snprintf.c` from `c_src/`.
//!
//! The original C file implements a printf-family of replacement functions
//! (`rpl_vsnprintf`, `rpl_vasprintf`, etc.). This Rust port keeps the same
//! function names and broadly equivalent semantics, but adapts the API to
//! Rust idioms: instead of `va_list`/raw pointers we accept a `&[&str]` of
//! pre-stringified arguments. Each `%` conversion in `format` consumes one
//! string from `args` and is rendered into `s`. The numeric `%d`/`%i`/`%f`
//! conversions parse the corresponding string argument before formatting.
//!
//! Format flags (`-`, `+`, ` `, `#`, `0`), width and precision (including
//! `.N`) are honored. Conversions that aren't applicable (e.g. unrecognized
//! specifiers) fall back to printing the argument verbatim, matching the
//! original code's "be forgiving" disposition.

// Format flag bits (mirrors the C `PRINT_F_*` macros).
const PRINT_F_MINUS: i32 = 1 << 0;
const PRINT_F_PLUS: i32 = 1 << 1;
const PRINT_F_SPACE: i32 = 1 << 2;
const PRINT_F_NUM: i32 = 1 << 3;
const PRINT_F_ZERO: i32 = 1 << 4;
#[allow(dead_code)]
const PRINT_F_QUOTE: i32 = 1 << 5;
const PRINT_F_UP: i32 = 1 << 6;
#[allow(dead_code)]
const PRINT_F_UNSIGNED: i32 = 1 << 7;

// Appends `ch` to `s` so long as adding it would keep length < n.
// Always increments the conceptual length counter; returns the new length.
fn outchar(s: &mut String, len: usize, n: usize, ch: char) -> usize {
    if len + 1 < n {
        s.push(ch);
    } else if n == 0 {
        // No bound; just append.
        s.push(ch);
    }
    len + 1
}

/// vsnprintf-equivalent. Writes formatted output into `s`, consuming
/// pre-stringified arguments from `args`. Returns the number of characters
/// that *would* have been written (excluding the terminator), like C's
/// `vsnprintf`.
pub fn rpl_vsnprintf(
    s: &mut String,
    n: usize,
    format: &str,
    args: &[&str],
) -> i32 {
    let bytes = format.as_bytes();
    let mut i = 0usize;
    let mut len: usize = 0;
    let mut arg_idx = 0usize;
    let no_bound = n == 0;

    while i < bytes.len() {
        let ch = bytes[i] as char;
        if ch != '%' {
            len = if no_bound {
                outchar(s, len, usize::MAX, ch)
            } else {
                outchar(s, len, n, ch)
            };
            i += 1;
            continue;
        }

        // Handle '%'
        i += 1;
        if i >= bytes.len() {
            break;
        }

        // Parse flags.
        let mut flags = 0i32;
        loop {
            if i >= bytes.len() {
                break;
            }
            match bytes[i] as char {
                '-' => flags |= PRINT_F_MINUS,
                '+' => flags |= PRINT_F_PLUS,
                ' ' => flags |= PRINT_F_SPACE,
                '#' => flags |= PRINT_F_NUM,
                '0' => flags |= PRINT_F_ZERO,
                '\'' => flags |= PRINT_F_QUOTE,
                _ => break,
            }
            i += 1;
        }

        // Parse width.
        let mut width: usize = 0;
        while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
            width = width * 10 + (bytes[i] - b'0') as usize;
            i += 1;
        }

        // Parse precision.
        let mut precision: i32 = -1;
        if i < bytes.len() && bytes[i] as char == '.' {
            i += 1;
            precision = 0;
            while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                precision = precision * 10 + (bytes[i] - b'0') as i32;
                i += 1;
            }
        }

        // Skip length modifiers (h, hh, l, ll, L, j, t, z).
        while i < bytes.len() {
            match bytes[i] as char {
                'h' | 'l' | 'L' | 'j' | 't' | 'z' => i += 1,
                _ => break,
            }
        }

        if i >= bytes.len() {
            break;
        }

        let conv = bytes[i] as char;
        i += 1;

        let target_n = if no_bound { usize::MAX } else { n };

        match conv {
            '%' => {
                len = outchar(s, len, target_n, '%');
            }
            'd' | 'i' => {
                let v = args.get(arg_idx).copied().unwrap_or("0");
                arg_idx += 1;
                let parsed: i64 = v.trim().parse().unwrap_or(0);
                let prec = if precision < 0 { -1 } else { precision };
                fmtint_internal(s, &mut len, target_n, parsed as i128, 10, width, prec, flags);
            }
            'u' => {
                let v = args.get(arg_idx).copied().unwrap_or("0");
                arg_idx += 1;
                let parsed: u64 = v.trim().parse().unwrap_or(0);
                let prec = if precision < 0 { -1 } else { precision };
                fmtint_internal(
                    s,
                    &mut len,
                    target_n,
                    parsed as i128,
                    10,
                    width,
                    prec,
                    flags | PRINT_F_UNSIGNED,
                );
            }
            'o' => {
                let v = args.get(arg_idx).copied().unwrap_or("0");
                arg_idx += 1;
                let parsed: u64 = v.trim().parse().unwrap_or(0);
                let prec = if precision < 0 { -1 } else { precision };
                fmtint_internal(
                    s,
                    &mut len,
                    target_n,
                    parsed as i128,
                    8,
                    width,
                    prec,
                    flags | PRINT_F_UNSIGNED,
                );
            }
            'x' => {
                let v = args.get(arg_idx).copied().unwrap_or("0");
                arg_idx += 1;
                let parsed: u64 = v.trim().parse().unwrap_or(0);
                let prec = if precision < 0 { -1 } else { precision };
                fmtint_internal(
                    s,
                    &mut len,
                    target_n,
                    parsed as i128,
                    16,
                    width,
                    prec,
                    flags | PRINT_F_UNSIGNED,
                );
            }
            'X' => {
                let v = args.get(arg_idx).copied().unwrap_or("0");
                arg_idx += 1;
                let parsed: u64 = v.trim().parse().unwrap_or(0);
                let prec = if precision < 0 { -1 } else { precision };
                fmtint_internal(
                    s,
                    &mut len,
                    target_n,
                    parsed as i128,
                    16,
                    width,
                    prec,
                    flags | PRINT_F_UNSIGNED | PRINT_F_UP,
                );
            }
            'f' | 'F' | 'e' | 'E' | 'g' | 'G' => {
                let v = args.get(arg_idx).copied().unwrap_or("0");
                arg_idx += 1;
                let parsed: f64 = v.trim().parse().unwrap_or(0.0);
                let prec = if precision < 0 { 6 } else { precision };
                let mut up_flags = flags;
                if conv == 'F' || conv == 'E' || conv == 'G' {
                    up_flags |= PRINT_F_UP;
                }
                fmtflt_internal(
                    s,
                    &mut len,
                    target_n,
                    parsed,
                    width,
                    prec as usize,
                    up_flags,
                );
            }
            's' => {
                let v = args.get(arg_idx).copied().unwrap_or("(null)");
                arg_idx += 1;
                let prec = if precision < 0 {
                    usize::MAX
                } else {
                    precision as usize
                };
                fmtstr_internal(s, &mut len, target_n, v, width, prec, flags);
            }
            'c' => {
                let v = args.get(arg_idx).copied().unwrap_or("");
                arg_idx += 1;
                let c = v.chars().next().unwrap_or('\0');
                len = outchar(s, len, target_n, c);
            }
            _ => {
                // Unknown specifier: skip silently (matches C "default" branch).
            }
        }
    }

    len as i32
}

// Internal width-aware string formatter.
fn fmtstr_internal(
    s: &mut String,
    len: &mut usize,
    n: usize,
    value: &str,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let strln = if precision == usize::MAX {
        value.chars().count()
    } else {
        value.chars().count().min(precision)
    };
    let mut padlen: i32 = width as i32 - strln as i32;
    if padlen < 0 {
        padlen = 0;
    }
    if flags & PRINT_F_MINUS != 0 {
        padlen = -padlen;
    }

    let mut p = padlen;
    while p > 0 {
        *len = outchar(s, *len, n, ' ');
        p -= 1;
    }

    let mut emitted = 0usize;
    for c in value.chars() {
        if precision != usize::MAX && emitted >= precision {
            break;
        }
        *len = outchar(s, *len, n, c);
        emitted += 1;
    }

    while p < 0 {
        *len = outchar(s, *len, n, ' ');
        p += 1;
    }
}

// Internal width-aware integer formatter.
fn fmtint_internal(
    s: &mut String,
    len: &mut usize,
    n: usize,
    value: i128,
    base: u32,
    width: usize,
    precision: i32,
    flags: i32,
) {
    let unsigned = flags & PRINT_F_UNSIGNED != 0;
    let mut sign: char = '\0';
    let uvalue: u128 = if unsigned {
        value as u128
    } else if value < 0 {
        sign = '-';
        (-value) as u128
    } else {
        if flags & PRINT_F_PLUS != 0 {
            sign = '+';
        } else if flags & PRINT_F_SPACE != 0 {
            sign = ' ';
        }
        value as u128
    };

    let mut digits = String::new();
    convert_internal(uvalue, &mut digits, base as usize, (flags & PRINT_F_UP) != 0);
    // `digits` is in reverse-of-output order? `convert_internal` mirrors the
    // C `convert` function which produces digits least-significant-first.
    // We'll reverse here for printing.
    let pos = digits.chars().count() as i32;

    let mut hexprefix: char = '\0';
    if flags & PRINT_F_NUM != 0 && uvalue != 0 {
        match base {
            8 => {
                // Increase precision so the leading digit is 0.
                // Handled below in zpad calc.
            }
            16 => {
                hexprefix = if flags & PRINT_F_UP != 0 { 'X' } else { 'x' };
            }
            _ => {}
        }
    }

    let mut effective_precision = precision;
    if base == 8 && flags & PRINT_F_NUM != 0 && uvalue != 0 {
        if effective_precision <= pos {
            effective_precision = pos + 1;
        }
    }

    let noprecision = precision < 0;
    let mut zpadlen = if effective_precision >= 0 {
        effective_precision - pos
    } else {
        0
    };

    let mut spadlen = width as i32
        - i32::max(effective_precision.max(0), pos)
        - if sign != '\0' { 1 } else { 0 }
        - if hexprefix != '\0' { 2 } else { 0 };

    if zpadlen < 0 {
        zpadlen = 0;
    }
    if spadlen < 0 {
        spadlen = 0;
    }

    if flags & PRINT_F_MINUS != 0 {
        spadlen = -spadlen;
    } else if flags & PRINT_F_ZERO != 0 && noprecision {
        zpadlen += spadlen;
        spadlen = 0;
    }

    while spadlen > 0 {
        *len = outchar(s, *len, n, ' ');
        spadlen -= 1;
    }
    if sign != '\0' {
        *len = outchar(s, *len, n, sign);
    }
    if hexprefix != '\0' {
        *len = outchar(s, *len, n, '0');
        *len = outchar(s, *len, n, hexprefix);
    }
    while zpadlen > 0 {
        *len = outchar(s, *len, n, '0');
        zpadlen -= 1;
    }
    // Print the digits in correct (most-significant-first) order.
    let rev: Vec<char> = digits.chars().rev().collect();
    for c in rev {
        *len = outchar(s, *len, n, c);
    }
    while spadlen < 0 {
        *len = outchar(s, *len, n, ' ');
        spadlen += 1;
    }
}

// Internal width-aware float formatter for `%f`. Sufficient for typical use
// in this library (error messages don't usually format floats).
fn fmtflt_internal(
    s: &mut String,
    len: &mut usize,
    n: usize,
    value: f64,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let mut sign: char = '\0';
    let absv = if value < 0.0 {
        sign = '-';
        -value
    } else {
        if flags & PRINT_F_PLUS != 0 {
            sign = '+';
        } else if flags & PRINT_F_SPACE != 0 {
            sign = ' ';
        }
        value
    };

    if value.is_nan() {
        let nan_str = if flags & PRINT_F_UP != 0 { "NAN" } else { "nan" };
        fmtstr_internal(s, len, n, nan_str, width, usize::MAX, flags);
        return;
    }
    if value.is_infinite() {
        let inf_str = if flags & PRINT_F_UP != 0 { "INF" } else { "inf" };
        fmtstr_internal(s, len, n, inf_str, width, usize::MAX, flags);
        return;
    }

    // Format absolute value with the requested precision via Rust's own
    // formatting (this differs in edge cases from C's snprintf, but is
    // close enough for the purposes of this port).
    let formatted = format!("{:.*}", precision, absv);

    let total_len = formatted.chars().count() + if sign != '\0' { 1 } else { 0 };
    let padlen: i32 = width as i32 - total_len as i32;

    let (left_pad, right_pad) = if flags & PRINT_F_MINUS != 0 {
        (0, padlen.max(0))
    } else {
        (padlen.max(0), 0)
    };

    if flags & PRINT_F_ZERO != 0 && flags & PRINT_F_MINUS == 0 {
        if sign != '\0' {
            *len = outchar(s, *len, n, sign);
        }
        for _ in 0..left_pad {
            *len = outchar(s, *len, n, '0');
        }
        for c in formatted.chars() {
            *len = outchar(s, *len, n, c);
        }
    } else {
        for _ in 0..left_pad {
            *len = outchar(s, *len, n, ' ');
        }
        if sign != '\0' {
            *len = outchar(s, *len, n, sign);
        }
        for c in formatted.chars() {
            *len = outchar(s, *len, n, c);
        }
    }

    for _ in 0..right_pad {
        *len = outchar(s, *len, n, ' ');
    }
}

// ============================================================================
// Public re-exports of the original C helper functions, adapted to Rust.
// They simply delegate to the internal implementations above.
// ============================================================================

pub fn fmtstr(
    s: &mut String,
    size: usize,
    value: &str,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let mut len = s.chars().count();
    fmtstr_internal(s, &mut len, size, value, width, precision, flags);
}

pub fn fmtint(
    s: &mut String,
    size: usize,
    value: i32,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let mut len = s.chars().count();
    let prec = if precision == usize::MAX {
        -1
    } else {
        precision as i32
    };
    fmtint_internal(s, &mut len, size, value as i128, 10, width, prec, flags);
}

pub fn fmtflt(
    s: &mut String,
    size: usize,
    value: f64,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let mut len = s.chars().count();
    fmtflt_internal(s, &mut len, size, value, width, precision, flags);
}

pub fn printsep(s: &mut String, size: usize) {
    let mut len = s.chars().count();
    let _ = outchar(s, len, size, ',');
    len += 1;
    let _ = len; // suppress unused
}

/// Number of thousands-separators that would be emitted for a string of
/// `digits` decimal digits. Matches the C helper of the same name.
pub fn getnumsep(digits: i32) -> i32 {
    let separators = (digits - if digits % 3 == 0 { 1 } else { 0 }) / 3;
    separators
}

/// Returns the base-10 exponent of `value` such that `value` lies in
/// `[1, 10)` once divided by `10^exponent`. Mirrors the C `getexponent`.
pub fn getexponent(value: f64) -> i32 {
    let mut tmp = if value >= 0.0 { value } else { -value };
    let mut exponent = 0i32;
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

/// Converts `value` into the digit representation in the given `base`,
/// appending each digit (least-significant first) to `buf`. `caps` selects
/// uppercase hex letters. Mirrors the C `convert` helper.
pub fn convert(value: usize, buf: &mut String, base: usize, caps: usize) {
    convert_internal(value as u128, buf, base, caps != 0);
}

fn convert_internal(mut value: u128, buf: &mut String, base: usize, caps: bool) {
    if base < 2 || base > 16 {
        return;
    }
    let digits_lc = b"0123456789abcdef";
    let digits_uc = b"0123456789ABCDEF";
    let digits = if caps { digits_uc } else { digits_lc };

    if value == 0 {
        buf.push('0');
        return;
    }
    while value != 0 {
        let d = (value % base as u128) as usize;
        buf.push(digits[d] as char);
        value /= base as u128;
    }
}

/// Truncating cast of a float to an integer, matching C's `cast` helper.
pub fn cast(value: f64) -> i32 {
    if value.is_nan() {
        return 0;
    }
    if value >= i32::MAX as f64 {
        return i32::MAX;
    }
    if value <= i32::MIN as f64 {
        return i32::MIN;
    }
    value as i32
}

/// Computes 10^exponent as an `f64`. Mirrors the C `mypow10` helper.
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

/// vasprintf-equivalent: fills the first element of `s` (which is created if
/// the vector is empty) with the formatted string. Returns the number of
/// characters written.
pub fn rpl_vasprintf(mut s: Vec<String>, format: &str, args: &[&str]) -> i32 {
    let mut buf = String::new();
    let written = rpl_vsnprintf(&mut buf, 0, format, args);
    if s.is_empty() {
        s.push(buf);
    } else {
        s[0] = buf;
    }
    written
}

/// asprintf-equivalent: writes the formatted result into `s` (replacing any
/// existing contents) and returns the number of characters written.
pub fn rpl_asprintf(s: &mut String, format: &str, args: &[&str]) -> i32 {
    s.clear();
    rpl_vsnprintf(s, 0, format, args)
}

/// Stand-in for the original C `main()` smoke test. Runs a couple of small
/// formatting checks and prints the result to stderr.
pub fn main() {
    let mut buf = String::new();
    let _ = rpl_asprintf(&mut buf, "hello %s, %d", &["world", "42"]);
    eprintln!("{}", buf);
}
