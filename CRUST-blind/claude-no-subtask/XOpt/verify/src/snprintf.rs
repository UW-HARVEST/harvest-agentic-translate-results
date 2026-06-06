// Format flags (matching C constants).
const PRINT_F_MINUS: i32 = 1 << 0;
const PRINT_F_PLUS: i32 = 1 << 1;
const PRINT_F_SPACE: i32 = 1 << 2;
const PRINT_F_NUM: i32 = 1 << 3;
const PRINT_F_ZERO: i32 = 1 << 4;
const PRINT_F_QUOTE: i32 = 1 << 5;
const PRINT_F_UP: i32 = 1 << 6;
const PRINT_F_UNSIGNED: i32 = 1 << 7;
#[allow(dead_code)]
const PRINT_F_TYPE_G: i32 = 1 << 8;
#[allow(dead_code)]
const PRINT_F_TYPE_E: i32 = 1 << 9;

const PRINT_S_DEFAULT: i32 = 0;
const PRINT_S_FLAGS: i32 = 1;
const PRINT_S_WIDTH: i32 = 2;
const PRINT_S_DOT: i32 = 3;
const PRINT_S_PRECISION: i32 = 4;
const PRINT_S_MOD: i32 = 5;
const PRINT_S_CONV: i32 = 6;

/// Helper that pushes a character onto a `String`, mimicking the C `OUTCHAR`
/// macro semantics: increments `len` always, but only writes the char when
/// `len + 1 < size`.
fn outchar(s: &mut String, len: &mut usize, size: usize, ch: char) {
    if *len + 1 < size {
        s.push(ch);
    }
    *len += 1;
}

/// Replacement vsnprintf-like function. The Rust signature uses
/// `args: &[&str]`, so each `%`-conversion consumes one entry from `args`,
/// parsing it as needed for the relevant type (`%d` -> i64, `%f` -> f64, etc.).
///
/// Returns the number of characters that *would* have been written, excluding
/// any terminator. On error returns -1.
pub fn rpl_vsnprintf(s: &mut String, n: usize, format: &str, args: &[&str]) -> i32 {
    let bytes: Vec<char> = format.chars().collect();
    let mut idx: usize = 0;
    let mut len: usize = 0;
    let mut state = PRINT_S_DEFAULT;
    let mut flags: i32 = 0;
    let mut width: i32 = 0;
    let mut precision: i32 = -1;
    let mut overflow = false;
    let mut arg_idx: usize = 0;

    let next_char = |i: &mut usize| -> char {
        if *i < bytes.len() {
            let c = bytes[*i];
            *i += 1;
            c
        } else {
            '\0'
        }
    };

    let mut ch = next_char(&mut idx);

    while ch != '\0' {
        match state {
            x if x == PRINT_S_DEFAULT => {
                if ch == '%' {
                    state = PRINT_S_FLAGS;
                } else {
                    outchar(s, &mut len, n, ch);
                }
                ch = next_char(&mut idx);
            }
            x if x == PRINT_S_FLAGS => match ch {
                '-' => {
                    flags |= PRINT_F_MINUS;
                    ch = next_char(&mut idx);
                }
                '+' => {
                    flags |= PRINT_F_PLUS;
                    ch = next_char(&mut idx);
                }
                ' ' => {
                    flags |= PRINT_F_SPACE;
                    ch = next_char(&mut idx);
                }
                '#' => {
                    flags |= PRINT_F_NUM;
                    ch = next_char(&mut idx);
                }
                '0' => {
                    flags |= PRINT_F_ZERO;
                    ch = next_char(&mut idx);
                }
                '\'' => {
                    flags |= PRINT_F_QUOTE;
                    ch = next_char(&mut idx);
                }
                _ => {
                    state = PRINT_S_WIDTH;
                }
            },
            x if x == PRINT_S_WIDTH => {
                if ch.is_ascii_digit() {
                    let d = (ch as i32) - ('0' as i32);
                    if width > (i32::MAX - d) / 10 {
                        overflow = true;
                        break;
                    }
                    width = 10 * width + d;
                    ch = next_char(&mut idx);
                } else if ch == '*' {
                    if let Some(arg) = args.get(arg_idx) {
                        arg_idx += 1;
                        let val: i32 = arg.parse().unwrap_or(0);
                        if val < 0 {
                            flags |= PRINT_F_MINUS;
                            width = -val;
                        } else {
                            width = val;
                        }
                    }
                    ch = next_char(&mut idx);
                    state = PRINT_S_DOT;
                } else {
                    state = PRINT_S_DOT;
                }
            }
            x if x == PRINT_S_DOT => {
                if ch == '.' {
                    state = PRINT_S_PRECISION;
                    ch = next_char(&mut idx);
                } else {
                    state = PRINT_S_MOD;
                }
            }
            x if x == PRINT_S_PRECISION => {
                if precision == -1 {
                    precision = 0;
                }
                if ch.is_ascii_digit() {
                    let d = (ch as i32) - ('0' as i32);
                    if precision > (i32::MAX - d) / 10 {
                        overflow = true;
                        break;
                    }
                    precision = 10 * precision + d;
                    ch = next_char(&mut idx);
                } else if ch == '*' {
                    if let Some(arg) = args.get(arg_idx) {
                        arg_idx += 1;
                        let val: i32 = arg.parse().unwrap_or(-1);
                        precision = if val < 0 { -1 } else { val };
                    }
                    ch = next_char(&mut idx);
                    state = PRINT_S_MOD;
                } else {
                    state = PRINT_S_MOD;
                }
            }
            x if x == PRINT_S_MOD => {
                // Skip length modifiers like h, l, L, j, t, z. They don't
                // change anything in our simplified Rust implementation since
                // we always treat numeric arguments as strings to parse.
                match ch {
                    'h' => {
                        ch = next_char(&mut idx);
                        if ch == 'h' {
                            ch = next_char(&mut idx);
                        }
                    }
                    'l' => {
                        ch = next_char(&mut idx);
                        if ch == 'l' {
                            ch = next_char(&mut idx);
                        }
                    }
                    'L' | 'j' | 't' | 'z' => {
                        ch = next_char(&mut idx);
                    }
                    _ => {}
                }
                state = PRINT_S_CONV;
            }
            x if x == PRINT_S_CONV => {
                match ch {
                    'd' | 'i' => {
                        let v: i64 = args
                            .get(arg_idx)
                            .and_then(|a| a.parse::<i64>().ok())
                            .unwrap_or(0);
                        arg_idx += 1;
                        let truncated = v as i32;
                        let mut prec_for_int = precision;
                        if prec_for_int < 0 {
                            prec_for_int = -1;
                        }
                        fmtint_internal(
                            s,
                            &mut len,
                            n,
                            truncated as i64,
                            10,
                            width,
                            prec_for_int,
                            flags,
                        );
                    }
                    'X' => {
                        let f = flags | PRINT_F_UP | PRINT_F_UNSIGNED;
                        let v: u64 = args
                            .get(arg_idx)
                            .and_then(|a| a.parse::<u64>().ok())
                            .unwrap_or(0);
                        arg_idx += 1;
                        fmtint_internal(s, &mut len, n, v as i64, 16, width, precision, f);
                    }
                    'x' => {
                        let f = flags | PRINT_F_UNSIGNED;
                        let v: u64 = args
                            .get(arg_idx)
                            .and_then(|a| a.parse::<u64>().ok())
                            .unwrap_or(0);
                        arg_idx += 1;
                        fmtint_internal(s, &mut len, n, v as i64, 16, width, precision, f);
                    }
                    'o' => {
                        let f = flags | PRINT_F_UNSIGNED;
                        let v: u64 = args
                            .get(arg_idx)
                            .and_then(|a| a.parse::<u64>().ok())
                            .unwrap_or(0);
                        arg_idx += 1;
                        fmtint_internal(s, &mut len, n, v as i64, 8, width, precision, f);
                    }
                    'u' => {
                        let f = flags | PRINT_F_UNSIGNED;
                        let v: u64 = args
                            .get(arg_idx)
                            .and_then(|a| a.parse::<u64>().ok())
                            .unwrap_or(0);
                        arg_idx += 1;
                        fmtint_internal(s, &mut len, n, v as i64, 10, width, precision, f);
                    }
                    'F' | 'f' | 'e' | 'E' | 'g' | 'G' | 'a' | 'A' => {
                        let v: f64 = args
                            .get(arg_idx)
                            .and_then(|a| a.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        arg_idx += 1;
                        let mut f = flags;
                        if ch == 'F' || ch == 'E' || ch == 'G' || ch == 'A' {
                            f |= PRINT_F_UP;
                        }
                        if ch == 'e' || ch == 'E' {
                            f |= PRINT_F_TYPE_E;
                        }
                        if ch == 'g' || ch == 'G' {
                            f |= PRINT_F_TYPE_G;
                        }
                        let mut p = precision;
                        if p == 0 && (ch == 'g' || ch == 'G') {
                            p = 1;
                        }
                        fmtflt_internal(s, &mut len, n, v, width, p, f, &mut overflow);
                        if overflow {
                            break;
                        }
                    }
                    'c' => {
                        let arg = args.get(arg_idx).copied().unwrap_or("");
                        arg_idx += 1;
                        let c = arg.chars().next().unwrap_or('\0');
                        outchar(s, &mut len, n, c);
                    }
                    's' => {
                        let arg = args.get(arg_idx).copied().unwrap_or("(null)");
                        arg_idx += 1;
                        fmtstr_internal(s, &mut len, n, arg, width, precision, flags);
                    }
                    'p' => {
                        let arg = args.get(arg_idx).copied().unwrap_or("(nil)");
                        arg_idx += 1;
                        // Treat as plain string (we don't have raw pointers in args).
                        fmtstr_internal(s, &mut len, n, arg, width, -1, flags);
                    }
                    '%' => {
                        outchar(s, &mut len, n, ch);
                    }
                    _ => {
                        // Skip unknown.
                    }
                }
                ch = next_char(&mut idx);
                state = PRINT_S_DEFAULT;
                flags = 0;
                width = 0;
                precision = -1;
            }
            _ => break,
        }
    }

    if overflow || len >= i32::MAX as usize {
        return -1;
    }
    len as i32
}

/// Public wrapper matching the `fmtstr` signature in the project header.
pub fn fmtstr(
    s: &mut String,
    size: usize,
    value: &str,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let mut len = s.chars().count();
    fmtstr_internal(s, &mut len, size, value, width as i32, precision as i32, flags);
}

fn fmtstr_internal(
    s: &mut String,
    len: &mut usize,
    size: usize,
    value: &str,
    width: i32,
    precision: i32,
    flags: i32,
) {
    let chars: Vec<char> = value.chars().collect();
    let noprecision = precision == -1;

    let mut strln: i32 = 0;
    while (strln as usize) < chars.len() && (noprecision || strln < precision) {
        strln += 1;
    }

    let mut padlen = width - strln;
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

    let mut remaining = precision;
    let mut i = 0usize;
    while i < chars.len() && (noprecision || remaining > 0) {
        outchar(s, len, size, chars[i]);
        i += 1;
        if !noprecision {
            remaining -= 1;
        }
    }
    while padlen < 0 {
        outchar(s, len, size, ' ');
        padlen += 1;
    }
}

/// Public wrapper matching the `fmtint` signature in the project header.
pub fn fmtint(
    s: &mut String,
    size: usize,
    value: i32,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let mut len = s.chars().count();
    fmtint_internal(
        s,
        &mut len,
        size,
        value as i64,
        10,
        width as i32,
        precision as i32,
        flags,
    );
}

fn fmtint_internal(
    s: &mut String,
    len: &mut usize,
    size: usize,
    value: i64,
    base: i32,
    width: i32,
    precision: i32,
    flags: i32,
) {
    let mut sign: char = '\0';
    let mut hexprefix: char = '\0';
    let noprecision = precision == -1;

    let uvalue: u64;
    if (flags & PRINT_F_UNSIGNED) != 0 {
        uvalue = value as u64;
    } else if value < 0 {
        uvalue = value.unsigned_abs();
        sign = '-';
    } else {
        uvalue = value as u64;
        if (flags & PRINT_F_PLUS) != 0 {
            sign = '+';
        } else if (flags & PRINT_F_SPACE) != 0 {
            sign = ' ';
        }
    }

    let mut iconvert = String::new();
    let pos = convert_internal(
        uvalue as u128,
        &mut iconvert,
        43,
        base as u32,
        (flags & PRINT_F_UP) != 0,
    );

    let mut adjusted_precision = precision;

    if (flags & PRINT_F_NUM) != 0 && uvalue != 0 {
        match base {
            8 => {
                if adjusted_precision <= pos {
                    adjusted_precision = pos + 1;
                }
            }
            16 => {
                hexprefix = if (flags & PRINT_F_UP) != 0 { 'X' } else { 'x' };
            }
            _ => {}
        }
    }

    let mut separators: i32 = if (flags & PRINT_F_QUOTE) != 0 { 1 } else { 0 };
    if separators != 0 {
        separators = getnumsep_internal(pos);
    }

    let mut zpadlen = adjusted_precision - pos - separators;
    let mut spadlen = width
        - separators
        - i32::max(adjusted_precision, pos)
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

    let chars: Vec<char> = iconvert.chars().collect();
    let mut p = pos;
    while p > 0 {
        p -= 1;
        if let Some(c) = chars.get(p as usize) {
            outchar(s, len, size, *c);
        }
        if separators > 0 && p > 0 && p % 3 == 0 {
            printsep_internal(s, len, size);
        }
    }
    while spadlen < 0 {
        outchar(s, len, size, ' ');
        spadlen += 1;
    }
}

/// Public wrapper for `fmtflt`.
pub fn fmtflt(
    s: &mut String,
    size: usize,
    value: f64,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let mut len = s.chars().count();
    let mut overflow = false;
    fmtflt_internal(
        s,
        &mut len,
        size,
        value,
        width as i32,
        precision as i32,
        flags,
        &mut overflow,
    );
}

fn fmtflt_internal(
    s: &mut String,
    len: &mut usize,
    size: usize,
    fvalue: f64,
    width: i32,
    precision_in: i32,
    flags_in: i32,
    overflow: &mut bool,
) {
    let mut precision = precision_in;
    let flags = flags_in;
    if precision == -1 {
        precision = 6;
    }

    let mut sign: char = '\0';
    if fvalue < 0.0 {
        sign = '-';
    } else if (flags & PRINT_F_PLUS) != 0 {
        sign = '+';
    } else if (flags & PRINT_F_SPACE) != 0 {
        sign = ' ';
    }

    let infnan: Option<&'static str> = if fvalue.is_nan() {
        Some(if (flags & PRINT_F_UP) != 0 { "NAN" } else { "nan" })
    } else if fvalue.is_infinite() {
        Some(if (flags & PRINT_F_UP) != 0 { "INF" } else { "inf" })
    } else {
        None
    };

    if let Some(text) = infnan {
        let mut buf = String::new();
        if sign != '\0' {
            buf.push(sign);
        }
        buf.push_str(text);
        let buf_len = buf.chars().count() as i32;
        fmtstr_internal(s, len, size, &buf, width, buf_len, flags);
        return;
    }

    let mut estyle = (flags & PRINT_F_TYPE_E) != 0;
    let mut omitzeros = false;
    let mut exponent: i32 = 0;

    if (flags & PRINT_F_TYPE_E) != 0 || (flags & PRINT_F_TYPE_G) != 0 {
        if (flags & PRINT_F_TYPE_G) != 0 {
            precision -= 1;
            if (flags & PRINT_F_NUM) == 0 {
                omitzeros = true;
            }
        }
        exponent = getexponent_internal(fvalue);
        estyle = true;
    }

    // Cap precision to avoid overflow with our cast.
    if precision > 18 {
        precision = 18;
    }
    if precision < 0 {
        precision = 0;
    }

    let mut separators: i32 = if (flags & PRINT_F_QUOTE) != 0 { 1 } else { 0 };

    loop {
        let mut ufvalue = fvalue.abs();
        if estyle {
            ufvalue /= mypow10_internal(exponent);
        }

        let castval = cast_internal(ufvalue);
        if castval == u64::MAX {
            *overflow = true;
            return;
        }
        let mut intpart: u64 = castval;
        let fracpart: u64;
        let mask = mypow10_internal(precision);
        let frac = myround_internal(mask * (ufvalue - intpart as f64));
        if frac as f64 >= mask {
            intpart += 1;
            fracpart = 0;
            if estyle && intpart == 10 {
                intpart = 1;
                exponent += 1;
            }
        } else {
            fracpart = frac;
        }

        if (flags & PRINT_F_TYPE_G) != 0 && estyle && precision + 1 > exponent && exponent >= -4
        {
            precision -= exponent;
            estyle = false;
            continue;
        }

        let mut iconvert = String::new();
        let ipos = convert_internal(intpart as u128, &mut iconvert, 43, 10, false);
        let mut fconvert = String::new();
        let mut fpos = 0;
        if fracpart != 0 {
            fpos = convert_internal(fracpart as u128, &mut fconvert, 43, 10, false);
        }

        let mut leadfraczeros: i32 = precision - fpos;
        let mut omitcount: i32 = 0;
        let mut emit_precision = precision;
        if omitzeros {
            if fpos > 0 {
                let fchars: Vec<char> = fconvert.chars().collect();
                while omitcount < fpos && fchars[omitcount as usize] == '0' {
                    omitcount += 1;
                }
            } else {
                omitcount = emit_precision;
                leadfraczeros = 0;
            }
            emit_precision -= omitcount;
        }

        let emitpoint = emit_precision > 0 || (flags & PRINT_F_NUM) != 0;
        if separators != 0 {
            separators = getnumsep_internal(ipos);
        }

        // Compute exponent representation if estyle.
        let mut econvert = String::new();
        let mut epos = 0;
        if estyle {
            let mut esign = '+';
            let mut exp = exponent;
            if exp < 0 {
                exp = -exp;
                esign = '-';
            }
            let _ = convert_internal(exp as u128, &mut econvert, 4, 10, false);
            // Reverse not needed since chars are stored reversed; we manage explicitly.
            // Need to count digits.
            let digits: Vec<char> = econvert.chars().collect();
            let mut digcount = digits.len();
            if digcount == 1 {
                econvert.push('0');
                digcount += 1;
            }
            econvert.push(esign);
            econvert.push(if (flags & PRINT_F_UP) != 0 { 'E' } else { 'e' });
            epos = (digcount + 2) as i32;
        }

        // Compute padlen.
        let mut padlen = width
            - ipos
            - epos
            - emit_precision
            - separators
            - if emitpoint { 1 } else { 0 }
            - if sign != '\0' { 1 } else { 0 };
        if padlen < 0 {
            padlen = 0;
        }
        let mut local_sign = sign;
        if (flags & PRINT_F_MINUS) != 0 {
            padlen = -padlen;
        } else if (flags & PRINT_F_ZERO) != 0 && padlen > 0 {
            if local_sign != '\0' {
                outchar(s, len, size, local_sign);
                local_sign = '\0';
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
        if local_sign != '\0' {
            outchar(s, len, size, local_sign);
        }

        let ichars: Vec<char> = iconvert.chars().collect();
        let mut ip = ipos;
        while ip > 0 {
            ip -= 1;
            if let Some(c) = ichars.get(ip as usize) {
                outchar(s, len, size, *c);
            }
            if separators > 0 && ip > 0 && ip % 3 == 0 {
                printsep_internal(s, len, size);
            }
        }
        if emitpoint {
            outchar(s, len, size, '.');
        }
        let mut leadz = leadfraczeros;
        while leadz > 0 {
            outchar(s, len, size, '0');
            leadz -= 1;
        }
        let fchars: Vec<char> = fconvert.chars().collect();
        let mut fp = fpos;
        while fp > omitcount {
            fp -= 1;
            if let Some(c) = fchars.get(fp as usize) {
                outchar(s, len, size, *c);
            }
        }
        // Exponent in correct order:
        if estyle {
            let echars: Vec<char> = econvert.chars().collect();
            // The convert produced digits reversed; then '0'(maybe), esign, e/E
            // were appended. We emit from end to start (the C code iterates
            // econvert from position epos down, but we built it differently —
            // emit raw forward order matching what C would emit).
            // C builds: digits reversed in [0..digcount], '0' if needed at digcount,
            // then esign, then 'e'/'E'. C emits from epos down to 0 (reversed),
            // producing 'e'/'E' first, then sign, then padded '0', then digits.
            for c in echars.iter().rev() {
                outchar(s, len, size, *c);
            }
        }
        while padlen < 0 {
            outchar(s, len, size, ' ');
            padlen += 1;
        }
        break;
    }
}

pub fn printsep(s: &mut String, size: usize) {
    let mut len = s.chars().count();
    printsep_internal(s, &mut len, size);
}

fn printsep_internal(s: &mut String, len: &mut usize, size: usize) {
    outchar(s, len, size, ',');
}

pub fn getnumsep(digits: i32) -> i32 {
    getnumsep_internal(digits)
}

fn getnumsep_internal(digits: i32) -> i32 {
    let separators = (digits - if digits % 3 == 0 { 1 } else { 0 }) / 3;
    separators
}

pub fn getexponent(value: f64) -> i32 {
    getexponent_internal(value)
}

fn getexponent_internal(value: f64) -> i32 {
    let mut tmp = value.abs();
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
    let _ = convert_internal(value as u128, buf, 64, base as u32, caps != 0);
}

fn convert_internal(value_in: u128, buf: &mut String, size: usize, base: u32, caps: bool) -> i32 {
    let digits = if caps {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    buf.clear();
    let mut value = value_in;
    let base = base as u128;
    if base == 0 {
        return 0;
    }
    let mut pos: usize = 0;
    loop {
        buf.push(digits[(value % base) as usize] as char);
        value /= base;
        pos += 1;
        if value == 0 || pos >= size {
            break;
        }
    }
    pos as i32
}

pub fn cast(value: f64) -> i32 {
    cast_internal(value) as i32
}

fn cast_internal(value: f64) -> u64 {
    if !value.is_finite() {
        return u64::MAX;
    }
    if value >= u64::MAX as f64 {
        return u64::MAX;
    }
    if value < 0.0 {
        return 0;
    }
    let result = value as u64;
    if (result as f64) <= value {
        result
    } else {
        result.saturating_sub(1)
    }
}

fn myround_internal(value: f64) -> u64 {
    let intpart = cast_internal(value);
    if intpart == u64::MAX {
        return u64::MAX;
    }
    let frac = value - (intpart as f64);
    if frac < 0.5 { intpart } else { intpart + 1 }
}

pub fn mypow10(exponent: i32) -> f64 {
    mypow10_internal(exponent)
}

fn mypow10_internal(exponent: i32) -> f64 {
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

pub fn rpl_vasprintf(_s: Vec<String>, _format: &str, _args: &[&str]) -> i32 {
    // Not used; provided for signature parity.
    0
}

pub fn rpl_asprintf(s: &mut String, format: &str, args: &[&str]) -> i32 {
    s.clear();
    rpl_vsnprintf(s, usize::MAX, format, args)
}

pub fn main() {
    // Test driver entry point — no-op in the Rust port.
}
