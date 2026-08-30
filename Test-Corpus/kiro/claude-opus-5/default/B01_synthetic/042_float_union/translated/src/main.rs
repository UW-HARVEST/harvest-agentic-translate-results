// Rust translation of c_src/src/main.c
//
// Original C:
//     typedef union { uint64_t x; double f; } raw_double_t;
//     void driver(double f) {
//         raw_double_t u = {.f = f};
//         printf("%llx %a %.4f\n", u.x, f, f);
//     }
//     int main() { double f = 0.0f; scanf("%lf", &f); driver(f); return 0; }
//
// The translation reproduces glibc's behaviour for `scanf("%lf", ...)`,
// `%llx`, `%a` and `%.4f` byte for byte, including the quirks of the
// scanf float scanner (e.g. a bare "0x" prefix is a matching failure, while
// "0x." parses as zero, and a partially spelled "infinity" fails).

use std::io::{Read, Write};

// ---------------------------------------------------------------------------
// scanf("%lf") emulation
// ---------------------------------------------------------------------------

fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn hex_val(c: u8) -> Option<u32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as u32),
        b'a'..=b'f' => Some((c - b'a') as u32 + 10),
        b'A'..=b'F' => Some((c - b'A') as u32 + 10),
        _ => None,
    }
}

/// Length of the longest case-insensitive prefix of `word` found at `pos`.
fn ci_prefix_len(input: &[u8], pos: usize, word: &[u8]) -> usize {
    let mut n = 0;
    while n < word.len() && pos + n < input.len() {
        let a = input[pos + n] | 0x20;
        if a != word[n] {
            break;
        }
        n += 1;
    }
    n
}

fn signed(neg: bool, v: f64) -> f64 {
    if neg {
        -v
    } else {
        v
    }
}

fn quiet_nan(neg: bool) -> f64 {
    let bits: u64 = 0x7ff8_0000_0000_0000 | if neg { 1u64 << 63 } else { 0 };
    f64::from_bits(bits)
}

/// Emulates one `%lf` conversion.  `None` means the conversion failed (input
/// or matching failure), in which case the C code leaves its variable at 0.0.
fn scan_double(input: &[u8]) -> Option<f64> {
    let len = input.len();
    let mut i = 0usize;

    // %lf skips leading white space, newlines included.
    while i < len && is_c_space(input[i]) {
        i += 1;
    }
    if i >= len {
        return None; // input failure (EOF)
    }

    let mut neg = false;
    if input[i] == b'+' || input[i] == b'-' {
        neg = input[i] == b'-';
        i += 1;
    }

    // "inf" / "infinity": glibc commits to the long spelling once a 4th
    // matching character shows up, so "infi".."infinit" are failures.
    let n = ci_prefix_len(input, i, b"infinity");
    if n >= 8 {
        return Some(signed(neg, f64::INFINITY));
    }
    if n == 3 {
        return Some(signed(neg, f64::INFINITY));
    }
    if n > 3 {
        return None;
    }

    // "nan", optionally followed by a parenthesised char sequence, which
    // glibc ignores; the result is always the default quiet NaN.
    if ci_prefix_len(input, i, b"nan") == 3 {
        return Some(quiet_nan(neg));
    }

    // Hexadecimal form: 0x / 0X prefix.
    if i < len && input[i] == b'0' && i + 1 < len && (input[i + 1] | 0x20) == b'x' {
        return scan_hex(input, i + 2, neg);
    }

    scan_decimal(input, i, neg)
}

fn scan_hex(input: &[u8], mut j: usize, neg: bool) -> Option<f64> {
    let len = input.len();
    let mut digits: Vec<u32> = Vec::new();
    let mut frac_digits: i64 = 0;
    let mut seen_dot = false;
    let mut any_digit = false;

    while j < len {
        let c = input[j];
        if c == b'.' && !seen_dot {
            seen_dot = true;
            j += 1;
            continue;
        }
        match hex_val(c) {
            Some(v) => {
                digits.push(v);
                if seen_dot {
                    frac_digits += 1;
                }
                any_digit = true;
                j += 1;
            }
            None => break,
        }
    }

    if !any_digit {
        if seen_dot {
            // glibc hands e.g. "-0x." to strtod, which converts the leading
            // "0" and stops at 'x' => signed zero, conversion succeeds.
            return Some(signed(neg, 0.0));
        }
        // Nothing but the "0x" prefix: matching failure.
        return None;
    }

    // Optional binary exponent; ignored when no digits follow it.
    let mut pexp: i64 = 0;
    if j < len && (input[j] | 0x20) == b'p' {
        let mut k = j + 1;
        let mut eneg = false;
        if k < len && (input[k] == b'+' || input[k] == b'-') {
            eneg = input[k] == b'-';
            k += 1;
        }
        if k < len && input[k].is_ascii_digit() {
            let mut v: i64 = 0;
            while k < len && input[k].is_ascii_digit() {
                if v < 1 << 40 {
                    v = v * 10 + (input[k] - b'0') as i64;
                }
                k += 1;
            }
            pexp = if eneg { -v } else { v };
        }
    }

    // value = mantissa * 16^(-frac_digits) * 2^pexp
    let mut m: u128 = 0;
    let mut sticky = false;
    let mut taken = 0;
    let mut extra_exp: i64 = 0;
    let mut started = false;
    for d in digits {
        if !started {
            if d == 0 {
                continue;
            }
            started = true;
        }
        if taken < 30 {
            m = (m << 4) | d as u128;
            taken += 1;
        } else {
            if d != 0 {
                sticky = true;
            }
            extra_exp += 4;
        }
    }
    if !started {
        return Some(signed(neg, 0.0));
    }

    let e2 = pexp
        .saturating_sub(frac_digits.saturating_mul(4))
        .saturating_add(extra_exp);
    Some(signed(neg, compose_f64(m, sticky, e2)))
}

/// Rounds `m * 2^e2` (plus a non-zero tail when `sticky`) to the nearest
/// double, ties to even, matching strtod.
fn compose_f64(mut m: u128, mut sticky: bool, mut e2: i64) -> f64 {
    if m == 0 {
        return 0.0;
    }

    let mut bl = (128 - m.leading_zeros()) as i64;
    if bl > 64 {
        let sh = (bl - 64) as u32;
        if m & ((1u128 << sh) - 1) != 0 {
            sticky = true;
        }
        m >>= sh;
        e2 = e2.saturating_add(sh as i64);
        bl = (128 - m.leading_zeros()) as i64;
    }

    let e = e2.saturating_add(bl - 1); // value == 1.f * 2^e
    if e > 1023 {
        return f64::INFINITY;
    }
    if e < -1080 {
        return 0.0;
    }

    // Target position of the least significant retained bit.
    let mut target = std::cmp::max(-1074i64, e - 52);
    let shift = target - e2;
    if shift > 0 {
        let sh = shift as u32;
        let rem = m & ((1u128 << sh) - 1);
        let half = 1u128 << (sh - 1);
        m >>= sh;
        let round_up = rem > half || (rem == half && (sticky || (m & 1) == 1));
        if round_up {
            m += 1;
        }
    } else if shift < 0 {
        m <<= (-shift) as u32;
    }
    if m == 0 {
        return 0.0;
    }

    let bl2 = (128 - m.leading_zeros()) as i64;
    let e_final = target + bl2 - 1;
    if e_final > 1023 {
        return f64::INFINITY;
    }
    if e_final < -1022 {
        // Subnormal: target == -1074, so m is the raw mantissa.
        return f64::from_bits(m as u64);
    }

    let sh = 53 - bl2;
    if sh > 0 {
        m <<= sh as u32;
        target -= sh;
    } else if sh < 0 {
        m >>= (-sh) as u32;
        target += -sh;
    }
    let _ = target;
    let bits = (((e_final + 1023) as u64) << 52) | ((m as u64) & 0x000f_ffff_ffff_ffff);
    f64::from_bits(bits)
}

fn scan_decimal(input: &[u8], i: usize, neg: bool) -> Option<f64> {
    let len = input.len();
    let mut j = i;
    let mut any_digit = false;
    let mut seen_dot = false;

    while j < len {
        let c = input[j];
        if c.is_ascii_digit() {
            any_digit = true;
            j += 1;
        } else if c == b'.' && !seen_dot {
            seen_dot = true;
            j += 1;
        } else {
            break;
        }
    }
    if !any_digit {
        return None; // matching failure
    }

    let mantissa_end = j;
    let mut end = j;
    if j < len && (input[j] | 0x20) == b'e' {
        let mut k = j + 1;
        if k < len && (input[k] == b'+' || input[k] == b'-') {
            k += 1;
        }
        if k < len && input[k].is_ascii_digit() {
            while k < len && input[k].is_ascii_digit() {
                k += 1;
            }
            end = k;
        }
    }

    // Normalise the token into a form Rust's parser accepts ("5." => "5.0",
    // ".5" => "0.5"), then rely on its correctly rounded conversion.
    let mut tok = String::new();
    if neg {
        tok.push('-');
    }
    let mant = &input[i..mantissa_end];
    if mant.first() == Some(&b'.') {
        tok.push('0');
    }
    tok.push_str(std::str::from_utf8(mant).unwrap());
    if mant.last() == Some(&b'.') {
        tok.push('0');
    }
    if end > mantissa_end {
        tok.push_str(std::str::from_utf8(&input[mantissa_end..end]).unwrap());
    }

    match tok.parse::<f64>() {
        Ok(v) => Some(v),
        Err(_) => None,
    }
}

// ---------------------------------------------------------------------------
// printf formatting
// ---------------------------------------------------------------------------

/// glibc's "%a".
fn format_hex_float(f: f64) -> String {
    let bits = f.to_bits();
    let sign = (bits >> 63) != 0;
    let exp_field = ((bits >> 52) & 0x7ff) as i64;
    let mantissa = bits & 0x000f_ffff_ffff_ffff;

    let mut out = String::new();
    if sign {
        out.push('-');
    }

    if exp_field == 0x7ff {
        out.push_str(if mantissa == 0 { "inf" } else { "nan" });
        return out;
    }

    let leading = if exp_field == 0 { '0' } else { '1' };
    let exponent: i64 = if exp_field == 0 {
        if mantissa == 0 {
            0
        } else {
            -1022
        }
    } else {
        exp_field - 1023
    };

    out.push_str("0x");
    out.push(leading);

    let digits = format!("{:013x}", mantissa);
    let trimmed = digits.trim_end_matches('0');
    if !trimmed.is_empty() {
        out.push('.');
        out.push_str(trimmed);
    }

    out.push('p');
    if exponent < 0 {
        out.push('-');
    } else {
        out.push('+');
    }
    out.push_str(&exponent.unsigned_abs().to_string());
    out
}

/// glibc's "%.4f".
fn format_fixed4(f: f64) -> String {
    let bits = f.to_bits();
    let sign = (bits >> 63) != 0;
    if f.is_nan() {
        return if sign {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if f.is_infinite() {
        return if sign {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    format!("{:.4}", f)
}

fn driver(f: f64, out: &mut impl Write) {
    // The union reinterprets the double's bits; "%llx" prints them unpadded.
    let x = f.to_bits();
    let _ = write!(
        out,
        "{:x} {} {}\n",
        x,
        format_hex_float(f),
        format_fixed4(f)
    );
}

fn main() {
    let mut input = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut input);

    let mut f: f64 = 0.0;
    if let Some(v) = scan_double(&input) {
        f = v;
    }

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    driver(f, &mut lock);
    let _ = lock.flush();
}
