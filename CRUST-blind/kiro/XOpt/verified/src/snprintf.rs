// Format flag constants matching the C code
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

fn outchar(s: &mut String, size: usize, ch: char) {
    if s.len() + 1 < size {
        s.push(ch);
    }
}

pub fn rpl_vsnprintf(
    s: &mut String,
    n: usize,
    format: &str,
    args: &[&str],
) -> i32 {
    s.clear();
    let mut arg_idx = 0;
    let chars: Vec<char> = format.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '%' {
            i += 1;
            if i >= chars.len() { break; }

            // Parse flags
            let mut flags = 0i32;
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

            // Parse width
            let mut width: usize = 0;
            if i < chars.len() && chars[i] == '*' {
                if arg_idx < args.len() {
                    let w: i32 = args[arg_idx].parse().unwrap_or(0);
                    arg_idx += 1;
                    if w < 0 {
                        flags |= PRINT_F_MINUS;
                        width = (-w) as usize;
                    } else {
                        width = w as usize;
                    }
                }
                i += 1;
            } else {
                while i < chars.len() && chars[i].is_ascii_digit() {
                    width = width * 10 + (chars[i] as usize - '0' as usize);
                    i += 1;
                }
            }

            // Parse precision
            let mut precision: i32 = -1;
            if i < chars.len() && chars[i] == '.' {
                i += 1;
                precision = 0;
                if i < chars.len() && chars[i] == '*' {
                    if arg_idx < args.len() {
                        let p: i32 = args[arg_idx].parse().unwrap_or(0);
                        arg_idx += 1;
                        precision = if p < 0 { -1 } else { p };
                    }
                    i += 1;
                } else {
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        precision = precision * 10 + (chars[i] as i32 - '0' as i32);
                        i += 1;
                    }
                }
            }

            // Parse length modifier (skip it, we use string args)
            while i < chars.len() && matches!(chars[i], 'h' | 'l' | 'L' | 'j' | 't' | 'z') {
                i += 1;
            }

            if i >= chars.len() { break; }

            // Conversion
            match chars[i] {
                '%' => { outchar(s, n, '%'); }
                's' => {
                    let val = if arg_idx < args.len() { args[arg_idx] } else { "(null)" };
                    arg_idx += 1;
                    fmtstr(s, n, val, width, precision as usize, flags);
                }
                'd' | 'i' => {
                    let val: i32 = if arg_idx < args.len() { args[arg_idx].parse().unwrap_or(0) } else { 0 };
                    arg_idx += 1;
                    fmtint(s, n, val, width, precision as usize, flags);
                }
                'c' => {
                    let val = if arg_idx < args.len() && !args[arg_idx].is_empty() {
                        args[arg_idx].chars().next().unwrap_or('\0')
                    } else { '\0' };
                    arg_idx += 1;
                    outchar(s, n, val);
                }
                'f' | 'F' => {
                    if chars[i] == 'F' { flags |= PRINT_F_UP; }
                    let val: f64 = if arg_idx < args.len() { args[arg_idx].parse().unwrap_or(0.0) } else { 0.0 };
                    arg_idx += 1;
                    fmtflt(s, n, val, width, precision as usize, flags);
                }
                'e' | 'E' => {
                    if chars[i] == 'E' { flags |= PRINT_F_UP; }
                    flags |= PRINT_F_TYPE_E;
                    let val: f64 = if arg_idx < args.len() { args[arg_idx].parse().unwrap_or(0.0) } else { 0.0 };
                    arg_idx += 1;
                    fmtflt(s, n, val, width, precision as usize, flags);
                }
                'g' | 'G' => {
                    if chars[i] == 'G' { flags |= PRINT_F_UP; }
                    flags |= PRINT_F_TYPE_G;
                    let val: f64 = if arg_idx < args.len() { args[arg_idx].parse().unwrap_or(0.0) } else { 0.0 };
                    arg_idx += 1;
                    if precision == 0 { precision = 1; }
                    fmtflt(s, n, val, width, precision as usize, flags);
                }
                'u' | 'o' | 'x' | 'X' => {
                    if chars[i] == 'X' { flags |= PRINT_F_UP; }
                    flags |= PRINT_F_UNSIGNED;
                    let val: i32 = if arg_idx < args.len() { args[arg_idx].parse().unwrap_or(0) } else { 0 };
                    arg_idx += 1;
                    fmtint(s, n, val, width, precision as usize, flags);
                }
                _ => {}
            }
            i += 1;
        } else {
            outchar(s, n, chars[i]);
            i += 1;
        }
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
    let _noprecision = precision == 0 && (flags & !0x3FF == 0);
    // In the C code, precision==-1 means no precision. Our Rust signature uses usize.
    // We treat precision==usize::MAX or the caller passing -1 cast as usize as "no precision".
    // The caller (rpl_vsnprintf) passes precision as usize from i32, so -1 becomes a huge number.
    let prec = precision;
    let noprecision = prec > 1_000_000; // effectively -1 cast to usize

    let val = if value.is_empty() && noprecision { "(null)" } else { value };

    let strln = if noprecision {
        val.len()
    } else {
        val.len().min(prec)
    };

    let padlen: i32 = if width > strln { (width - strln) as i32 } else { 0 };
    let mut padlen = if flags & PRINT_F_MINUS != 0 { -padlen } else { padlen };

    while padlen > 0 {
        outchar(s, size, ' ');
        padlen -= 1;
    }
    for ch in val.chars().take(strln) {
        outchar(s, size, ch);
    }
    while padlen < 0 {
        outchar(s, size, ' ');
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
    let noprecision = precision > 1_000_000;
    let precision = if noprecision { 0 } else { precision };

    let uvalue: u64 = if flags & PRINT_F_UNSIGNED != 0 {
        value as u32 as u64
    } else if value >= 0 {
        value as u64
    } else {
        (-(value as i64)) as u64
    };

    let mut sign: char = '\0';
    if flags & PRINT_F_UNSIGNED == 0 {
        if value < 0 {
            sign = '-';
        } else if flags & PRINT_F_PLUS != 0 {
            sign = '+';
        } else if flags & PRINT_F_SPACE != 0 {
            sign = ' ';
        }
    }

    let mut iconvert = String::new();
    convert(uvalue as usize, &mut iconvert, 10, 0);
    let pos = iconvert.len();

    let separators = if flags & PRINT_F_QUOTE != 0 { getnumsep(pos as i32) } else { 0 };

    let zpadlen: i32 = precision as i32 - pos as i32 - separators;
    let mut zpadlen = if zpadlen < 0 { 0 } else { zpadlen };

    let spadlen: i32 = width as i32
        - separators
        - std::cmp::max(precision as i32, pos as i32)
        - if sign != '\0' { 1 } else { 0 };
    let mut spadlen = if spadlen < 0 { 0 } else { spadlen };

    if flags & PRINT_F_MINUS != 0 {
        spadlen = -spadlen;
    } else if flags & PRINT_F_ZERO != 0 && noprecision {
        zpadlen += spadlen;
        spadlen = 0;
    }

    while spadlen > 0 { outchar(s, size, ' '); spadlen -= 1; }
    if sign != '\0' { outchar(s, size, sign); }
    while zpadlen > 0 { outchar(s, size, '0'); zpadlen -= 1; }

    let digits: Vec<char> = iconvert.chars().collect();
    let mut p = digits.len();
    while p > 0 {
        p -= 1;
        outchar(s, size, digits[p]);
        if separators > 0 && p > 0 && p % 3 == 0 {
            printsep(s, size);
        }
    }

    while spadlen < 0 { outchar(s, size, ' '); spadlen += 1; }
}

pub fn fmtflt(
    s: &mut String,
    size: usize,
    value: f64,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let mut precision = if precision > 1_000_000 { 6i32 } else { precision as i32 };
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
        fmtstr(s, size, &tmp, width, tmp.len(), flags);
        return;
    }
    if value.is_infinite() {
        let infnan = if flags & PRINT_F_UP != 0 { "INF" } else { "inf" };
        let mut tmp = String::new();
        if sign != '\0' { tmp.push(sign); }
        tmp.push_str(infnan);
        fmtstr(s, size, &tmp, width, tmp.len(), flags);
        return;
    }

    let mut estyle = flags & PRINT_F_TYPE_E != 0;
    let mut omitzeros = false;
    let mut exponent = 0i32;
    let separators_flag = flags & PRINT_F_QUOTE != 0;

    if flags & PRINT_F_TYPE_E != 0 || flags & PRINT_F_TYPE_G != 0 {
        if flags & PRINT_F_TYPE_G != 0 {
            precision -= 1;
            if flags & PRINT_F_NUM == 0 { omitzeros = true; }
        }
        exponent = getexponent(value);
        estyle = true;
    }

    // Limit precision
    if precision > 19 { precision = 19; }

    loop {
        let mut ufvalue = value.abs();
        if estyle { ufvalue /= mypow10(exponent); }

        let mut intpart = cast(ufvalue) as u64;
        let mask = mypow10(precision) as u64;
        let frac_raw = (mask as f64) * (ufvalue - intpart as f64);
        let mut fracpart = myround(frac_raw);

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

        // Build exponent string
        let mut econvert = String::new();
        let mut epos = 0;
        if estyle {
            let esign = if exponent < 0 { '-' } else { '+' };
            let eabs = exponent.unsigned_abs() as usize;
            let mut ebuf = String::new();
            convert(eabs, &mut ebuf, 10, 0);
            let edigits: Vec<char> = ebuf.chars().collect();
            // Build econvert in reverse order (will be printed reversed)
            // econvert stores: digits(reversed), esign, 'e'/'E'
            let e_ch = if flags & PRINT_F_UP != 0 { 'E' } else { 'e' };
            // We need to output: e+01 (or E-12 etc.)
            // Store reversed: "10+e" so when printed reversed it's "e+01"
            econvert.clear();
            // pad to at least 2 digits
            if edigits.len() == 1 {
                econvert.push(edigits[0]);
                econvert.push('0');
            } else {
                for &d in &edigits {
                    econvert.push(d);
                }
            }
            econvert.push(esign);
            econvert.push(e_ch);
            epos = econvert.len();
        }

        // Convert integer and fractional parts
        let mut iconvert_s = String::new();
        convert(intpart as usize, &mut iconvert_s, 10, 0);
        let ipos = iconvert_s.len();

        let mut fconvert_s = String::new();
        let mut fpos = 0;
        if fracpart != 0 {
            convert(fracpart as usize, &mut fconvert_s, 10, 0);
            fpos = fconvert_s.len();
        }

        let mut leadfraczeros = precision - fpos as i32;
        let mut omitcount = 0;

        if omitzeros {
            if fpos > 0 {
                let fchars: Vec<char> = fconvert_s.chars().collect();
                while omitcount < fpos && fchars[omitcount] == '0' {
                    omitcount += 1;
                }
            } else {
                omitcount = precision as usize;
                leadfraczeros = 0;
            }
            precision -= omitcount as i32;
        }

        let emitpoint = precision > 0 || flags & PRINT_F_NUM != 0;
        let separators = if separators_flag { getnumsep(ipos as i32) } else { 0 };

        let mut padlen: i32 = width as i32
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
                outchar(s, size, sign);
                sign = '\0';
            }
            while padlen > 0 { outchar(s, size, '0'); padlen -= 1; }
        }

        while padlen > 0 { outchar(s, size, ' '); padlen -= 1; }
        if sign != '\0' { outchar(s, size, sign); }

        let ichars: Vec<char> = iconvert_s.chars().collect();
        let mut p = ichars.len();
        while p > 0 {
            p -= 1;
            outchar(s, size, ichars[p]);
            if separators > 0 && p > 0 && p % 3 == 0 {
                printsep(s, size);
            }
        }

        if emitpoint { outchar(s, size, '.'); }
        while leadfraczeros > 0 { outchar(s, size, '0'); leadfraczeros -= 1; }

        let fchars: Vec<char> = fconvert_s.chars().collect();
        let mut fp = fpos;
        while fp > omitcount {
            fp -= 1;
            outchar(s, size, fchars[fp]);
        }

        let echars: Vec<char> = econvert.chars().collect();
        let mut ep = epos;
        while ep > 0 {
            ep -= 1;
            outchar(s, size, echars[ep]);
        }

        while padlen < 0 { outchar(s, size, ' '); padlen += 1; }

        break;
    }
}

pub fn printsep(s: &mut String, size: usize) {
    outchar(s, size, ',');
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
    let digits: &[u8] = if caps != 0 { b"0123456789ABCDEF" } else { b"0123456789abcdef" };
    buf.clear();
    let mut v = value;
    loop {
        buf.push(digits[v % base] as char);
        v /= base;
        if v == 0 { break; }
    }
}

pub fn cast(value: f64) -> i32 {
    if value >= u64::MAX as f64 {
        return i32::MAX;
    }
    let result = value as u64;
    if (result as f64) <= value { result as i32 } else { (result - 1) as i32 }
}

fn myround(value: f64) -> u64 {
    let intpart = value as u64;
    if (value - intpart as f64) < 0.5 { intpart } else { intpart + 1 }
}

pub fn mypow10(exponent: i32) -> f64 {
    let mut result: f64 = 1.0;
    let mut exp = exponent;
    while exp > 0 { result *= 10.0; exp -= 1; }
    while exp < 0 { result /= 10.0; exp += 1; }
    result
}

pub fn rpl_vasprintf(
    _s: Vec<String>,
    format: &str,
    args: &[&str],
) -> i32 {
    // First pass: determine length
    let mut dummy = String::new();
    let len = rpl_vsnprintf(&mut dummy, usize::MAX, format, args);
    if len < 0 { return -1; }
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
    // Dummy main - matches the C code's dummy declaration
}
