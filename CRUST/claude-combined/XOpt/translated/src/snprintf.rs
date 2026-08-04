// Pure-Rust translation of the rpl_vsnprintf / rpl_asprintf family from
// snprintf.c. The signatures here use `&[&str]` for the variadic arguments,
// which means we can only support a small subset of format conversions
// (essentially `%s`, `%d`, `%c`, `%x` and `%%`). This is sufficient for the
// internal error formatting used by the xopt library.

const PRINT_S_DEFAULT: i32 = 0;
const PRINT_S_FLAGS: i32 = 1;
const PRINT_S_WIDTH: i32 = 2;
const PRINT_S_DOT: i32 = 3;
const PRINT_S_PRECISION: i32 = 4;
const PRINT_S_MOD: i32 = 5;
const PRINT_S_CONV: i32 = 6;

const PRINT_F_MINUS: i32 = 1 << 0;
const PRINT_F_PLUS: i32 = 1 << 1;
const PRINT_F_SPACE: i32 = 1 << 2;
const PRINT_F_NUM: i32 = 1 << 3;
const PRINT_F_ZERO: i32 = 1 << 4;
const PRINT_F_QUOTE: i32 = 1 << 5;
const PRINT_F_UP: i32 = 1 << 6;
const PRINT_F_UNSIGNED: i32 = 1 << 7;

fn outchar(s: &mut String, len: &mut usize, size: usize, ch: char) {
    if *len + 1 < size {
        // Replace the character at position *len with ch.
        let mut chars: Vec<char> = s.chars().collect();
        if *len < chars.len() {
            chars[*len] = ch;
        } else {
            // Extend with spaces if necessary, then push.
            while chars.len() < *len {
                chars.push(' ');
            }
            chars.push(ch);
        }
        *s = chars.into_iter().collect();
    }
    *len += 1;
}

/// Mimics the C `rpl_vsnprintf`. Because Rust's slice of `&str` cannot carry
/// type information, we treat every `%` conversion as a string substitution.
/// Only `%s`, `%c`, `%d`, `%i`, `%x`, `%X`, `%u`, and `%%` are supported.
pub fn rpl_vsnprintf(s: &mut String, n: usize, format: &str, args: &[&str]) -> i32 {
    let _ = n; // size limit is informational here (we always grow s).
    s.clear();

    let bytes = format.as_bytes();
    let mut i = 0usize;
    let mut argi = 0usize;
    let mut state = PRINT_S_DEFAULT;
    let mut flags = 0i32;
    let mut width = 0usize;
    let mut precision: i32 = -1;

    while i < bytes.len() {
        let ch = bytes[i] as char;
        match state {
            x if x == PRINT_S_DEFAULT => {
                if ch == '%' {
                    state = PRINT_S_FLAGS;
                    flags = 0;
                    width = 0;
                    precision = -1;
                } else {
                    s.push(ch);
                }
                i += 1;
            }
            x if x == PRINT_S_FLAGS => match ch {
                '-' => {
                    flags |= PRINT_F_MINUS;
                    i += 1;
                }
                '+' => {
                    flags |= PRINT_F_PLUS;
                    i += 1;
                }
                ' ' => {
                    flags |= PRINT_F_SPACE;
                    i += 1;
                }
                '#' => {
                    flags |= PRINT_F_NUM;
                    i += 1;
                }
                '0' => {
                    flags |= PRINT_F_ZERO;
                    i += 1;
                }
                '\'' => {
                    flags |= PRINT_F_QUOTE;
                    i += 1;
                }
                _ => {
                    state = PRINT_S_WIDTH;
                }
            },
            x if x == PRINT_S_WIDTH => {
                if ch.is_ascii_digit() {
                    width = width * 10 + (ch as usize - '0' as usize);
                    i += 1;
                } else {
                    state = PRINT_S_DOT;
                }
            }
            x if x == PRINT_S_DOT => {
                if ch == '.' {
                    state = PRINT_S_PRECISION;
                    i += 1;
                } else {
                    state = PRINT_S_MOD;
                }
            }
            x if x == PRINT_S_PRECISION => {
                if precision == -1 {
                    precision = 0;
                }
                if ch.is_ascii_digit() {
                    precision = precision * 10 + (ch as i32 - '0' as i32);
                    i += 1;
                } else {
                    state = PRINT_S_MOD;
                }
            }
            x if x == PRINT_S_MOD => {
                // Skip length modifiers.
                match ch {
                    'h' | 'l' | 'L' | 'j' | 't' | 'z' => {
                        i += 1;
                        if i < bytes.len() && (bytes[i] as char) == ch {
                            i += 1;
                        }
                    }
                    _ => {}
                }
                state = PRINT_S_CONV;
            }
            x if x == PRINT_S_CONV => {
                match ch {
                    '%' => s.push('%'),
                    's' | 'c' | 'd' | 'i' | 'u' | 'x' | 'X' | 'o' | 'p' | 'f' | 'e' | 'g'
                    | 'E' | 'G' | 'F' => {
                        if argi < args.len() {
                            let v = args[argi];
                            argi += 1;
                            // Apply width / precision in a simplified way.
                            let mut piece = String::from(v);
                            if precision >= 0 && (precision as usize) < piece.chars().count()
                                && (ch == 's')
                            {
                                piece = piece.chars().take(precision as usize).collect();
                            }
                            let padlen = if width > piece.chars().count() {
                                width - piece.chars().count()
                            } else {
                                0
                            };
                            if (flags & PRINT_F_MINUS) != 0 {
                                s.push_str(&piece);
                                for _ in 0..padlen {
                                    s.push(' ');
                                }
                            } else {
                                let pad_ch = if (flags & PRINT_F_ZERO) != 0
                                    && (ch == 'd' || ch == 'i' || ch == 'u' || ch == 'x'
                                        || ch == 'X' || ch == 'o')
                                {
                                    '0'
                                } else {
                                    ' '
                                };
                                for _ in 0..padlen {
                                    s.push(pad_ch);
                                }
                                s.push_str(&piece);
                            }
                        }
                    }
                    'n' => {
                        // Side-effect not supported.
                    }
                    _ => {}
                }
                i += 1;
                state = PRINT_S_DEFAULT;
                flags = 0;
                width = 0;
                precision = -1;
            }
            _ => {
                i += 1;
            }
        }
    }

    s.chars().count() as i32
}

pub fn fmtstr(
    s: &mut String,
    size: usize,
    value: &str,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let mut len: usize = s.chars().count();
    let noprecision = precision == 0; // we treat 0 as "no precision".

    let mut strln: usize = 0;
    for ch in value.chars() {
        if ch == '\0' {
            break;
        }
        if !noprecision && strln >= precision {
            break;
        }
        strln += 1;
    }

    let mut padlen: i32 = width as i32 - strln as i32;
    if padlen < 0 {
        padlen = 0;
    }
    if (flags & PRINT_F_MINUS) != 0 {
        padlen = -padlen;
    }

    while padlen > 0 {
        outchar(s, &mut len, size, ' ');
        padlen -= 1;
    }
    let mut count = 0usize;
    for ch in value.chars() {
        if ch == '\0' {
            break;
        }
        if !noprecision && count >= precision {
            break;
        }
        outchar(s, &mut len, size, ch);
        count += 1;
    }
    while padlen < 0 {
        outchar(s, &mut len, size, ' ');
        padlen += 1;
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
    let mut text = if (flags & PRINT_F_UNSIGNED) != 0 {
        (value as u32).to_string()
    } else {
        value.to_string()
    };
    let mut len = s.chars().count();
    if (precision as i32) > text.chars().count() as i32 {
        let pad = precision - text.chars().count();
        let mut prefix = String::new();
        for _ in 0..pad {
            prefix.push('0');
        }
        text = prefix + &text;
    }
    let padlen = if width > text.chars().count() {
        width - text.chars().count()
    } else {
        0
    };
    if (flags & PRINT_F_MINUS) != 0 {
        for ch in text.chars() {
            outchar(s, &mut len, size, ch);
        }
        for _ in 0..padlen {
            outchar(s, &mut len, size, ' ');
        }
    } else {
        let pad_ch = if (flags & PRINT_F_ZERO) != 0 { '0' } else { ' ' };
        for _ in 0..padlen {
            outchar(s, &mut len, size, pad_ch);
        }
        for ch in text.chars() {
            outchar(s, &mut len, size, ch);
        }
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
    let prec = if precision == 0 { 6 } else { precision };
    let text = format!("{:.*}", prec, value);
    let mut len = s.chars().count();
    let padlen = if width > text.chars().count() {
        width - text.chars().count()
    } else {
        0
    };
    if (flags & PRINT_F_MINUS) != 0 {
        for ch in text.chars() {
            outchar(s, &mut len, size, ch);
        }
        for _ in 0..padlen {
            outchar(s, &mut len, size, ' ');
        }
    } else {
        let pad_ch = if (flags & PRINT_F_ZERO) != 0 { '0' } else { ' ' };
        for _ in 0..padlen {
            outchar(s, &mut len, size, pad_ch);
        }
        for ch in text.chars() {
            outchar(s, &mut len, size, ch);
        }
    }
}

pub fn printsep(s: &mut String, size: usize) {
    let mut len = s.chars().count();
    outchar(s, &mut len, size, ',');
}

pub fn getnumsep(digits: i32) -> i32 {
    let separators = (digits - if digits % 3 == 0 { 1 } else { 0 }) / 3;
    separators
}

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

pub fn convert(value: usize, buf: &mut String, base: usize, caps: usize) {
    let digits = if caps != 0 {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    let mut v = value;
    buf.clear();
    if v == 0 {
        buf.push('0');
        return;
    }
    let mut tmp = Vec::<u8>::new();
    while v != 0 {
        tmp.push(digits[v % base]);
        v /= base;
    }
    for &b in tmp.iter().rev() {
        buf.push(b as char);
    }
}

pub fn cast(value: f64) -> i32 {
    if value >= i32::MAX as f64 {
        return i32::MAX;
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
    // Not used by xopt; provide a benign implementation.
    -1
}

pub fn rpl_asprintf(s: &mut String, format: &str, args: &[&str]) -> i32 {
    rpl_vsnprintf(s, usize::MAX, format, args)
}

pub fn main() {
    // Test entry point – nothing to do.
}
