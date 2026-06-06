/// Simplified Rust port of the rpl_* snprintf family from snprintf.c.
///
/// The Rust signatures only accept string arguments, so this implementation
/// performs basic printf-style format-substitution where every conversion
/// specifier is treated as a string argument.

fn parse_spec(bytes: &[u8], start: usize) -> (usize, u8) {
    // Returns (position-after-spec, conversion-character)
    let mut j = start;
    // skip flags
    while j < bytes.len() && b"-+ #0'".contains(&bytes[j]) {
        j += 1;
    }
    // skip width (digits or '*')
    while j < bytes.len() && (bytes[j].is_ascii_digit() || bytes[j] == b'*') {
        j += 1;
    }
    // skip precision
    if j < bytes.len() && bytes[j] == b'.' {
        j += 1;
        while j < bytes.len() && (bytes[j].is_ascii_digit() || bytes[j] == b'*') {
            j += 1;
        }
    }
    // skip length modifiers
    while j < bytes.len() && b"hlLjzt".contains(&bytes[j]) {
        j += 1;
    }
    if j < bytes.len() {
        (j + 1, bytes[j])
    } else {
        (j, 0)
    }
}

pub fn rpl_vsnprintf(
    s: &mut String,
    n: usize,
    format: &str,
    args: &[&str],
) -> i32 {
    s.clear();
    let bytes = format.as_bytes();
    let mut arg_idx = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'%' && i + 1 < bytes.len() {
            let (next_i, conv) = parse_spec(bytes, i + 1);
            if conv == 0 {
                // malformed - just emit literally
                s.push(c as char);
                i += 1;
                continue;
            }
            if conv == b'%' {
                s.push('%');
            } else {
                if arg_idx < args.len() {
                    s.push_str(args[arg_idx]);
                    arg_idx += 1;
                }
            }
            i = next_i;
        } else {
            s.push(c as char);
            i += 1;
        }
    }
    if n > 0 && s.len() >= n {
        s.truncate(n - 1);
    }
    s.len() as i32
}

pub fn fmtstr(
    s: &mut String,
    size: usize,
    value: &str,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let _ = size;
    // PRINT_F_MINUS = 1 (left-justify)
    let left_justify = (flags & 1) != 0;

    let v = if precision > 0 && precision < value.len() {
        &value[..precision]
    } else {
        value
    };

    let pad = if width > v.len() { width - v.len() } else { 0 };

    if !left_justify {
        for _ in 0..pad {
            s.push(' ');
        }
    }
    s.push_str(v);
    if left_justify {
        for _ in 0..pad {
            s.push(' ');
        }
    }
}

pub fn fmtint(
    s: &mut String,
    size: usize,
    value: i32,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let _ = size;
    let left_justify = (flags & 1) != 0;
    let plus = (flags & 2) != 0;
    let space = (flags & 4) != 0;
    let zero = (flags & 16) != 0;

    let mut sign = String::new();
    let abs_str;
    if value < 0 {
        sign.push('-');
        abs_str = (-(value as i64)).to_string();
    } else {
        if plus {
            sign.push('+');
        } else if space {
            sign.push(' ');
        }
        abs_str = (value as i64).to_string();
    }

    let digits = if precision > abs_str.len() {
        let mut t = String::new();
        for _ in 0..(precision - abs_str.len()) {
            t.push('0');
        }
        t.push_str(&abs_str);
        t
    } else {
        abs_str
    };

    let total = sign.len() + digits.len();
    let pad = if width > total { width - total } else { 0 };

    if left_justify {
        s.push_str(&sign);
        s.push_str(&digits);
        for _ in 0..pad {
            s.push(' ');
        }
    } else if zero && precision == 0 {
        s.push_str(&sign);
        for _ in 0..pad {
            s.push('0');
        }
        s.push_str(&digits);
    } else {
        for _ in 0..pad {
            s.push(' ');
        }
        s.push_str(&sign);
        s.push_str(&digits);
    }
}

pub fn fmtflt(
    s: &mut String,
    size: usize,
    value: f64,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let _ = size;
    let left_justify = (flags & 1) != 0;
    let plus = (flags & 2) != 0;
    let space = (flags & 4) != 0;

    let prec = if precision == 0 { 6 } else { precision };
    let mut sign = String::new();
    if value < 0.0 {
        sign.push('-');
    } else if plus {
        sign.push('+');
    } else if space {
        sign.push(' ');
    }

    let abs = value.abs();
    let formatted = format!("{:.*}", prec, abs);

    let total = sign.len() + formatted.len();
    let pad = if width > total { width - total } else { 0 };

    if left_justify {
        s.push_str(&sign);
        s.push_str(&formatted);
        for _ in 0..pad {
            s.push(' ');
        }
    } else {
        for _ in 0..pad {
            s.push(' ');
        }
        s.push_str(&sign);
        s.push_str(&formatted);
    }
}

pub fn printsep(s: &mut String, size: usize) {
    let _ = size;
    s.push(',');
}

pub fn getnumsep(digits: i32) -> i32 {
    let extra = if digits % 3 == 0 { 1 } else { 0 };
    (digits - extra) / 3
}

pub fn getexponent(value: f64) -> i32 {
    if value == 0.0 {
        return 0;
    }
    let mut tmp = value.abs();
    let mut exponent = 0i32;
    while tmp < 1.0 && tmp > 0.0 && exponent > -99 {
        tmp *= 10.0;
        exponent -= 1;
    }
    while tmp >= 10.0 && exponent < 99 {
        tmp /= 10.0;
        exponent += 1;
    }
    exponent
}

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
    let mut tmp = Vec::new();
    while v != 0 {
        tmp.push(digits[v % base] as char);
        v /= base;
    }
    for c in tmp.into_iter().rev() {
        buf.push(c);
    }
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

pub fn rpl_vasprintf(s: Vec<String>, format: &str, args: &[&str]) -> i32 {
    let _ = s;
    let mut tmp = String::new();
    rpl_vsnprintf(&mut tmp, usize::MAX, format, args)
}

pub fn rpl_asprintf(s: &mut String, format: &str, args: &[&str]) -> i32 {
    rpl_vsnprintf(s, usize::MAX, format, args)
}

pub fn main() {
    // Self-test stub: nothing to do for the simplified Rust port.
}
