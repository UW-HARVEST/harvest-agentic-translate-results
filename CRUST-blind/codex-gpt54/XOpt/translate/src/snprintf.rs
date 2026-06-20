use std::fmt::Write as _;

const PRINT_F_MINUS: i32 = 0x01;
const PRINT_F_ZERO: i32 = 0x10;

pub fn rpl_vsnprintf(s: &mut String, n: usize, format: &str, args: &[&str]) -> i32 {
    let rendered = render_format(format, args);
    let total_len = rendered.chars().count() as i32;
    s.clear();
    if n == 0 {
        return total_len;
    }

    let truncated: String = rendered.chars().take(n.saturating_sub(1)).collect();
    s.push_str(&truncated);
    total_len
}

pub fn fmtstr(s: &mut String, size: usize, value: &str, width: usize, precision: usize, flags: i32) {
    let has_precision = precision != usize::MAX;
    let truncated: String = if has_precision {
        value.chars().take(precision).collect()
    } else {
        value.to_owned()
    };
    append_padded(s, size, &truncated, width, flags);
}

pub fn fmtint(
    s: &mut String,
    size: usize,
    value: i32,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let negative = value < 0;
    let mut digits = value.unsigned_abs().to_string();
    if precision != usize::MAX && digits.len() < precision {
        digits = format!("{}{}", "0".repeat(precision - digits.len()), digits);
    }
    if negative {
        digits.insert(0, '-');
    }
    append_padded(s, size, &digits, width, flags);
}

pub fn fmtflt(
    s: &mut String,
    size: usize,
    value: f64,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let formatted = if precision == usize::MAX {
        format!("{value}")
    } else {
        format!("{value:.precision$}")
    };
    append_padded(s, size, &formatted, width, flags);
}

pub fn printsep(s: &mut String, size: usize) {
    if s.len() < size {
        s.push(',');
    }
}

pub fn getnumsep(digits: i32) -> i32 {
    if digits <= 0 {
        0
    } else {
        (digits - if digits % 3 == 0 { 1 } else { 0 }) / 3
    }
}

pub fn getexponent(value: f64) -> i32 {
    let mut tmp = value.abs();
    let mut exponent = 0;
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
    let digits = if caps != 0 {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };

    buf.clear();
    let mut n = value;
    loop {
        let digit = digits[n % base] as char;
        buf.push(digit);
        n /= base;
        if n == 0 {
            break;
        }
    }
}

pub fn cast(value: f64) -> i32 {
    if value.is_nan() {
        0
    } else if value >= i32::MAX as f64 {
        i32::MAX
    } else if value <= i32::MIN as f64 {
        i32::MIN
    } else {
        value.trunc() as i32
    }
}

pub fn mypow10(exponent: i32) -> f64 {
    let mut result = 1.0;
    let mut exponent = exponent;
    while exponent > 0 {
        result *= 10.0;
        exponent -= 1;
    }
    while exponent < 0 {
        result /= 10.0;
        exponent += 1;
    }
    result
}

pub fn rpl_vasprintf(s: Vec<String>, format: &str, args: &[&str]) -> i32 {
    let _ = s;
    render_format(format, args).chars().count() as i32
}

pub fn rpl_asprintf(s: &mut String, format: &str, args: &[&str]) -> i32 {
    let rendered = render_format(format, args);
    let total_len = rendered.chars().count() as i32;
    s.clear();
    s.push_str(&rendered);
    total_len
}

pub fn main() {}

fn render_format(format: &str, args: &[&str]) -> String {
    let mut rendered = String::new();
    let mut chars = format.chars().peekable();
    let mut arg_index = 0usize;

    while let Some(ch) = chars.next() {
        if ch != '%' {
            rendered.push(ch);
            continue;
        }

        if chars.peek() == Some(&'%') {
            chars.next();
            rendered.push('%');
            continue;
        }

        let mut spec = String::from("%");
        while let Some(&next) = chars.peek() {
            spec.push(next);
            chars.next();
            if next.is_ascii_alphabetic() || next == '%' {
                break;
            }
        }

        if let Some(arg) = args.get(arg_index) {
            arg_index += 1;
            rendered.push_str(&apply_string_spec(&spec, arg));
        } else {
            rendered.push_str(&spec);
        }
    }

    rendered
}

fn apply_string_spec(spec: &str, arg: &str) -> String {
    let conv = spec.chars().last().unwrap_or('s');
    if conv == '%' {
        return "%".to_string();
    }

    let body = &spec[1..spec.len().saturating_sub(1)];
    let (flags, width, precision) = parse_format_body(body);
    let base = match conv {
        'x' => i64::from_str_radix(arg.trim_start_matches("0x"), 16)
            .ok()
            .map(|n| format!("{n:x}"))
            .unwrap_or_else(|| arg.to_lowercase()),
        'X' => i64::from_str_radix(arg.trim_start_matches("0x"), 16)
            .ok()
            .map(|n| format!("{n:X}"))
            .unwrap_or_else(|| arg.to_uppercase()),
        'd' | 'i' | 'u' | 'o' => arg.to_string(),
        'f' | 'F' | 'e' | 'E' | 'g' | 'G' => {
            if let Ok(value) = arg.parse::<f64>() {
                let precision = precision.unwrap_or(6);
                match conv {
                    'e' => format!("{value:.precision$e}"),
                    'E' => format!("{value:.precision$E}"),
                    'f' | 'F' => format!("{value:.precision$}"),
                    'g' => format!("{value:.precision$}"),
                    'G' => format!("{value:.precision$}").to_uppercase(),
                    _ => arg.to_string(),
                }
            } else {
                arg.to_string()
            }
        }
        _ => {
            if let Some(precision) = precision {
                arg.chars().take(precision).collect()
            } else {
                arg.to_string()
            }
        }
    };

    if let Some(width) = width {
        let mut padded = String::new();
        append_padded(&mut padded, usize::MAX, &base, width, flags);
        padded
    } else {
        base
    }
}

fn parse_format_body(body: &str) -> (i32, Option<usize>, Option<usize>) {
    let bytes = body.as_bytes();
    let mut idx = 0usize;
    let mut flags = 0i32;

    while idx < bytes.len() {
        match bytes[idx] {
            b'-' => flags |= PRINT_F_MINUS,
            b'0' => flags |= PRINT_F_ZERO,
            b'+' | b' ' | b'#' | b'\'' => {}
            _ => break,
        }
        idx += 1;
    }

    let width_start = idx;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
    }
    let width = if idx > width_start {
        body[width_start..idx].parse::<usize>().ok()
    } else {
        None
    };

    let precision = if idx < bytes.len() && bytes[idx] == b'.' {
        idx += 1;
        let precision_start = idx;
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            idx += 1;
        }
        Some(body[precision_start..idx].parse::<usize>().unwrap_or(0))
    } else {
        None
    };

    (flags, width, precision)
}

fn append_padded(s: &mut String, size: usize, value: &str, width: usize, flags: i32) {
    let pad = width.saturating_sub(value.chars().count());
    let pad_char = if (flags & PRINT_F_ZERO) != 0 && (flags & PRINT_F_MINUS) == 0 {
        '0'
    } else {
        ' '
    };

    let mut out = String::new();
    if (flags & PRINT_F_MINUS) == 0 {
        for _ in 0..pad {
            out.push(pad_char);
        }
    }
    let _ = write!(out, "{value}");
    if (flags & PRINT_F_MINUS) != 0 {
        for _ in 0..pad {
            out.push(' ');
        }
    }

    if size == usize::MAX {
        s.push_str(&out);
    } else {
        let remaining = size.saturating_sub(s.len());
        s.extend(out.chars().take(remaining));
    }
}
