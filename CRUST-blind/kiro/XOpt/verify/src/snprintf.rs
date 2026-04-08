// Format flags
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

const MAX_CONVERT_LENGTH: usize = 43;

fn outchar(s: &mut String, len: &mut usize, size: usize, ch: char) {
    if *len + 1 < size {
        s.push(ch);
    }
    *len += 1;
}

pub fn rpl_vsnprintf(
    s: &mut String,
    n: usize,
    format: &str,
    args: &[&str],
) -> i32 {
    s.clear();
    let mut len: usize = 0;
    let mut arg_idx: usize = 0;
    let fmt_bytes = format.as_bytes();
    let mut fi = 0;

    while fi < fmt_bytes.len() {
        let ch = fmt_bytes[fi] as char;
        fi += 1;

        if ch != '%' {
            outchar(s, &mut len, n, ch);
            continue;
        }

        if fi >= fmt_bytes.len() {
            break;
        }

        // Parse flags
        let mut flags: i32 = 0;
        loop {
            if fi >= fmt_bytes.len() { break; }
            match fmt_bytes[fi] as char {
                '-' => { flags |= PRINT_F_MINUS; fi += 1; }
                '+' => { flags |= PRINT_F_PLUS; fi += 1; }
                ' ' => { flags |= PRINT_F_SPACE; fi += 1; }
                '#' => { flags |= PRINT_F_NUM; fi += 1; }
                '0' => { flags |= PRINT_F_ZERO; fi += 1; }
                '\'' => { flags |= PRINT_F_QUOTE; fi += 1; }
                _ => break,
            }
        }

        // Parse width
        let mut width: i32 = 0;
        if fi < fmt_bytes.len() && fmt_bytes[fi] == b'*' {
            if arg_idx < args.len() {
                width = args[arg_idx].parse::<i32>().unwrap_or(0);
                arg_idx += 1;
                if width < 0 {
                    flags |= PRINT_F_MINUS;
                    width = -width;
                }
            }
            fi += 1;
        } else {
            while fi < fmt_bytes.len() && (fmt_bytes[fi] as char).is_ascii_digit() {
                let d = (fmt_bytes[fi] - b'0') as i32;
                width = 10 * width + d;
                fi += 1;
            }
        }

        // Parse precision
        let mut precision: i32 = -1;
        if fi < fmt_bytes.len() && fmt_bytes[fi] == b'.' {
            fi += 1;
            precision = 0;
            if fi < fmt_bytes.len() && fmt_bytes[fi] == b'*' {
                if arg_idx < args.len() {
                    precision = args[arg_idx].parse::<i32>().unwrap_or(-1);
                    arg_idx += 1;
                    if precision < 0 { precision = -1; }
                }
                fi += 1;
            } else {
                while fi < fmt_bytes.len() && (fmt_bytes[fi] as char).is_ascii_digit() {
                    let d = (fmt_bytes[fi] - b'0') as i32;
                    precision = 10 * precision + d;
                    fi += 1;
                }
            }
        }

        // Parse length modifiers (skip them - our args are all strings)
        while fi < fmt_bytes.len() {
            match fmt_bytes[fi] as char {
                'h' | 'l' | 'L' | 'j' | 't' | 'z' => { fi += 1; }
                _ => break,
            }
        }

        if fi >= fmt_bytes.len() { break; }

        let conv = fmt_bytes[fi] as char;
        fi += 1;

        match conv {
            'd' | 'i' => {
                let val_str = if arg_idx < args.len() { args[arg_idx] } else { "0" };
                arg_idx += 1;
                let value: i64 = val_str.parse().unwrap_or(0);
                fmtint(s, n, value as i32, width as usize, precision as usize, flags);
            }
            'u' => {
                let val_str = if arg_idx < args.len() { args[arg_idx] } else { "0" };
                arg_idx += 1;
                let value: u64 = val_str.parse().unwrap_or(0);
                fmtint(s, n, value as i32, width as usize, precision as usize, flags | PRINT_F_UNSIGNED);
            }
            'x' | 'X' => {
                let mut f = flags | PRINT_F_UNSIGNED;
                if conv == 'X' { f |= PRINT_F_UP; }
                let val_str = if arg_idx < args.len() { args[arg_idx] } else { "0" };
                arg_idx += 1;
                let value: u64 = val_str.parse().unwrap_or(0);
                fmtint_base(s, n, value as i64, 16, width as usize, precision as usize, f);
            }
            'o' => {
                let val_str = if arg_idx < args.len() { args[arg_idx] } else { "0" };
                arg_idx += 1;
                let value: u64 = val_str.parse().unwrap_or(0);
                fmtint_base(s, n, value as i64, 8, width as usize, precision as usize, flags | PRINT_F_UNSIGNED);
            }
            'f' | 'F' => {
                let mut f = flags;
                if conv == 'F' { f |= PRINT_F_UP; }
                let val_str = if arg_idx < args.len() { args[arg_idx] } else { "0" };
                arg_idx += 1;
                let value: f64 = val_str.parse().unwrap_or(0.0);
                fmtflt(s, n, value, width as usize, precision as usize, f);
            }
            'e' | 'E' => {
                let mut f = flags | PRINT_F_TYPE_E;
                if conv == 'E' { f |= PRINT_F_UP; }
                let val_str = if arg_idx < args.len() { args[arg_idx] } else { "0" };
                arg_idx += 1;
                let value: f64 = val_str.parse().unwrap_or(0.0);
                fmtflt(s, n, value, width as usize, precision as usize, f);
            }
            'g' | 'G' => {
                let mut f = flags | PRINT_F_TYPE_G;
                if conv == 'G' { f |= PRINT_F_UP; }
                let val_str = if arg_idx < args.len() { args[arg_idx] } else { "0" };
                arg_idx += 1;
                let value: f64 = val_str.parse().unwrap_or(0.0);
                let p = if precision == 0 { 1 } else { precision };
                fmtflt(s, n, value, width as usize, p as usize, f);
            }
            'c' => {
                let val_str = if arg_idx < args.len() { args[arg_idx] } else { "" };
                arg_idx += 1;
                let c = val_str.chars().next().unwrap_or('\0');
                outchar(s, &mut len, n, c);
            }
            's' => {
                let val_str = if arg_idx < args.len() { args[arg_idx] } else { "" };
                arg_idx += 1;
                fmtstr(s, n, val_str, width as usize, precision as usize, flags);
            }
            '%' => {
                outchar(s, &mut len, n, '%');
            }
            _ => {}
        }
        len = s.len();
    }

    // Truncate to n-1 if needed
    if n > 0 && s.len() >= n {
        let truncated: String = s.chars().take(n - 1).collect();
        *s = truncated;
    }

    len as i32
}

pub fn fmtstr(
    s: &mut String,
    size: usize,
    value: &str,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let mut len = s.len();
    let noprecision = precision == usize::MAX;

    let val = value;

    let mut strln: i32 = 0;
    for ch in val.chars() {
        if ch == '\0' { break; }
        if !noprecision && strln >= precision as i32 { break; }
        strln += 1;
    }

    let mut padlen = width as i32 - strln;
    if padlen < 0 { padlen = 0; }
    if flags & PRINT_F_MINUS != 0 { padlen = -padlen; }

    while padlen > 0 {
        outchar(s, &mut len, size, ' ');
        padlen -= 1;
    }
    let mut count = 0;
    for ch in val.chars() {
        if ch == '\0' { break; }
        if !noprecision && count >= precision as i32 { break; }
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
    fmtint_base(s, size, value as i64, 10, width, precision, flags);
}

fn fmtint_base(
    s: &mut String,
    size: usize,
    value: i64,
    base: usize,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let mut len = s.len();
    let noprecision = precision == usize::MAX;
    let precision = if noprecision { 0usize } else { precision };

    let uvalue: u64;
    let mut sign: char = '\0';
    let mut hexprefix: char = '\0';

    if flags & PRINT_F_UNSIGNED != 0 {
        uvalue = value as u64;
    } else {
        if value < 0 {
            uvalue = (-value) as u64;
            sign = '-';
        } else {
            uvalue = value as u64;
            if flags & PRINT_F_PLUS != 0 {
                sign = '+';
            } else if flags & PRINT_F_SPACE != 0 {
                sign = ' ';
            }
        }
    }

    let mut iconvert = [0u8; MAX_CONVERT_LENGTH];
    let pos = convert_internal(uvalue, &mut iconvert, base, (flags & PRINT_F_UP) != 0);

    let mut prec = precision as i32;
    if flags & PRINT_F_NUM != 0 && uvalue != 0 {
        match base {
            8 => { if prec <= pos as i32 { prec = pos as i32 + 1; } }
            16 => { hexprefix = if flags & PRINT_F_UP != 0 { 'X' } else { 'x' }; }
            _ => {}
        }
    }

    let separators = if flags & PRINT_F_QUOTE != 0 { getnumsep(pos as i32) } else { 0 };

    let mut zpadlen = prec - pos as i32 - separators;
    let mut spadlen = width as i32
        - separators
        - std::cmp::max(prec, pos as i32)
        - if sign != '\0' { 1 } else { 0 }
        - if hexprefix != '\0' { 2 } else { 0 };

    if zpadlen < 0 { zpadlen = 0; }
    if spadlen < 0 { spadlen = 0; }

    if flags & PRINT_F_MINUS != 0 {
        spadlen = -spadlen;
    } else if flags & PRINT_F_ZERO != 0 && noprecision {
        zpadlen += spadlen;
        spadlen = 0;
    }

    while spadlen > 0 { outchar(s, &mut len, size, ' '); spadlen -= 1; }
    if sign != '\0' { outchar(s, &mut len, size, sign); }
    if hexprefix != '\0' {
        outchar(s, &mut len, size, '0');
        outchar(s, &mut len, size, hexprefix);
    }
    while zpadlen > 0 { outchar(s, &mut len, size, '0'); zpadlen -= 1; }

    let mut p = pos;
    while p > 0 {
        p -= 1;
        outchar(s, &mut len, size, iconvert[p] as char);
        if separators > 0 && p > 0 && p % 3 == 0 {
            printsep(s, size);
        }
    }
    while spadlen < 0 { outchar(s, &mut len, size, ' '); spadlen += 1; }
}

pub fn fmtflt(
    s: &mut String,
    size: usize,
    value: f64,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let mut len = s.len();
    let mut precision = if precision == usize::MAX { 6i32 } else { precision as i32 };
    let mut flags = flags;

    let mut sign: char = '\0';
    if value < 0.0 {
        sign = '-';
    } else if flags & PRINT_F_PLUS != 0 {
        sign = '+';
    } else if flags & PRINT_F_SPACE != 0 {
        sign = ' ';
    }

    if value.is_nan() {
        let infnan = if flags & PRINT_F_UP != 0 { "NAN" } else { "nan" };
        let mut tmp = String::new();
        if sign != '\0' { tmp.push(sign); }
        tmp.push_str(infnan);
        let ipos = tmp.len() as i32;
        fmtstr(s, size, &tmp, width, ipos as usize, flags);
        return;
    }
    if value.is_infinite() {
        let infnan = if flags & PRINT_F_UP != 0 { "INF" } else { "inf" };
        let mut tmp = String::new();
        if sign != '\0' { tmp.push(sign); }
        tmp.push_str(infnan);
        let ipos = tmp.len() as i32;
        fmtstr(s, size, &tmp, width, ipos as usize, flags);
        return;
    }

    let mut estyle = (flags & PRINT_F_TYPE_E) != 0;
    let mut omitzeros = false;
    let mut exponent: i32 = 0;

    if (flags & PRINT_F_TYPE_E != 0) || (flags & PRINT_F_TYPE_G != 0) {
        if flags & PRINT_F_TYPE_G != 0 {
            precision -= 1;
            if flags & PRINT_F_NUM == 0 {
                omitzeros = true;
            }
        }
        exponent = getexponent(value);
        estyle = true;
    }

    loop {
        // Clamp precision
        if precision > 19 { precision = 19; }

        let ufvalue_base = if value >= 0.0 { value } else { -value };
        let mut ufvalue = if estyle {
            ufvalue_base / mypow10(exponent)
        } else {
            ufvalue_base
        };

        let mut intpart = cast(ufvalue) as u64;
        let mask = mypow10(precision) as u64;
        let frac_raw = ((mask as f64) * (ufvalue - intpart as f64) + 0.5) as u64;
        let mut fracpart = frac_raw;

        if fracpart >= mask {
            intpart += 1;
            fracpart = 0;
            if estyle && intpart == 10 {
                intpart = 1;
                exponent += 1;
            }
        }

        if flags & PRINT_F_TYPE_G != 0 && estyle && precision + 1 > exponent && exponent >= -4 {
            precision -= exponent;
            estyle = false;
            continue;
        }

        let mut esign: char = '\0';
        let mut econvert = [0u8; 4];
        let mut epos: usize = 0;

        if estyle {
            if exponent < 0 {
                esign = '-';
                exponent = -exponent;
            } else {
                esign = '+';
            }
            epos = convert_internal(exponent as u64, &mut econvert, 10, false);
            if epos == 1 {
                econvert[epos] = b'0';
                epos += 1;
            }
            econvert[epos] = esign as u8;
            epos += 1;
            econvert[epos] = if flags & PRINT_F_UP != 0 { b'E' } else { b'e' };
            epos += 1;
        }

        let mut iconvert_buf = [0u8; MAX_CONVERT_LENGTH];
        let ipos = convert_internal(intpart, &mut iconvert_buf, 10, false);

        let mut fconvert_buf = [0u8; MAX_CONVERT_LENGTH];
        let fpos = if fracpart != 0 {
            convert_internal(fracpart, &mut fconvert_buf, 10, false)
        } else {
            0
        };

        let mut leadfraczeros = precision - fpos as i32;
        let mut omitcount: i32 = 0;

        if omitzeros {
            if fpos > 0 {
                while omitcount < fpos as i32 && fconvert_buf[omitcount as usize] == b'0' {
                    omitcount += 1;
                }
            } else {
                omitcount = precision;
                leadfraczeros = 0;
            }
            precision -= omitcount;
        }

        let emitpoint = precision > 0 || (flags & PRINT_F_NUM != 0);
        let separators = if flags & PRINT_F_QUOTE != 0 { getnumsep(ipos as i32) } else { 0 };

        let mut padlen = width as i32
            - ipos as i32
            - epos as i32
            - precision
            - separators
            - if emitpoint { 1 } else { 0 }
            - if sign != '\0' { 1 } else { 0 };

        if padlen < 0 { padlen = 0; }

        if flags & PRINT_F_MINUS != 0 {
            padlen = -padlen;
        } else if flags & PRINT_F_ZERO != 0 && padlen > 0 {
            if sign != '\0' {
                outchar(s, &mut len, size, sign);
                sign = '\0';
            }
            while padlen > 0 {
                outchar(s, &mut len, size, '0');
                padlen -= 1;
            }
        }

        while padlen > 0 { outchar(s, &mut len, size, ' '); padlen -= 1; }
        if sign != '\0' { outchar(s, &mut len, size, sign); }

        let mut ip = ipos;
        while ip > 0 {
            ip -= 1;
            outchar(s, &mut len, size, iconvert_buf[ip] as char);
            if separators > 0 && ip > 0 && ip % 3 == 0 {
                printsep(s, size);
            }
        }

        if emitpoint { outchar(s, &mut len, size, '.'); }

        while leadfraczeros > 0 {
            outchar(s, &mut len, size, '0');
            leadfraczeros -= 1;
        }

        let mut fp = fpos;
        while fp as i32 > omitcount {
            fp -= 1;
            outchar(s, &mut len, size, fconvert_buf[fp] as char);
        }

        let mut ep = epos;
        while ep > 0 {
            ep -= 1;
            outchar(s, &mut len, size, econvert[ep] as char);
        }

        while padlen < 0 { outchar(s, &mut len, size, ' '); padlen += 1; }

        break;
    }
}

pub fn printsep(s: &mut String, size: usize) {
    let mut len = s.len();
    outchar(s, &mut len, size, ',');
}

pub fn getnumsep(digits: i32) -> i32 {
    (digits - if digits % 3 == 0 { 1 } else { 0 }) / 3
}

pub fn getexponent(value: f64) -> i32 {
    let mut tmp = if value >= 0.0 { value } else { -value };
    let mut exponent: i32 = 0;

    while tmp < 1.0 && tmp > 0.0 && { exponent -= 1; exponent } > -99 {
        tmp *= 10.0;
    }
    while tmp >= 10.0 && { exponent += 1; exponent } < 99 {
        tmp /= 10.0;
    }

    exponent
}

pub fn convert(
    value: usize,
    buf: &mut String,
    base: usize,
    caps: usize,
) {
    buf.clear();
    let digits: &[u8] = if caps != 0 {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    let mut v = value;
    loop {
        buf.push(digits[v % base] as char);
        v /= base;
        if v == 0 { break; }
    }
}

fn convert_internal(value: u64, buf: &mut [u8], base: usize, caps: bool) -> usize {
    let digits: &[u8] = if caps {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    let mut pos = 0;
    let mut v = value;
    loop {
        buf[pos] = digits[(v % base as u64) as usize];
        pos += 1;
        v /= base as u64;
        if v == 0 || pos >= buf.len() { break; }
    }
    pos
}

pub fn cast(value: f64) -> i32 {
    if value >= u64::MAX as f64 {
        return u64::MAX as i32;
    }
    let result = value as u64;
    if result as f64 <= value { result as i32 } else { (result - 1) as i32 }
}

pub fn mypow10(exponent: i32) -> f64 {
    let mut result: f64 = 1.0;
    let mut exp = exponent;
    while exp > 0 { result *= 10.0; exp -= 1; }
    while exp < 0 { result /= 10.0; exp += 1; }
    result
}

pub fn rpl_vasprintf(
    s: Vec<String>,
    format: &str,
    args: &[&str],
) -> i32 {
    // First pass: determine length
    let mut dummy = String::new();
    let len = rpl_vsnprintf(&mut dummy, 0, format, args);
    if len < 0 { return -1; }
    // s is passed by value and can't be mutated meaningfully, just return len
    len
}

pub fn rpl_asprintf(
    s: &mut String,
    format: &str,
    args: &[&str],
) -> i32 {
    // Use a large size so content is actually written and length is correct
    s.clear();
    rpl_vsnprintf(s, usize::MAX, format, args)
}

pub fn main() {
    // Dummy main - the C code only has main() when TEST_SNPRINTF is defined
}
