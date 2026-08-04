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

pub fn rpl_vsnprintf(
    s: &mut String,
    n: usize,
    format: &str,
    args: &[&str],
) -> i32 {
    // Simplified: format the string using Rust's formatting, respecting size limit n.
    // Since the Rust interface uses String and &[&str], we do a basic printf-style parse.
    let mut result = String::new();
    let mut arg_idx = 0;
    let chars: Vec<char> = format.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '%' {
            i += 1;
            if i >= chars.len() {
                break;
            }
            // Handle %%
            if chars[i] == '%' {
                result.push('%');
                i += 1;
                continue;
            }
            // Parse flags, width, precision, length modifiers, then conversion
            let mut flags = 0i32;
            let mut width: i32 = 0;
            let mut precision: i32 = -1;

            // Flags
            loop {
                if i >= chars.len() { break; }
                match chars[i] {
                    '-' => { flags |= PRINT_F_MINUS; i += 1; }
                    '+' => { flags |= PRINT_F_PLUS; i += 1; }
                    ' ' => { flags |= PRINT_F_SPACE; i += 1; }
                    '#' => { flags |= PRINT_F_NUM; i += 1; }
                    '0' => { flags |= PRINT_F_ZERO; i += 1; }
                    '\'' => { flags |= PRINT_F_QUOTE; i += 1; }
                    _ => break,
                }
            }

            // Width
            while i < chars.len() && chars[i].is_ascii_digit() {
                width = width * 10 + (chars[i] as i32 - '0' as i32);
                i += 1;
            }

            // Precision
            if i < chars.len() && chars[i] == '.' {
                i += 1;
                precision = 0;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    precision = precision * 10 + (chars[i] as i32 - '0' as i32);
                    i += 1;
                }
            }

            // Length modifiers (skip them)
            while i < chars.len() && matches!(chars[i], 'h' | 'l' | 'L' | 'j' | 't' | 'z') {
                i += 1;
            }

            if i >= chars.len() { break; }

            let conv = chars[i];
            i += 1;

            match conv {
                'd' | 'i' => {
                    let val_str = if arg_idx < args.len() { args[arg_idx] } else { "0" };
                    arg_idx += 1;
                    let value: i64 = val_str.parse().unwrap_or(0);
                    let mut tmp = String::new();
                    fmtint_to(&mut tmp, value as i32, width as usize, if precision < 0 { usize::MAX } else { precision as usize }, flags);
                    result.push_str(&tmp);
                }
                'u' | 'o' | 'x' | 'X' => {
                    let mut f = flags | PRINT_F_UNSIGNED;
                    if conv == 'X' { f |= PRINT_F_UP; }
                    let val_str = if arg_idx < args.len() { args[arg_idx] } else { "0" };
                    arg_idx += 1;
                    let value: i64 = val_str.parse().unwrap_or(0);
                    let mut tmp = String::new();
                    fmtint_to(&mut tmp, value as i32, width as usize, if precision < 0 { usize::MAX } else { precision as usize }, f);
                    result.push_str(&tmp);
                }
                'f' | 'F' | 'e' | 'E' | 'g' | 'G' => {
                    let mut f = flags;
                    if conv == 'F' || conv == 'E' || conv == 'G' { f |= PRINT_F_UP; }
                    if conv == 'e' || conv == 'E' { f |= PRINT_F_TYPE_E; }
                    if conv == 'g' || conv == 'G' { f |= PRINT_F_TYPE_G; }
                    let val_str = if arg_idx < args.len() { args[arg_idx] } else { "0" };
                    arg_idx += 1;
                    let value: f64 = val_str.parse().unwrap_or(0.0);
                    let mut tmp = String::new();
                    fmtflt_to(&mut tmp, value, width as usize, if precision < 0 { usize::MAX } else { precision as usize }, f);
                    result.push_str(&tmp);
                }
                'c' => {
                    let val_str = if arg_idx < args.len() { args[arg_idx] } else { "" };
                    arg_idx += 1;
                    if let Some(c) = val_str.chars().next() {
                        result.push(c);
                    }
                }
                's' => {
                    let val_str = if arg_idx < args.len() { args[arg_idx] } else { "(null)" };
                    arg_idx += 1;
                    let mut tmp = String::new();
                    fmtstr_to(&mut tmp, val_str, width as usize, if precision < 0 { usize::MAX } else { precision as usize }, flags);
                    result.push_str(&tmp);
                }
                'p' => {
                    let val_str = if arg_idx < args.len() { args[arg_idx] } else { "" };
                    arg_idx += 1;
                    result.push_str(val_str);
                }
                _ => {}
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    if n > 0 {
        let truncated: String = result.chars().take(n.saturating_sub(1)).collect();
        let len = result.len() as i32;
        *s = truncated;
        len
    } else {
        let len = result.len() as i32;
        s.clear();
        len
    }
}

fn fmtstr_to(s: &mut String, value: &str, width: usize, precision: usize, flags: i32) {
    let noprecision = precision == usize::MAX;
    let strln = if noprecision {
        value.len()
    } else {
        value.len().min(precision)
    };
    let padlen = if width > strln { width - strln } else { 0 };
    let left = flags & PRINT_F_MINUS != 0;

    if !left {
        for _ in 0..padlen { s.push(' '); }
    }
    for (i, c) in value.chars().enumerate() {
        if !noprecision && i >= precision { break; }
        s.push(c);
    }
    if left {
        for _ in 0..padlen { s.push(' '); }
    }
}

fn fmtint_to(s: &mut String, value: i32, width: usize, precision: usize, flags: i32) {
    let noprecision = precision == usize::MAX;
    let unsigned = flags & PRINT_F_UNSIGNED != 0;
    let uvalue: u64 = if unsigned { value as u32 as u64 } else if value >= 0 { value as u64 } else { (-(value as i64)) as u64 };

    let mut sign = '\0';
    if !unsigned {
        if value < 0 { sign = '-'; }
        else if flags & PRINT_F_PLUS != 0 { sign = '+'; }
        else if flags & PRINT_F_SPACE != 0 { sign = ' '; }
    }

    let mut buf = String::new();
    convert_to(&mut buf, uvalue as usize, 10, if flags & PRINT_F_UP != 0 { 1 } else { 0 });
    let pos = buf.len();

    let separators = if flags & PRINT_F_QUOTE != 0 { getnumsep(pos as i32) } else { 0 };
    let prec = if noprecision { pos } else { precision };
    let zpadlen = if prec > pos + separators as usize { prec - pos - separators as usize } else { 0 };
    let total = separators as usize + prec.max(pos) + if sign != '\0' { 1 } else { 0 };
    let spadlen = if width > total { width - total } else { 0 };

    let left = flags & PRINT_F_MINUS != 0;
    let (mut spad, mut zpad) = if left {
        (0usize, zpadlen)
    } else if flags & PRINT_F_ZERO != 0 && noprecision {
        (0, zpadlen + spadlen)
    } else {
        (spadlen, zpadlen)
    };

    if !left {
        for _ in 0..spad { s.push(' '); }
    }
    if sign != '\0' { s.push(sign); }
    for _ in 0..zpad { s.push('0'); }
    s.push_str(&buf);
    if left {
        for _ in 0..spadlen { s.push(' '); }
    }
}

fn fmtflt_to(s: &mut String, fvalue: f64, width: usize, precision: usize, flags: i32) {
    let prec = if precision == usize::MAX { 6 } else { precision };

    let mut sign = '\0';
    if fvalue < 0.0 { sign = '-'; }
    else if flags & PRINT_F_PLUS != 0 { sign = '+'; }
    else if flags & PRINT_F_SPACE != 0 { sign = ' '; }

    if fvalue.is_nan() {
        let inf = if flags & PRINT_F_UP != 0 { "NAN" } else { "nan" };
        let mut tmp = String::new();
        if sign != '\0' { tmp.push(sign); }
        tmp.push_str(inf);
        let tlen = tmp.len();
        fmtstr_to(s, &tmp, width, tlen, flags);
        return;
    }
    if fvalue.is_infinite() {
        let inf = if flags & PRINT_F_UP != 0 { "INF" } else { "inf" };
        let mut tmp = String::new();
        if sign != '\0' { tmp.push(sign); }
        tmp.push_str(inf);
        let tlen = tmp.len();
        fmtstr_to(s, &tmp, width, tlen, flags);
        return;
    }

    let ufvalue = fvalue.abs();
    let mut prec = prec.min(19);

    let mut estyle = flags & PRINT_F_TYPE_E != 0;
    let is_g = flags & PRINT_F_TYPE_G != 0;
    let mut exponent = 0i32;
    let mut omitzeros = false;

    if is_g || estyle {
        let mut p = prec;
        if is_g {
            if p == 0 { p = 1; }
            p -= 1;
            if flags & PRINT_F_NUM == 0 { omitzeros = true; }
        }
        exponent = getexponent(ufvalue);
        estyle = true;
        prec = p;
    }

    // Possibly re-decide for %g
    let (intpart, fracpart, leadfraczeros, exponent, estyle, prec) = {
        let mut prec = prec.min(19);
        let mut estyle = estyle;
        let mut exponent = exponent;

        loop {
            let uf = if estyle { ufvalue / mypow10(exponent) } else { ufvalue };
            let ip = uf as u64;
            let mask = 10u64.pow(prec as u32);
            let mut fp = ((mask as f64) * (uf - ip as f64) + 0.5) as u64;
            let mut ip = ip;
            if fp >= mask {
                ip += 1;
                fp = 0;
                if estyle && ip == 10 {
                    ip = 1;
                    exponent += 1;
                }
            }

            if is_g && estyle && (prec as i32 + 1) > exponent && exponent >= -4 {
                prec = (prec as i32 - exponent) as usize;
                estyle = false;
                continue;
            }

            let fstr = if fp != 0 { format!("{}", fp) } else { String::new() };
            let lfz = if fp != 0 { prec.saturating_sub(fstr.len()) } else { 0 };
            break (ip, fp, lfz, exponent, estyle, prec);
        }
    };

    let mut omitcount = 0;
    let fstr = if fracpart != 0 { format!("{}", fracpart) } else { String::new() };
    let mut actual_prec = prec;
    if omitzeros {
        if !fstr.is_empty() {
            let trailing = fstr.chars().rev().take_while(|&c| c == '0').count();
            omitcount = trailing;
        } else {
            omitcount = prec;
        }
        actual_prec = prec - omitcount;
    }

    let emitpoint = actual_prec > 0 || flags & PRINT_F_NUM != 0;
    let istr = format!("{}", intpart);

    let mut estr = String::new();
    if estyle {
        let esign = if exponent < 0 { '-' } else { '+' };
        let eabs = exponent.unsigned_abs();
        let e_letter = if flags & PRINT_F_UP != 0 { 'E' } else { 'e' };
        if eabs < 10 {
            estr = format!("{}{}0{}", e_letter, esign, eabs);
        } else {
            estr = format!("{}{}{}", e_letter, esign, eabs);
        }
    }

    let content_len = istr.len() + estr.len() + actual_prec + if emitpoint { 1 } else { 0 } + if sign != '\0' { 1 } else { 0 };
    let padlen = if width > content_len { width - content_len } else { 0 };
    let left = flags & PRINT_F_MINUS != 0;

    if !left && flags & PRINT_F_ZERO != 0 && padlen > 0 {
        if sign != '\0' { s.push(sign); sign = '\0'; }
        for _ in 0..padlen { s.push('0'); }
    } else if !left {
        for _ in 0..padlen { s.push(' '); }
    }
    if sign != '\0' { s.push(sign); }
    s.push_str(&istr);
    if emitpoint { s.push('.'); }
    for _ in 0..leadfraczeros { s.push('0'); }
    if !fstr.is_empty() && actual_prec > leadfraczeros {
        let remaining = actual_prec - leadfraczeros;
        let take = fstr.len().min(remaining);
        s.push_str(&fstr[..fstr.len() - omitcount]);
    }
    s.push_str(&estr);
    if left {
        for _ in 0..padlen { s.push(' '); }
    }
}

pub fn fmtstr(
    s: &mut String,
    size: usize,
    value: &str,
    width: usize,
    precision: usize,
    flags: i32,
) {
    fmtstr_to(s, value, width, precision, flags);
}

pub fn fmtint(
    s: &mut String,
    size: usize,
    value: i32,
    width: usize,
    precision: usize,
    flags: i32,
) {
    fmtint_to(s, value, width, precision, flags);
}

pub fn fmtflt(
    s: &mut String,
    size: usize,
    value: f64,
    width: usize,
    precision: usize,
    flags: i32,
) {
    fmtflt_to(s, value, width, precision, flags);
}

pub fn printsep(s: &mut String, size: usize) {
    s.push(',');
}

pub fn getnumsep(digits: i32) -> i32 {
    (digits - if digits % 3 == 0 { 1 } else { 0 }) / 3
}

pub fn getexponent(value: f64) -> i32 {
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
    convert_to(buf, value, base, caps);
}

fn convert_to(buf: &mut String, value: usize, base: usize, caps: usize) {
    let digits: &[u8] = if caps != 0 { b"0123456789ABCDEF" } else { b"0123456789abcdef" };
    let mut v = value;
    let mut tmp = Vec::new();
    loop {
        tmp.push(digits[v % base] as char);
        v /= base;
        if v == 0 { break; }
    }
    for c in tmp.into_iter().rev() {
        buf.push(c);
    }
}

pub fn cast(value: f64) -> i32 {
    if value >= i32::MAX as f64 { return i32::MAX; }
    value as i32
}

pub fn mypow10(exponent: i32) -> f64 {
    let mut result = 1.0f64;
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
    let mut result = String::new();
    let len = rpl_vsnprintf(&mut result, usize::MAX, format, args);
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
    // The C main() is a test harness comparing sprintf vs snprintf.
    // In Rust, this is not needed as we test through the test binary.
}
