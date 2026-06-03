// Pure-Rust port of a small subset of the C99 snprintf replacement
// (`snprintf.c` by Patrick Powell / Holger Weiss).  The signatures defined in
// the project don't allow us to faithfully re-implement the variadic C API, so
// we provide a best-effort interpreter that works with a slice of pre-stringified
// arguments.  This is sufficient for the way `xopt.c` uses `rpl_vsnprintf`
// (single error message formatting).

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Default,
    Flags,
    Width,
    Dot,
    Precision,
    Mod,
    Conv,
}

/// Append `ch` to `s` if the current length is below `size - 1` (mirroring the
/// `OUTCHAR` macro's behaviour), and bump `len`.
fn outchar(s: &mut String, len: &mut usize, size: usize, ch: char) {
    if *len + 1 < size {
        s.push(ch);
    }
    *len += 1;
}

fn outstr(s: &mut String, len: &mut usize, size: usize, value: &str) {
    for ch in value.chars() {
        outchar(s, len, size, ch);
    }
}

/// Best-effort interpretation of a printf-style format string against a slice
/// of pre-stringified arguments.  Returns the total number of characters that
/// would have been written had `n` been large enough (matching the C99 return
/// value of `vsnprintf`).
pub fn rpl_vsnprintf(s: &mut String, n: usize, format: &str, args: &[&str]) -> i32 {
    s.clear();

    let mut len: usize = 0;
    let size = n;
    let mut state = State::Default;
    let mut flags: i32 = 0;
    let mut width: i32 = 0;
    let mut precision: i32 = -1;
    let mut base: i32 = 0;
    // The `cflags` from C aren't terribly meaningful here because we receive
    // strings rather than typed args, but we still consume length modifiers so
    // they don't break parsing.
    let mut arg_index: usize = 0;

    let chars: Vec<char> = format.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        match state {
            State::Default => {
                if ch == '%' {
                    state = State::Flags;
                } else {
                    outchar(s, &mut len, size, ch);
                }
                i += 1;
            }
            State::Flags => {
                match ch {
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
                        state = State::Width;
                    }
                }
            }
            State::Width => {
                if let Some(d) = ch.to_digit(10) {
                    width = width.saturating_mul(10).saturating_add(d as i32);
                    i += 1;
                } else if ch == '*' {
                    if let Some(arg) = args.get(arg_index) {
                        arg_index += 1;
                        if let Ok(v) = arg.parse::<i32>() {
                            if v < 0 {
                                flags |= PRINT_F_MINUS;
                                width = -v;
                            } else {
                                width = v;
                            }
                        }
                    }
                    i += 1;
                    state = State::Dot;
                } else {
                    state = State::Dot;
                }
            }
            State::Dot => {
                if ch == '.' {
                    state = State::Precision;
                    i += 1;
                } else {
                    state = State::Mod;
                }
            }
            State::Precision => {
                if precision == -1 {
                    precision = 0;
                }
                if let Some(d) = ch.to_digit(10) {
                    precision = precision.saturating_mul(10).saturating_add(d as i32);
                    i += 1;
                } else if ch == '*' {
                    if let Some(arg) = args.get(arg_index) {
                        arg_index += 1;
                        if let Ok(v) = arg.parse::<i32>() {
                            precision = if v < 0 { -1 } else { v };
                        }
                    }
                    i += 1;
                    state = State::Mod;
                } else {
                    state = State::Mod;
                }
            }
            State::Mod => {
                match ch {
                    'h' => {
                        i += 1;
                        if i < chars.len() && chars[i] == 'h' {
                            i += 1;
                        }
                    }
                    'l' => {
                        i += 1;
                        if i < chars.len() && chars[i] == 'l' {
                            i += 1;
                        }
                    }
                    'L' | 'j' | 't' | 'z' => {
                        i += 1;
                    }
                    _ => {}
                }
                state = State::Conv;
            }
            State::Conv => {
                let arg_opt = args.get(arg_index).copied();
                match ch {
                    'd' | 'i' => {
                        if let Some(arg) = arg_opt {
                            arg_index += 1;
                            let value: i64 = arg.parse().unwrap_or(0);
                            fmtint_i64(
                                s,
                                &mut len,
                                size,
                                value,
                                10,
                                width as usize,
                                precision,
                                flags,
                            );
                        }
                    }
                    'X' => {
                        flags |= PRINT_F_UP;
                        base = 16;
                        flags |= PRINT_F_UNSIGNED;
                        if let Some(arg) = arg_opt {
                            arg_index += 1;
                            let v: u64 = parse_unsigned(arg);
                            fmtuint(s, &mut len, size, v, base, width as usize, precision, flags);
                        }
                    }
                    'x' => {
                        base = 16;
                        flags |= PRINT_F_UNSIGNED;
                        if let Some(arg) = arg_opt {
                            arg_index += 1;
                            let v: u64 = parse_unsigned(arg);
                            fmtuint(s, &mut len, size, v, base, width as usize, precision, flags);
                        }
                    }
                    'o' => {
                        base = 8;
                        flags |= PRINT_F_UNSIGNED;
                        if let Some(arg) = arg_opt {
                            arg_index += 1;
                            let v: u64 = parse_unsigned(arg);
                            fmtuint(s, &mut len, size, v, base, width as usize, precision, flags);
                        }
                    }
                    'u' => {
                        base = 10;
                        flags |= PRINT_F_UNSIGNED;
                        if let Some(arg) = arg_opt {
                            arg_index += 1;
                            let v: u64 = parse_unsigned(arg);
                            fmtuint(s, &mut len, size, v, base, width as usize, precision, flags);
                        }
                    }
                    'F' => {
                        flags |= PRINT_F_UP;
                        if let Some(arg) = arg_opt {
                            arg_index += 1;
                            let v: f64 = arg.parse().unwrap_or(0.0);
                            fmtflt_inner(s, &mut len, size, v, width as usize, precision, flags);
                        }
                    }
                    'f' => {
                        if let Some(arg) = arg_opt {
                            arg_index += 1;
                            let v: f64 = arg.parse().unwrap_or(0.0);
                            fmtflt_inner(s, &mut len, size, v, width as usize, precision, flags);
                        }
                    }
                    'E' => {
                        flags |= PRINT_F_UP | PRINT_F_TYPE_E;
                        if let Some(arg) = arg_opt {
                            arg_index += 1;
                            let v: f64 = arg.parse().unwrap_or(0.0);
                            fmtflt_inner(s, &mut len, size, v, width as usize, precision, flags);
                        }
                    }
                    'e' => {
                        flags |= PRINT_F_TYPE_E;
                        if let Some(arg) = arg_opt {
                            arg_index += 1;
                            let v: f64 = arg.parse().unwrap_or(0.0);
                            fmtflt_inner(s, &mut len, size, v, width as usize, precision, flags);
                        }
                    }
                    'G' => {
                        flags |= PRINT_F_UP | PRINT_F_TYPE_G;
                        if precision == 0 {
                            precision = 1;
                        }
                        if let Some(arg) = arg_opt {
                            arg_index += 1;
                            let v: f64 = arg.parse().unwrap_or(0.0);
                            fmtflt_inner(s, &mut len, size, v, width as usize, precision, flags);
                        }
                    }
                    'g' => {
                        flags |= PRINT_F_TYPE_G;
                        if precision == 0 {
                            precision = 1;
                        }
                        if let Some(arg) = arg_opt {
                            arg_index += 1;
                            let v: f64 = arg.parse().unwrap_or(0.0);
                            fmtflt_inner(s, &mut len, size, v, width as usize, precision, flags);
                        }
                    }
                    'c' => {
                        if let Some(arg) = arg_opt {
                            arg_index += 1;
                            if let Some(c) = arg.chars().next() {
                                outchar(s, &mut len, size, c);
                            }
                        }
                    }
                    's' => {
                        if let Some(arg) = arg_opt {
                            arg_index += 1;
                            fmtstr_inner(
                                s,
                                &mut len,
                                size,
                                arg,
                                width as usize,
                                precision,
                                flags,
                            );
                        } else {
                            fmtstr_inner(
                                s,
                                &mut len,
                                size,
                                "(null)",
                                width as usize,
                                precision,
                                flags,
                            );
                        }
                    }
                    'p' => {
                        if let Some(arg) = arg_opt {
                            arg_index += 1;
                            outstr(s, &mut len, size, arg);
                        } else {
                            outstr(s, &mut len, size, "(nil)");
                        }
                    }
                    'n' => {
                        // The "%n" specifier writes the count back to a pointer
                        // in C; with our string-based args we can't honour it.
                        if arg_opt.is_some() {
                            arg_index += 1;
                        }
                    }
                    '%' => {
                        outchar(s, &mut len, size, '%');
                    }
                    _ => {
                        // Unknown specifier: skip, matching C behaviour.
                    }
                }
                i += 1;
                state = State::Default;
                base = 0;
                flags = 0;
                width = 0;
                precision = -1;
            }
        }
    }

    if len < size {
        // Implicitly nul-terminated by the use of String, no action required.
    }

    if len > i32::MAX as usize {
        return -1;
    }
    len as i32
}

fn parse_unsigned(arg: &str) -> u64 {
    if let Some(rest) = arg.strip_prefix("0x").or_else(|| arg.strip_prefix("0X")) {
        u64::from_str_radix(rest, 16).unwrap_or(0)
    } else if let Some(rest) = arg.strip_prefix('0') {
        if rest.is_empty() {
            0
        } else {
            u64::from_str_radix(rest, 8).unwrap_or_else(|_| arg.parse::<u64>().unwrap_or(0))
        }
    } else if let Ok(v) = arg.parse::<u64>() {
        v
    } else if let Ok(v) = arg.parse::<i64>() {
        v as u64
    } else {
        0
    }
}

/// Public version mirroring the C `fmtstr` signature.  `value` must already be
/// available as a Rust string (the C version accepted a NUL-terminated `char*`
/// and substituted "(null)" for `NULL`).
pub fn fmtstr(
    s: &mut String,
    size: usize,
    value: &str,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let mut len = s.chars().count();
    let prec = precision as i32;
    let no_precision = precision == usize::MAX;
    let actual_prec = if no_precision { -1 } else { prec };
    let str_chars: Vec<char> = value.chars().collect();
    let strln = if actual_prec < 0 {
        str_chars.len() as i32
    } else {
        std::cmp::min(actual_prec, str_chars.len() as i32)
    };
    let mut padlen = width as i32 - strln;
    if padlen < 0 {
        padlen = 0;
    }
    if flags & PRINT_F_MINUS != 0 {
        padlen = -padlen;
    }
    while padlen > 0 {
        outchar(s, &mut len, size, ' ');
        padlen -= 1;
    }
    let mut emitted = 0;
    for c in &str_chars {
        if actual_prec >= 0 && emitted >= actual_prec {
            break;
        }
        outchar(s, &mut len, size, *c);
        emitted += 1;
    }
    while padlen < 0 {
        outchar(s, &mut len, size, ' ');
        padlen += 1;
    }
}

/// Internal helper used by the `rpl_vsnprintf` interpreter — keeps the same
/// behaviour as the C `fmtstr` but with explicit precision -1 meaning "no
/// precision".
fn fmtstr_inner(
    s: &mut String,
    len: &mut usize,
    size: usize,
    value: &str,
    width: usize,
    precision: i32,
    flags: i32,
) {
    let str_chars: Vec<char> = value.chars().collect();
    let strln = if precision < 0 {
        str_chars.len() as i32
    } else {
        std::cmp::min(precision, str_chars.len() as i32)
    };
    let mut padlen = width as i32 - strln;
    if padlen < 0 {
        padlen = 0;
    }
    if flags & PRINT_F_MINUS != 0 {
        padlen = -padlen;
    }
    while padlen > 0 {
        outchar(s, len, size, ' ');
        padlen -= 1;
    }
    let mut emitted = 0;
    for c in &str_chars {
        if precision >= 0 && emitted >= precision {
            break;
        }
        outchar(s, len, size, *c);
        emitted += 1;
    }
    while padlen < 0 {
        outchar(s, len, size, ' ');
        padlen += 1;
    }
}

// fmtstr_inner doesn't get called via the public fmtstr since rpl_vsnprintf
// uses it directly through fmtstr() above with usize-typed precision; redirect
// the public signature to the inner.
#[allow(dead_code)]
fn _use_fmtstr_inner_marker() {
    let mut s = String::new();
    let mut l = 0usize;
    fmtstr_inner(&mut s, &mut l, 0, "", 0, -1, 0);
}

/// Public 32-bit signed integer formatter mirroring the C `fmtint` signature.
pub fn fmtint(
    s: &mut String,
    size: usize,
    value: i32,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let mut len = s.chars().count();
    let prec = if precision == usize::MAX {
        -1
    } else {
        precision as i32
    };
    fmtint_i64(s, &mut len, size, value as i64, 10, width, prec, flags);
}

fn fmtint_i64(
    s: &mut String,
    len: &mut usize,
    size: usize,
    value: i64,
    base: i32,
    width: usize,
    precision: i32,
    flags: i32,
) {
    let uvalue: u64;
    let mut sign: char = '\0';
    if flags & PRINT_F_UNSIGNED != 0 {
        uvalue = value as u64;
    } else if value < 0 {
        uvalue = (-(value as i128)) as u64;
        sign = '-';
    } else {
        uvalue = value as u64;
        if flags & PRINT_F_PLUS != 0 {
            sign = '+';
        } else if flags & PRINT_F_SPACE != 0 {
            sign = ' ';
        }
    }
    fmtuint_inner(s, len, size, uvalue, base, width, precision, flags, sign);
}

fn fmtuint(
    s: &mut String,
    len: &mut usize,
    size: usize,
    value: u64,
    base: i32,
    width: usize,
    precision: i32,
    flags: i32,
) {
    fmtuint_inner(s, len, size, value, base, width, precision, flags, '\0');
}

fn fmtuint_inner(
    s: &mut String,
    len: &mut usize,
    size: usize,
    uvalue: u64,
    base: i32,
    width: usize,
    precision: i32,
    flags: i32,
    sign_in: char,
) {
    let mut sign = sign_in;
    let mut buf = String::new();
    let pos = convert_u64(uvalue, &mut buf, base as usize, flags & PRINT_F_UP != 0);
    let iconvert: Vec<char> = buf.chars().collect();

    let no_precision = precision < 0;
    let mut precision = precision;
    let mut hexprefix: char = '\0';

    if flags & PRINT_F_NUM != 0 && uvalue != 0 {
        match base {
            8 => {
                if precision <= pos as i32 {
                    precision = pos as i32 + 1;
                }
            }
            16 => {
                hexprefix = if flags & PRINT_F_UP != 0 { 'X' } else { 'x' };
            }
            _ => {}
        }
    }

    let separators = if flags & PRINT_F_QUOTE != 0 {
        getnumsep(pos as i32)
    } else {
        0
    };

    let mut zpadlen = precision - pos as i32 - separators;
    let mut spadlen = width as i32
        - separators
        - std::cmp::max(precision, pos as i32)
        - if sign != '\0' { 1 } else { 0 }
        - if hexprefix != '\0' { 2 } else { 0 };

    if zpadlen < 0 {
        zpadlen = 0;
    }
    if spadlen < 0 {
        spadlen = 0;
    }

    if flags & PRINT_F_MINUS != 0 {
        spadlen = -spadlen;
    } else if flags & PRINT_F_ZERO != 0 && no_precision {
        zpadlen += spadlen;
        spadlen = 0;
    }

    while spadlen > 0 {
        outchar(s, len, size, ' ');
        spadlen -= 1;
    }
    if sign != '\0' {
        outchar(s, len, size, sign);
        sign = '\0';
        let _ = sign;
    }
    if hexprefix != '\0' {
        outchar(s, len, size, '0');
        outchar(s, len, size, hexprefix);
    }
    while zpadlen > 0 {
        outchar(s, len, size, '0');
        zpadlen -= 1;
    }
    let mut p = pos as i32;
    let mut sep_remaining = separators;
    while p > 0 {
        p -= 1;
        outchar(s, len, size, iconvert[p as usize]);
        if sep_remaining > 0 && p > 0 && p % 3 == 0 {
            printsep_inner(s, len, size);
        }
        let _ = sep_remaining;
        sep_remaining = separators; // separators count is for total, simplified
    }
    while spadlen < 0 {
        outchar(s, len, size, ' ');
        spadlen += 1;
    }
}

/// Public f64 formatter.  Mirrors the C `fmtflt` signature — `width` and
/// `precision` are taken as `usize` here.
pub fn fmtflt(
    s: &mut String,
    size: usize,
    value: f64,
    width: usize,
    precision: usize,
    flags: i32,
) {
    let mut len = s.chars().count();
    let prec = if precision == usize::MAX {
        -1
    } else {
        precision as i32
    };
    fmtflt_inner(s, &mut len, size, value, width, prec, flags);
}

fn fmtflt_inner(
    s: &mut String,
    len: &mut usize,
    size: usize,
    fvalue: f64,
    width: usize,
    precision: i32,
    flags: i32,
) {
    let mut precision = if precision == -1 { 6 } else { precision };

    let mut sign: char = '\0';
    if fvalue < 0.0 {
        sign = '-';
    } else if flags & PRINT_F_PLUS != 0 {
        sign = '+';
    } else if flags & PRINT_F_SPACE != 0 {
        sign = ' ';
    }

    if fvalue.is_nan() {
        let infnan = if flags & PRINT_F_UP != 0 { "NAN" } else { "nan" };
        let mut tmp = String::new();
        if sign != '\0' {
            tmp.push(sign);
        }
        tmp.push_str(infnan);
        let plen = tmp.chars().count();
        fmtstr_inner(s, len, size, &tmp, width, plen as i32, flags);
        return;
    }
    if fvalue.is_infinite() {
        let infnan = if flags & PRINT_F_UP != 0 { "INF" } else { "inf" };
        let mut tmp = String::new();
        if sign != '\0' {
            tmp.push(sign);
        }
        tmp.push_str(infnan);
        let plen = tmp.chars().count();
        fmtstr_inner(s, len, size, &tmp, width, plen as i32, flags);
        return;
    }

    let mut estyle = (flags & PRINT_F_TYPE_E) != 0;
    let mut omitzeros = false;
    let g_mode = (flags & PRINT_F_TYPE_G) != 0;
    let mut exponent: i32 = 0;
    if g_mode || estyle {
        if g_mode {
            precision -= 1;
            if flags & PRINT_F_NUM == 0 {
                omitzeros = true;
            }
        }
        exponent = getexponent(fvalue);
        estyle = true;
    }

    let mut intpart: u64;
    let mut fracpart: u64;
    let mut omitcount: i32 = 0;
    let mut leadfraczeros: i32;
    let mut ipos: i32;
    let mut fpos: i32 = 0;
    let mut epos: i32 = 0;
    let mut econvert: Vec<char> = Vec::new();
    let mut iconvert_str = String::new();
    let mut fconvert_str = String::new();

    loop {
        if precision > 19 {
            precision = 19;
        }

        let mut ufvalue = if fvalue >= 0.0 { fvalue } else { -fvalue };
        if estyle {
            ufvalue /= mypow10(exponent);
        }

        intpart = cast_u64(ufvalue);
        let mask_f = mypow10(precision);
        let frac_in = mask_f * (ufvalue - intpart as f64);
        fracpart = myround_u64(frac_in);
        let mask_u = if (mask_f as u64) > 0 {
            mask_f as u64
        } else {
            1
        };
        if fracpart >= mask_u && precision >= 0 {
            intpart += 1;
            fracpart = 0;
            if estyle && intpart == 10 {
                intpart = 1;
                exponent += 1;
            }
        }

        if g_mode && estyle && precision + 1 > exponent && exponent >= -4 {
            precision -= exponent;
            estyle = false;
            continue;
        }

        epos = 0;
        econvert.clear();
        if estyle {
            let (esign, exp_abs) = if exponent < 0 {
                ('-', -exponent)
            } else {
                ('+', exponent)
            };
            let mut buf = String::new();
            epos = convert_u64(exp_abs as u64, &mut buf, 10, false) as i32;
            econvert = buf.chars().collect();
            if epos == 1 {
                econvert.push('0');
                epos += 1;
            }
            econvert.push(esign);
            epos += 1;
            econvert.push(if flags & PRINT_F_UP != 0 { 'E' } else { 'e' });
            epos += 1;
        }

        iconvert_str.clear();
        ipos = convert_u64(intpart, &mut iconvert_str, 10, false) as i32;
        fconvert_str.clear();
        fpos = if fracpart != 0 {
            convert_u64(fracpart, &mut fconvert_str, 10, false) as i32
        } else {
            0
        };

        leadfraczeros = precision - fpos;

        if omitzeros {
            if fpos > 0 {
                let fconvert_chars: Vec<char> = fconvert_str.chars().collect();
                while omitcount < fpos && fconvert_chars[omitcount as usize] == '0' {
                    omitcount += 1;
                }
            } else {
                omitcount = precision;
                leadfraczeros = 0;
            }
            precision -= omitcount;
        }
        break;
    }

    let emitpoint = precision > 0 || (flags & PRINT_F_NUM) != 0;
    let separators = if flags & PRINT_F_QUOTE != 0 {
        getnumsep(ipos)
    } else {
        0
    };

    let mut padlen = width as i32
        - ipos
        - epos
        - precision
        - separators
        - if emitpoint { 1 } else { 0 }
        - if sign != '\0' { 1 } else { 0 };

    if padlen < 0 {
        padlen = 0;
    }
    let mut sign_local = sign;
    if flags & PRINT_F_MINUS != 0 {
        padlen = -padlen;
    } else if (flags & PRINT_F_ZERO) != 0 && padlen > 0 {
        if sign_local != '\0' {
            outchar(s, len, size, sign_local);
            sign_local = '\0';
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
    if sign_local != '\0' {
        outchar(s, len, size, sign_local);
    }

    let iconvert: Vec<char> = iconvert_str.chars().collect();
    let fconvert: Vec<char> = fconvert_str.chars().collect();

    let mut p = ipos;
    while p > 0 {
        p -= 1;
        outchar(s, len, size, iconvert[p as usize]);
        if separators > 0 && p > 0 && p % 3 == 0 {
            printsep_inner(s, len, size);
        }
    }
    if emitpoint {
        outchar(s, len, size, '.');
    }
    let mut lfz = leadfraczeros;
    while lfz > 0 {
        outchar(s, len, size, '0');
        lfz -= 1;
    }
    let mut fp = fpos;
    while fp > omitcount {
        fp -= 1;
        outchar(s, len, size, fconvert[fp as usize]);
    }
    let mut ep = epos;
    while ep > 0 {
        ep -= 1;
        outchar(s, len, size, econvert[ep as usize]);
    }
    while padlen < 0 {
        outchar(s, len, size, ' ');
        padlen += 1;
    }
}

/// Append a "thousand-separator" character.  The C version consults
/// `localeconv()`; we hard-code the comma (which the C fallback also uses).
pub fn printsep(s: &mut String, size: usize) {
    let mut len = s.chars().count();
    printsep_inner(s, &mut len, size);
}

fn printsep_inner(s: &mut String, len: &mut usize, size: usize) {
    outchar(s, len, size, ',');
}

/// Number of separator characters required for `digits` integer digits when the
/// "'" flag is in effect.
pub fn getnumsep(digits: i32) -> i32 {
    if digits <= 0 {
        return 0;
    }
    (digits - if digits % 3 == 0 { 1 } else { 0 }) / 3
}

/// Compute the decimal exponent of `value` (number of digits before the
/// decimal point, minus one), clamped to the same +/-99 range used by the C
/// implementation.
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

/// Convert `value` into base-`base` ASCII digits (in reverse order, matching
/// the C `convert` helper) and append them to `buf`.  The Rust signature uses
/// `usize` for the value; for hex-style conversions we use the `caps` flag to
/// pick between lower- and upper-case digits.
pub fn convert(value: usize, buf: &mut String, base: usize, caps: usize) {
    convert_u64(value as u64, buf, base, caps != 0);
}

fn convert_u64(mut value: u64, buf: &mut String, base: usize, caps: bool) -> usize {
    let digits: &[u8] = if caps {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    let base = base as u64;
    if base == 0 {
        return 0;
    }
    let start = buf.chars().count();
    loop {
        let d = (value % base) as usize;
        buf.push(digits[d] as char);
        value /= base;
        if value == 0 {
            break;
        }
    }
    buf.chars().count() - start
}

/// Cast a floating-point value to `i32`, mirroring the C `cast` helper that
/// avoids round-toward-zero quirks.
pub fn cast(value: f64) -> i32 {
    if !value.is_finite() {
        return i32::MAX;
    }
    if value >= i32::MAX as f64 {
        return i32::MAX;
    }
    let truncated = value.trunc() as i32;
    if (truncated as f64) <= value {
        truncated
    } else {
        truncated - 1
    }
}

fn cast_u64(value: f64) -> u64 {
    if !value.is_finite() {
        return u64::MAX;
    }
    if value >= u64::MAX as f64 {
        return u64::MAX;
    }
    if value < 0.0 {
        return 0;
    }
    let truncated = value.trunc() as u64;
    if (truncated as f64) <= value {
        truncated
    } else {
        truncated - 1
    }
}

fn myround_u64(value: f64) -> u64 {
    let intpart = cast_u64(value);
    if (value - intpart as f64) < 0.5 {
        intpart
    } else {
        intpart + 1
    }
}

/// Compute `10^exponent` using simple repeated multiplication so the result
/// is bit-identical to the C reference implementation.
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

/// Allocate-as-needed equivalent of the C `vasprintf`.  Because of the
/// project's signature constraints the buffer is passed as a `Vec<String>`
/// (interpreted as a single-slot output container).
pub fn rpl_vasprintf(mut s: Vec<String>, format: &str, args: &[&str]) -> i32 {
    let mut buf = String::new();
    let len = rpl_vsnprintf(&mut buf, usize::MAX, format, args);
    if len < 0 {
        return -1;
    }
    if s.is_empty() {
        s.push(buf);
    } else {
        s[0] = buf;
    }
    len
}

/// `asprintf` analogue: write the formatted text into `s`, replacing any
/// existing contents.
pub fn rpl_asprintf(s: &mut String, format: &str, args: &[&str]) -> i32 {
    rpl_vsnprintf(s, usize::MAX, format, args)
}

/// The original `snprintf.c` exposes a `main` only behind `#if TEST_SNPRINTF`,
/// so we provide a no-op equivalent here.
pub fn main() {
    // Intentionally empty — the C test harness is gated behind TEST_SNPRINTF.
}
