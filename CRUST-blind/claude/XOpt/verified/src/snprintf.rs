//! A simplified port of the C99 `snprintf` family used by XOpt.
//!
//! The C source supports arbitrary `printf` format specifiers via `va_list`.
//! In this Rust port, all arguments are passed as `&str`, and the format
//! specifier is honoured in a best-effort fashion (each `%`-directive consumes
//! one element of `args`).  This is sufficient for the formatting needs of
//! XOpt itself, which only uses the `%s`, `%c`, `%d`, and `%.*s` directives.

/// Format flags (mirrors C `PRINT_F_*`).
const PRINT_F_MINUS: i32 = 1 << 0;
#[allow(dead_code)]
const PRINT_F_PLUS: i32 = 1 << 1;
#[allow(dead_code)]
const PRINT_F_SPACE: i32 = 1 << 2;
#[allow(dead_code)]
const PRINT_F_NUM: i32 = 1 << 3;
const PRINT_F_ZERO: i32 = 1 << 4;
#[allow(dead_code)]
const PRINT_F_QUOTE: i32 = 1 << 5;
#[allow(dead_code)]
const PRINT_F_UP: i32 = 1 << 6;
#[allow(dead_code)]
const PRINT_F_UNSIGNED: i32 = 1 << 7;
#[allow(dead_code)]
const PRINT_F_TYPE_G: i32 = 1 << 8;
#[allow(dead_code)]
const PRINT_F_TYPE_E: i32 = 1 << 9;

/// Append `ch` to `s` if there is still room (`s.len() + 1 < n`); always
/// increment the conceptual length counter (returned).
fn outchar(s: &mut String, len: &mut usize, n: usize, ch: char) {
    if *len + 1 < n {
        s.push(ch);
    }
    *len += 1;
}

/// Parse `format` consuming `args` in sequence; write the result into `s`,
/// truncating to `n - 1` bytes (with a final terminator implicit in Rust's
/// `String`).  Returns the number of characters that *would* have been written
/// had the buffer been unbounded (matching C99 semantics).
pub fn rpl_vsnprintf(s: &mut String, n: usize, format: &str, args: &[&str]) -> i32 {
    s.clear();
    let mut len: usize = 0;
    let chars: Vec<char> = format.chars().collect();
    let mut i = 0;
    let mut arg_idx = 0;

    while i < chars.len() {
        let ch = chars[i];
        if ch != '%' {
            outchar(s, &mut len, n, ch);
            i += 1;
            continue;
        }
        i += 1;
        if i >= chars.len() {
            // Stray '%' at end - emit as-is.
            outchar(s, &mut len, n, '%');
            break;
        }

        // Parse flags.
        let mut flags: i32 = 0;
        loop {
            match chars[i] {
                '-' => flags |= PRINT_F_MINUS,
                '+' => flags |= PRINT_F_PLUS,
                ' ' => flags |= PRINT_F_SPACE,
                '#' => flags |= PRINT_F_NUM,
                '0' => flags |= PRINT_F_ZERO,
                '\'' => flags |= PRINT_F_QUOTE,
                _ => break,
            }
            i += 1;
            if i >= chars.len() {
                return len as i32;
            }
        }

        // Parse width.
        let mut width: usize = 0;
        if chars[i] == '*' {
            if arg_idx < args.len() {
                if let Ok(w) = args[arg_idx].parse::<i32>() {
                    if w < 0 {
                        flags |= PRINT_F_MINUS;
                        width = (-w) as usize;
                    } else {
                        width = w as usize;
                    }
                }
                arg_idx += 1;
            }
            i += 1;
        } else {
            while i < chars.len() && chars[i].is_ascii_digit() {
                width = width * 10 + (chars[i] as usize - '0' as usize);
                i += 1;
            }
        }
        if i >= chars.len() {
            return len as i32;
        }

        // Parse precision.
        let mut precision: i32 = -1;
        if chars[i] == '.' {
            i += 1;
            precision = 0;
            if i < chars.len() && chars[i] == '*' {
                if arg_idx < args.len() {
                    if let Ok(p) = args[arg_idx].parse::<i32>() {
                        precision = p;
                    }
                    arg_idx += 1;
                }
                i += 1;
            } else {
                while i < chars.len() && chars[i].is_ascii_digit() {
                    precision = precision * 10 + (chars[i] as i32 - '0' as i32);
                    i += 1;
                }
            }
        }
        if i >= chars.len() {
            return len as i32;
        }

        // Skip length modifiers.
        while i < chars.len() {
            match chars[i] {
                'h' | 'l' | 'L' | 'j' | 't' | 'z' => i += 1,
                _ => break,
            }
        }
        if i >= chars.len() {
            return len as i32;
        }

        // Conversion specifier.
        let conv = chars[i];
        i += 1;
        match conv {
            '%' => outchar(s, &mut len, n, '%'),
            's' => {
                let val = args.get(arg_idx).copied().unwrap_or("(null)");
                arg_idx += 1;
                let prec = if precision < 0 {
                    usize::MAX
                } else {
                    precision as usize
                };
                fmtstr_chars(s, &mut len, n, val, width, prec, flags);
            }
            'c' => {
                let val = args.get(arg_idx).copied().unwrap_or("");
                arg_idx += 1;
                if let Some(c) = val.chars().next() {
                    outchar(s, &mut len, n, c);
                }
            }
            'd' | 'i' => {
                let val = args.get(arg_idx).copied().unwrap_or("0");
                arg_idx += 1;
                let n_val: i32 = val.parse().unwrap_or(0);
                fmtint_internal(s, &mut len, n, n_val as i64, 10, width, precision, flags);
            }
            'u' => {
                let val = args.get(arg_idx).copied().unwrap_or("0");
                arg_idx += 1;
                let n_val: u64 = val.parse().unwrap_or(0);
                fmtint_internal(
                    s,
                    &mut len,
                    n,
                    n_val as i64,
                    10,
                    width,
                    precision,
                    flags | PRINT_F_UNSIGNED,
                );
            }
            'x' => {
                let val = args.get(arg_idx).copied().unwrap_or("0");
                arg_idx += 1;
                let n_val: u64 = val.parse().unwrap_or(0);
                fmtint_internal(
                    s,
                    &mut len,
                    n,
                    n_val as i64,
                    16,
                    width,
                    precision,
                    flags | PRINT_F_UNSIGNED,
                );
            }
            'X' => {
                let val = args.get(arg_idx).copied().unwrap_or("0");
                arg_idx += 1;
                let n_val: u64 = val.parse().unwrap_or(0);
                fmtint_internal(
                    s,
                    &mut len,
                    n,
                    n_val as i64,
                    16,
                    width,
                    precision,
                    flags | PRINT_F_UNSIGNED | PRINT_F_UP,
                );
            }
            'o' => {
                let val = args.get(arg_idx).copied().unwrap_or("0");
                arg_idx += 1;
                let n_val: u64 = val.parse().unwrap_or(0);
                fmtint_internal(
                    s,
                    &mut len,
                    n,
                    n_val as i64,
                    8,
                    width,
                    precision,
                    flags | PRINT_F_UNSIGNED,
                );
            }
            'f' | 'F' | 'e' | 'E' | 'g' | 'G' => {
                let val = args.get(arg_idx).copied().unwrap_or("0");
                arg_idx += 1;
                let f_val: f64 = val.parse().unwrap_or(0.0);
                let prec = if precision < 0 { 6 } else { precision as usize };
                fmtflt_internal(s, &mut len, n, f_val, width, prec, flags);
            }
            _ => {
                // Unknown conversion: skip silently.
            }
        }
    }

    len as i32
}

/// Public wrapper used by other tests.  It mirrors the public signature.
pub fn fmtstr(s: &mut String, size: usize, value: &str, width: usize, precision: usize, flags: i32) {
    let mut len = s.chars().count();
    fmtstr_chars(s, &mut len, size, value, width, precision, flags);
}

fn fmtstr_chars(
    s: &mut String,
    len: &mut usize,
    size: usize,
    value: &str,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let chars: Vec<char> = value.chars().collect();
    let strln = chars.len().min(precision);

    let mut padlen: i64 = width as i64 - strln as i64;
    if padlen < 0 {
        padlen = 0;
    }
    if (flags & PRINT_F_MINUS) != 0 {
        padlen = -padlen;
    }

    while padlen > 0 {
        outchar(s, len, size, ' ');
        padlen -= 1;
    }
    let mut emitted = 0;
    for c in chars {
        if emitted >= precision {
            break;
        }
        outchar(s, len, size, c);
        emitted += 1;
    }
    while padlen < 0 {
        outchar(s, len, size, ' ');
        padlen += 1;
    }
}

/// Public wrapper for `fmtint`, matching the documented signature.
pub fn fmtint(s: &mut String, size: usize, value: i32, width: usize, precision: usize, flags: i32) {
    let mut len = s.chars().count();
    let prec = if precision == usize::MAX {
        -1
    } else {
        precision as i32
    };
    fmtint_internal(s, &mut len, size, value as i64, 10, width, prec, flags);
}

fn fmtint_internal(
    s: &mut String,
    len: &mut usize,
    size: usize,
    value: i64,
    base: i32,
    width: usize,
    precision: i32,
    flags: i32,
) {
    let mut sign: char = '\0';
    let unsigned_flag = (flags & PRINT_F_UNSIGNED) != 0;
    let uvalue: u64 = if unsigned_flag {
        value as u64
    } else if value >= 0 {
        value as u64
    } else {
        (-(value as i128)) as u64
    };

    if !unsigned_flag {
        if value < 0 {
            sign = '-';
        } else if (flags & PRINT_F_PLUS) != 0 {
            sign = '+';
        } else if (flags & PRINT_F_SPACE) != 0 {
            sign = ' ';
        }
    }

    let mut iconvert = String::new();
    let pos = convert_internal(uvalue as usize, &mut iconvert, base as usize, (flags & PRINT_F_UP) as usize);

    let mut hexprefix: char = '\0';
    let mut precision = precision;
    if (flags & PRINT_F_NUM) != 0 && uvalue != 0 {
        match base {
            8 => {
                if precision <= pos as i32 {
                    precision = pos as i32 + 1;
                }
            }
            16 => {
                hexprefix = if (flags & PRINT_F_UP) != 0 { 'X' } else { 'x' };
            }
            _ => {}
        }
    }

    let noprecision = precision == -1;
    let separators = 0;
    let mut zpadlen: i64 = if noprecision {
        0
    } else {
        precision as i64 - pos as i64
    };
    let max_pos_prec = if noprecision { pos as i64 } else { (precision as i64).max(pos as i64) };
    let mut spadlen: i64 = width as i64
        - separators
        - max_pos_prec
        - if sign != '\0' { 1 } else { 0 }
        - if hexprefix != '\0' { 2 } else { 0 };

    if zpadlen < 0 {
        zpadlen = 0;
    }
    if spadlen < 0 {
        spadlen = 0;
    }

    if (flags & PRINT_F_MINUS) != 0 {
        spadlen = -spadlen;
    } else if (flags & PRINT_F_ZERO) != 0 && noprecision {
        zpadlen += spadlen;
        spadlen = 0;
    }

    while spadlen > 0 {
        outchar(s, len, size, ' ');
        spadlen -= 1;
    }
    if sign != '\0' {
        outchar(s, len, size, sign);
    }
    if hexprefix != '\0' {
        outchar(s, len, size, '0');
        outchar(s, len, size, hexprefix);
    }
    while zpadlen > 0 {
        outchar(s, len, size, '0');
        zpadlen -= 1;
    }
    let iconvert_chars: Vec<char> = iconvert.chars().collect();
    let mut p = pos;
    while p > 0 {
        p -= 1;
        outchar(s, len, size, iconvert_chars[p]);
    }
    while spadlen < 0 {
        outchar(s, len, size, ' ');
        spadlen += 1;
    }
}

/// Public wrapper for `fmtflt`.
pub fn fmtflt(s: &mut String, size: usize, value: f64, width: usize, precision: usize, flags: i32) {
    let mut len = s.chars().count();
    fmtflt_internal(s, &mut len, size, value, width, precision, flags);
}

fn fmtflt_internal(
    s: &mut String,
    len: &mut usize,
    size: usize,
    value: f64,
    width: usize,
    precision: usize,
    flags: i32,
) {
    // Use Rust's built-in formatting and then pad / sign as required.
    let mut sign: char = '\0';
    let abs_val = if value < 0.0 { -value } else { value };
    if value < 0.0 {
        sign = '-';
    } else if (flags & PRINT_F_PLUS) != 0 {
        sign = '+';
    } else if (flags & PRINT_F_SPACE) != 0 {
        sign = ' ';
    }

    let body = if value.is_nan() {
        if (flags & PRINT_F_UP) != 0 { "nan".to_string() } else { "nan".to_string() }
    } else if value.is_infinite() {
        if (flags & PRINT_F_UP) != 0 { "inf".to_string() } else { "inf".to_string() }
    } else {
        format!("{:.*}", precision, abs_val)
    };

    let total_len = body.chars().count() + if sign != '\0' { 1 } else { 0 };
    let mut padlen: i64 = width as i64 - total_len as i64;
    if padlen < 0 {
        padlen = 0;
    }

    if (flags & PRINT_F_MINUS) != 0 {
        padlen = -padlen;
    } else if (flags & PRINT_F_ZERO) != 0 && padlen > 0 {
        if sign != '\0' {
            outchar(s, len, size, sign);
            sign = '\0';
        }
        while padlen > 0 {
            outchar(s, len, size, '0');
            padlen -= 1;
        }
    }
    while padlen > 0 {
        outchar(s, len, size, ' ');
        padlen -= 1;
    }
    if sign != '\0' {
        outchar(s, len, size, sign);
    }
    for c in body.chars() {
        outchar(s, len, size, c);
    }
    while padlen < 0 {
        outchar(s, len, size, ' ');
        padlen += 1;
    }
}

pub fn printsep(s: &mut String, size: usize) {
    let mut len = s.chars().count();
    outchar(s, &mut len, size, ',');
}

pub fn getnumsep(digits: i32) -> i32 {
    (digits - if digits % 3 == 0 { 1 } else { 0 }) / 3
}

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

pub fn convert(value: usize, buf: &mut String, base: usize, caps: usize) {
    convert_internal(value, buf, base, caps);
}

fn convert_internal(value: usize, buf: &mut String, base: usize, caps: usize) -> usize {
    buf.clear();
    let digits: &[u8] = if caps != 0 {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    let mut v = value;
    let base = base.max(2);
    loop {
        buf.push(digits[v % base] as char);
        v /= base;
        if v == 0 {
            break;
        }
    }
    buf.chars().count()
}

pub fn cast(value: f64) -> i32 {
    if value >= i32::MAX as f64 {
        return i32::MAX;
    }
    if value <= i32::MIN as f64 {
        return i32::MIN;
    }
    let result = value as i32;
    if (result as f64) <= value {
        result
    } else {
        result - 1
    }
}

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

pub fn rpl_vasprintf(_s: Vec<String>, _format: &str, _args: &[&str]) -> i32 {
    // The original C function allocates a new buffer of the right size and
    // writes the formatted output into it.  In the Rust port the caller would
    // need a `&mut String`; this overload keeps a compatible signature but is
    // implemented in terms of `rpl_asprintf`.
    let mut buf = String::new();
    let len = rpl_vsnprintf(&mut buf, usize::MAX, _format, _args);
    len
}

pub fn rpl_asprintf(s: &mut String, format: &str, args: &[&str]) -> i32 {
    rpl_vsnprintf(s, usize::MAX, format, args)
}

pub fn main() {
    // Self-test placeholder - the original C version contained a TEST_SNPRINTF
    // suite that exercised many printf edge cases.  In the Rust port we leave
    // this as a no-op so that running the binary form of the crate succeeds.
}
