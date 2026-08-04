// Pure Rust translation of snprintf.c
//
// The Rust signatures are simplified compared to the original C code (which
// uses C varargs). Here `args` is a list of pre-stringified arguments.
// For numeric conversions (`%d`, `%i`, `%f`, ...), the corresponding string
// is parsed.

const PRINT_F_MINUS: i32 = 1 << 0;
const PRINT_F_PLUS: i32 = 1 << 1;
const PRINT_F_SPACE: i32 = 1 << 2;
const PRINT_F_NUM: i32 = 1 << 3;
const PRINT_F_ZERO: i32 = 1 << 4;
const PRINT_F_QUOTE: i32 = 1 << 5;
const PRINT_F_UP: i32 = 1 << 6;
const PRINT_F_UNSIGNED: i32 = 1 << 7;
const PRINT_F_TYPE_G: i32 = 1 << 8;
const PRINT_F_TYPE_E: i32 = 1 << 9;

#[inline]
fn outchar(s: &mut String, len: &mut usize, size: usize, ch: char) {
    if *len + 1 < size {
        // Append the char only if within buffer size
        s.push(ch);
    } else if size > 0 && *len < size {
        // Within size bound, even if not strictly less than size-1
        s.push(ch);
    }
    // Always count toward len (matches C OUTCHAR semantics for return value)
    *len += 1;
}

pub fn rpl_vsnprintf(
    s: &mut String,
    n: usize,
    format: &str,
    args: &[&str],
) -> i32 {
    s.clear();
    let bytes: Vec<char> = format.chars().collect();
    let mut i = 0usize;
    let mut len: usize = 0;
    let mut arg_idx = 0usize;

    while i < bytes.len() {
        let ch = bytes[i];
        if ch != '%' {
            outchar(s, &mut len, n, ch);
            i += 1;
            continue;
        }
        i += 1;
        if i >= bytes.len() {
            break;
        }
        // Parse flags
        let mut flags: i32 = 0;
        loop {
            if i >= bytes.len() {
                break;
            }
            match bytes[i] {
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
        // Parse width
        let mut width: i32 = 0;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            width = width * 10 + (bytes[i] as i32 - '0' as i32);
            i += 1;
        }
        // Parse precision
        let mut precision: i32 = -1;
        if i < bytes.len() && bytes[i] == '.' {
            i += 1;
            precision = 0;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                precision = precision * 10 + (bytes[i] as i32 - '0' as i32);
                i += 1;
            }
        }
        // Skip length modifiers (h, l, L, j, t, z)
        while i < bytes.len() && matches!(bytes[i], 'h' | 'l' | 'L' | 'j' | 't' | 'z') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let conv = bytes[i];
        i += 1;
        match conv {
            'd' | 'i' => {
                let arg = args.get(arg_idx).copied().unwrap_or("0");
                arg_idx += 1;
                let v: i32 = parse_int(arg);
                let mut tmp = String::new();
                fmtint(
                    &mut tmp,
                    usize::MAX,
                    v,
                    width as usize,
                    if precision < 0 { usize::MAX } else { precision as usize },
                    flags,
                );
                for c in tmp.chars() {
                    outchar(s, &mut len, n, c);
                }
            }
            'u' | 'x' | 'X' | 'o' => {
                let arg = args.get(arg_idx).copied().unwrap_or("0");
                arg_idx += 1;
                let v: i32 = parse_int(arg);
                let base: u32 = match conv {
                    'x' | 'X' => 16,
                    'o' => 8,
                    _ => 10,
                };
                let caps = conv == 'X';
                let s_lower = format_unsigned(v as u32, base, caps);
                for c in s_lower.chars() {
                    outchar(s, &mut len, n, c);
                }
                let _ = flags;
            }
            'f' | 'F' | 'e' | 'E' | 'g' | 'G' => {
                let arg = args.get(arg_idx).copied().unwrap_or("0");
                arg_idx += 1;
                let v: f64 = arg.parse().unwrap_or(0.0);
                let mut tmp = String::new();
                fmtflt(
                    &mut tmp,
                    usize::MAX,
                    v,
                    width as usize,
                    if precision < 0 { 6 } else { precision as usize },
                    flags
                        | (if conv == 'e' || conv == 'E' { PRINT_F_TYPE_E } else { 0 })
                        | (if conv == 'g' || conv == 'G' { PRINT_F_TYPE_G } else { 0 })
                        | (if conv == 'F' || conv == 'E' || conv == 'G' { PRINT_F_UP } else { 0 }),
                );
                for c in tmp.chars() {
                    outchar(s, &mut len, n, c);
                }
            }
            's' => {
                let arg = args.get(arg_idx).copied().unwrap_or("");
                arg_idx += 1;
                let mut tmp = String::new();
                fmtstr(
                    &mut tmp,
                    usize::MAX,
                    arg,
                    width as usize,
                    if precision < 0 { usize::MAX } else { precision as usize },
                    flags,
                );
                for c in tmp.chars() {
                    outchar(s, &mut len, n, c);
                }
            }
            'c' => {
                let arg = args.get(arg_idx).copied().unwrap_or("");
                arg_idx += 1;
                if let Some(c) = arg.chars().next() {
                    outchar(s, &mut len, n, c);
                }
            }
            '%' => outchar(s, &mut len, n, '%'),
            _ => { /* skip unknown */ }
        }
    }

    if len < n && s.len() < n {
        // already null-terminated implicit in Rust strings
    }
    len as i32
}

fn parse_int(arg: &str) -> i32 {
    let arg = arg.trim();
    if arg.is_empty() {
        return 0;
    }
    // strtol with base 0 detects 0x prefix and 0 octal prefix
    if let Some(rest) = arg.strip_prefix("0x").or_else(|| arg.strip_prefix("0X")) {
        return i32::from_str_radix(rest, 16).unwrap_or(0);
    }
    if let Some(rest) = arg.strip_prefix("-0x").or_else(|| arg.strip_prefix("-0X")) {
        return -i32::from_str_radix(rest, 16).unwrap_or(0);
    }
    if arg.starts_with('0') && arg.len() > 1 && arg.chars().nth(1).unwrap().is_ascii_digit() {
        if let Ok(v) = i32::from_str_radix(&arg[1..], 8) {
            return v;
        }
    }
    arg.parse().unwrap_or(0)
}

fn format_unsigned(value: u32, base: u32, caps: bool) -> String {
    if value == 0 {
        return "0".to_string();
    }
    let digits_l = b"0123456789abcdef";
    let digits_u = b"0123456789ABCDEF";
    let digits = if caps { digits_u } else { digits_l };
    let mut out = String::new();
    let mut v = value;
    while v != 0 {
        out.insert(0, digits[(v % base) as usize] as char);
        v /= base;
    }
    out
}

pub fn fmtstr(
    s: &mut String,
    size: usize,
    value: &str,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let mut len: usize = 0;
    // If precision is usize::MAX, treat as "no precision"
    let noprecision = precision == usize::MAX;

    let strln: usize = if noprecision {
        value.chars().count()
    } else {
        value.chars().take(precision).count()
    };

    let mut padlen: i64 = (width as i64) - (strln as i64);
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

    let mut count: i64 = if noprecision { i64::MAX } else { precision as i64 };
    for c in value.chars() {
        if count <= 0 {
            break;
        }
        outchar(s, &mut len, size, c);
        count -= 1;
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
    let mut len: usize = 0;

    // We treat precision == usize::MAX as "no precision specified".
    let noprecision = precision == usize::MAX;

    let mut sign: char = '\0';
    let uvalue: u64;
    if (flags & PRINT_F_UNSIGNED) != 0 {
        uvalue = value as u32 as u64;
    } else if value < 0 {
        // safely compute absolute value as u64
        uvalue = (value as i64).unsigned_abs();
        sign = '-';
    } else {
        uvalue = value as u32 as u64;
        if (flags & PRINT_F_PLUS) != 0 {
            sign = '+';
        } else if (flags & PRINT_F_SPACE) != 0 {
            sign = ' ';
        }
    }

    // convert
    let mut iconvert: Vec<char> = Vec::new();
    let caps = (flags & PRINT_F_UP) != 0;
    let digits_l = b"0123456789abcdef";
    let digits_u = b"0123456789ABCDEF";
    let digits = if caps { digits_u } else { digits_l };
    let base: u64 = 10;
    let mut v = uvalue;
    if v == 0 {
        iconvert.push('0');
    }
    while v != 0 {
        iconvert.push(digits[(v % base) as usize] as char);
        v /= base;
    }
    let pos = iconvert.len();

    let separators_flag = (flags & PRINT_F_QUOTE) != 0;
    let separators = if separators_flag { getnumsep(pos as i32) as i64 } else { 0 };

    let precision_val = if noprecision { 0i64 } else { precision as i64 };

    let mut zpadlen: i64 = precision_val - (pos as i64) - separators;
    let mut spadlen: i64 = (width as i64)
        - separators
        - if precision_val > pos as i64 { precision_val } else { pos as i64 }
        - if sign != '\0' { 1 } else { 0 };

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
        outchar(s, &mut len, size, ' ');
        spadlen -= 1;
    }
    if sign != '\0' {
        outchar(s, &mut len, size, sign);
    }
    while zpadlen > 0 {
        outchar(s, &mut len, size, '0');
        zpadlen -= 1;
    }
    let mut p = pos;
    let sep_remaining = separators;
    while p > 0 {
        p -= 1;
        outchar(s, &mut len, size, iconvert[p]);
        if sep_remaining > 0 && p > 0 && p % 3 == 0 {
            printsep(s, size);
            len += 1;
        }
    }
    while spadlen < 0 {
        outchar(s, &mut len, size, ' ');
        spadlen += 1;
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
    let mut len: usize = 0;
    let mut precision = precision as i32;
    if precision < 0 {
        precision = 6;
    }

    let mut sign: char = '\0';
    if value < 0.0 {
        sign = '-';
    } else if (flags & PRINT_F_PLUS) != 0 {
        sign = '+';
    } else if (flags & PRINT_F_SPACE) != 0 {
        sign = ' ';
    }

    if value.is_nan() {
        let label = if (flags & PRINT_F_UP) != 0 { "NAN" } else { "nan" };
        let mut tmp = String::new();
        if sign != '\0' {
            tmp.push(sign);
        }
        tmp.push_str(label);
        fmtstr(s, size, &tmp, width, tmp.chars().count(), flags);
        return;
    }
    if value.is_infinite() {
        let label = if (flags & PRINT_F_UP) != 0 { "INF" } else { "inf" };
        let mut tmp = String::new();
        if sign != '\0' {
            tmp.push(sign);
        }
        tmp.push_str(label);
        fmtstr(s, size, &tmp, width, tmp.chars().count(), flags);
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
        exponent = getexponent(value);
        estyle = true;
    }

    loop {
        if precision > 19 {
            precision = 19;
        }
        let ufvalue = value.abs();
        let mut work_value = ufvalue;
        if estyle {
            work_value /= mypow10(exponent);
        }

        let intpart_f = work_value.floor();
        if !intpart_f.is_finite() || intpart_f > (u64::MAX as f64) {
            return;
        }
        let mut intpart: u64 = intpart_f as u64;
        let mask = mypow10(precision);
        let mut fracpart: u64 = (mask * (work_value - intpart_f) + 0.5).floor() as u64;
        if (fracpart as f64) >= mask {
            intpart += 1;
            fracpart = 0;
            if estyle && intpart == 10 {
                intpart = 1;
                exponent += 1;
            }
        }

        if (flags & PRINT_F_TYPE_G) != 0
            && estyle
            && precision + 1 > exponent
            && exponent >= -4
        {
            precision -= exponent;
            estyle = false;
            continue;
        }

        // emit
        let mut econvert: Vec<char> = Vec::new();
        let mut esign = '\0';
        if estyle {
            let exp = if exponent < 0 {
                esign = '-';
                -exponent
            } else {
                esign = '+';
                exponent
            };
            // exponent string e.g. "e+12"
            let mut exp_str = format!("{}", exp);
            if exp_str.len() == 1 {
                exp_str.insert(0, '0');
            }
            econvert.push(if (flags & PRINT_F_UP) != 0 { 'E' } else { 'e' });
            econvert.push(esign);
            for c in exp_str.chars() {
                econvert.push(c);
            }
        }
        let _ = esign;

        let int_str = format!("{}", intpart);
        let iconvert: Vec<char> = int_str.chars().rev().collect();
        let ipos = iconvert.len();

        let mut fconvert: Vec<char> = Vec::new();
        if fracpart != 0 {
            let frac_str = format!("{}", fracpart);
            fconvert = frac_str.chars().rev().collect();
        }
        let fpos = fconvert.len();
        let mut leadfraczeros = precision - (fpos as i32);

        let mut omitcount = 0i32;
        if omitzeros {
            if fpos > 0 {
                while (omitcount as usize) < fpos
                    && fconvert[omitcount as usize] == '0'
                {
                    omitcount += 1;
                }
            } else {
                omitcount = precision;
                leadfraczeros = 0;
            }
            precision -= omitcount;
        }

        let emitpoint = precision > 0 || (flags & PRINT_F_NUM) != 0;
        let separators_flag = (flags & PRINT_F_QUOTE) != 0;
        let separators = if separators_flag { getnumsep(ipos as i32) } else { 0 };

        let mut padlen: i64 = (width as i64)
            - (ipos as i64)
            - (econvert.len() as i64)
            - (precision as i64)
            - (separators as i64)
            - if emitpoint { 1 } else { 0 }
            - if sign != '\0' { 1 } else { 0 };
        if padlen < 0 {
            padlen = 0;
        }

        if (flags & PRINT_F_MINUS) != 0 {
            padlen = -padlen;
        } else if (flags & PRINT_F_ZERO) != 0 && padlen > 0 {
            if sign != '\0' {
                outchar(s, &mut len, size, sign);
            }
            while padlen > 0 {
                outchar(s, &mut len, size, '0');
                padlen -= 1;
            }
            return emit_rest(
                s,
                size,
                &mut len,
                '\0',
                &iconvert,
                ipos,
                emitpoint,
                leadfraczeros,
                &fconvert,
                fpos,
                omitcount,
                &econvert,
                separators_flag,
                separators,
                padlen,
            );
        }
        return emit_rest(
            s,
            size,
            &mut len,
            sign,
            &iconvert,
            ipos,
            emitpoint,
            leadfraczeros,
            &fconvert,
            fpos,
            omitcount,
            &econvert,
            separators_flag,
            separators,
            padlen,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_rest(
    s: &mut String,
    size: usize,
    len: &mut usize,
    sign: char,
    iconvert: &[char],
    ipos: usize,
    emitpoint: bool,
    mut leadfraczeros: i32,
    fconvert: &[char],
    fpos: usize,
    omitcount: i32,
    econvert: &[char],
    _separators_flag: bool,
    separators: i32,
    mut padlen: i64,
) {
    while padlen > 0 {
        outchar(s, len, size, ' ');
        padlen -= 1;
    }
    if sign != '\0' {
        outchar(s, len, size, sign);
    }
    let mut ip = ipos;
    let mut sep_remaining = separators;
    while ip > 0 {
        ip -= 1;
        outchar(s, len, size, iconvert[ip]);
        if sep_remaining > 0 && ip > 0 && ip % 3 == 0 {
            printsep(s, size);
            *len += 1;
            sep_remaining -= 1;
        }
    }
    if emitpoint {
        outchar(s, len, size, '.');
    }
    while leadfraczeros > 0 {
        outchar(s, len, size, '0');
        leadfraczeros -= 1;
    }
    let mut fp = fpos;
    while fp as i32 > omitcount {
        fp -= 1;
        outchar(s, len, size, fconvert[fp]);
    }
    let mut ep = econvert.len();
    while ep > 0 {
        ep -= 1;
        outchar(s, len, size, econvert[ep]);
    }
    while padlen < 0 {
        outchar(s, len, size, ' ');
        padlen += 1;
    }
}

pub fn printsep(s: &mut String, size: usize) {
    if s.len() < size {
        s.push(',');
    }
}

pub fn getnumsep(digits: i32) -> i32 {
    let separators = (digits - if digits % 3 == 0 { 1 } else { 0 }) / 3;
    separators
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
    buf.clear();
    let digits_l = b"0123456789abcdef";
    let digits_u = b"0123456789ABCDEF";
    let digits: &[u8] = if caps != 0 { digits_u } else { digits_l };
    let mut v = value;
    if base < 2 || base > 16 {
        return;
    }
    loop {
        buf.push(digits[v % base] as char);
        v /= base;
        if v == 0 {
            break;
        }
    }
}

pub fn cast(value: f64) -> i32 {
    // Match C cast: returns floor for positive values (avoid implementations
    // that round 1.9 -> 2). For negative values just use as-cast.
    if !value.is_finite() {
        return i32::MAX;
    }
    if value >= (i32::MAX as f64) {
        return i32::MAX;
    }
    if value <= (i32::MIN as f64) {
        return i32::MIN;
    }
    let r = value as i32;
    // If the cast produced a value > original, subtract one (matches C logic)
    if (r as f64) <= value {
        r
    } else {
        r - 1
    }
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

pub fn rpl_vasprintf(
    mut s: Vec<String>,
    format: &str,
    args: &[&str],
) -> i32 {
    let mut buf = String::new();
    let len = rpl_vsnprintf(&mut buf, usize::MAX, format, args);
    s.push(buf);
    len
}

pub fn rpl_asprintf(
    s: &mut String,
    format: &str,
    args: &[&str],
) -> i32 {
    rpl_vsnprintf(s, usize::MAX, format, args)
}

pub fn main() {
    // Empty main, matches C TEST_SNPRINTF main but isn't invoked here
}
