// Pure-Rust port of the snprintf library.
//
// The original C code in c_src/snprintf.c is a self-contained C99-compatible
// snprintf/asprintf implementation. The signatures provided here in Rust
// don't map cleanly onto the C semantics (the original takes a va_list and
// many of these helpers operate on a raw char buffer with state). We provide
// a sensible best-effort implementation that mirrors the original behavior
// as closely as the new signatures allow, while avoiding `unsafe` and
// matching the high-level intent of each function.

/// Formats the supplied `format` string into `s`, replacing each `%s`
/// placeholder with successive entries from `args`. Other format specifiers
/// (e.g. `%d`, `%f`) are accepted but the value is taken verbatim from
/// `args` as well, matching the test-style usage of this function. Returns
/// the number of bytes that the format would have produced (matching the
/// C99 vsnprintf return convention).
pub fn rpl_vsnprintf(
    s: &mut String,
    n: usize,
    format: &str,
    args: &[&str],
) -> i32 {
    let mut out = String::new();
    let mut iter = format.chars().peekable();
    let mut arg_idx = 0usize;

    while let Some(c) = iter.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        // Skip flags / width / precision / length specifiers until we hit
        // an actual conversion character.
        let mut spec_chars: Vec<char> = Vec::new();
        let mut conv = '\0';
        while let Some(&nc) = iter.peek() {
            iter.next();
            if nc.is_ascii_alphabetic() || nc == '%' {
                conv = nc;
                break;
            }
            spec_chars.push(nc);
        }
        match conv {
            '%' => out.push('%'),
            's' | 'd' | 'i' | 'u' | 'x' | 'X' | 'o' | 'f' | 'F' | 'e' | 'E' | 'g' | 'G' | 'c' => {
                if arg_idx < args.len() {
                    out.push_str(args[arg_idx]);
                    arg_idx += 1;
                }
            }
            _ => {
                // Unknown conversion - keep the literal text as-is.
                out.push('%');
                for sc in &spec_chars {
                    out.push(*sc);
                }
                if conv != '\0' {
                    out.push(conv);
                }
            }
        }
    }

    let total_len = out.len() as i32;
    let take = if n == 0 { 0 } else { out.len().min(n.saturating_sub(0)) };
    s.clear();
    s.push_str(&out[..take]);
    total_len
}

pub fn fmtstr(
    s: &mut String,
    _size: usize,
    value: &str,
    width: usize,
    precision: usize,
    flags: i32,
) {
    const PRINT_F_MINUS: i32 = 1 << 0;

    let strln = if precision != 0 && precision < value.len() {
        precision
    } else {
        value.len()
    };
    let padlen = if width > strln { width - strln } else { 0 };

    if (flags & PRINT_F_MINUS) == 0 {
        for _ in 0..padlen {
            s.push(' ');
        }
    }
    s.push_str(&value[..strln]);
    if (flags & PRINT_F_MINUS) != 0 {
        for _ in 0..padlen {
            s.push(' ');
        }
    }
}

pub fn fmtint(
    s: &mut String,
    _size: usize,
    value: i32,
    width: usize,
    precision: usize,
    flags: i32,
) {
    const PRINT_F_MINUS: i32 = 1 << 0;
    const PRINT_F_PLUS: i32 = 1 << 1;
    const PRINT_F_SPACE: i32 = 1 << 2;
    const PRINT_F_ZERO: i32 = 1 << 4;

    let mut sign = String::new();
    let abs_str: String;
    if value < 0 {
        sign.push('-');
        abs_str = format!("{}", (value as i64).unsigned_abs());
    } else if (flags & PRINT_F_PLUS) != 0 {
        sign.push('+');
        abs_str = format!("{}", value);
    } else if (flags & PRINT_F_SPACE) != 0 {
        sign.push(' ');
        abs_str = format!("{}", value);
    } else {
        abs_str = format!("{}", value);
    }

    let mut digits = abs_str;
    while digits.len() < precision {
        digits.insert(0, '0');
    }

    let total = sign.len() + digits.len();
    let padlen = if width > total { width - total } else { 0 };

    if (flags & PRINT_F_MINUS) != 0 {
        s.push_str(&sign);
        s.push_str(&digits);
        for _ in 0..padlen {
            s.push(' ');
        }
    } else if (flags & PRINT_F_ZERO) != 0 {
        s.push_str(&sign);
        for _ in 0..padlen {
            s.push('0');
        }
        s.push_str(&digits);
    } else {
        for _ in 0..padlen {
            s.push(' ');
        }
        s.push_str(&sign);
        s.push_str(&digits);
    }
}

pub fn fmtflt(
    s: &mut String,
    _size: usize,
    value: f64,
    width: usize,
    precision: usize,
    flags: i32,
) {
    const PRINT_F_MINUS: i32 = 1 << 0;

    let prec = if precision == 0 { 6 } else { precision };
    let formatted = format!("{:.*}", prec, value);
    let padlen = if width > formatted.len() {
        width - formatted.len()
    } else {
        0
    };

    if (flags & PRINT_F_MINUS) == 0 {
        for _ in 0..padlen {
            s.push(' ');
        }
    }
    s.push_str(&formatted);
    if (flags & PRINT_F_MINUS) != 0 {
        for _ in 0..padlen {
            s.push(' ');
        }
    }
}

pub fn printsep(s: &mut String, _size: usize) {
    s.push(',');
}

pub fn getnumsep(digits: i32) -> i32 {
    let mut separators = (digits - (if digits % 3 == 0 { 1 } else { 0 })) / 3;
    if separators < 0 {
        separators = 0;
    }
    separators
}

pub fn getexponent(value: f64) -> i32 {
    if value == 0.0 {
        return 0;
    }
    let v = value.abs();
    v.log10().floor() as i32
}

pub fn convert(value: usize, buf: &mut String, base: usize, caps: usize) {
    buf.clear();
    let digits_lower = b"0123456789abcdef";
    let digits_upper = b"0123456789ABCDEF";
    let digit_set = if caps != 0 { digits_upper } else { digits_lower };

    if value == 0 {
        buf.push('0');
        return;
    }
    let mut v = value;
    let mut tmp = String::new();
    while v != 0 {
        tmp.push(digit_set[v % base] as char);
        v /= base;
    }
    // C version stores reversed (low-order first); replicate that.
    buf.push_str(&tmp);
}

pub fn cast(value: f64) -> i32 {
    if value >= i32::MAX as f64 {
        return i32::MAX;
    }
    if value <= i32::MIN as f64 {
        return i32::MIN;
    }
    value as i32
}

pub fn mypow10(exponent: i32) -> f64 {
    let mut result: f64 = 1.0;
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

pub fn rpl_vasprintf(mut s: Vec<String>, format: &str, args: &[&str]) -> i32 {
    let mut buf = String::new();
    let n = rpl_vsnprintf(&mut buf, usize::MAX, format, args);
    s.push(buf);
    n
}

pub fn rpl_asprintf(s: &mut String, format: &str, args: &[&str]) -> i32 {
    rpl_vsnprintf(s, usize::MAX, format, args)
}

pub fn main() {
    // The original C file has an optional TEST_SNPRINTF main; nothing required here.
}
