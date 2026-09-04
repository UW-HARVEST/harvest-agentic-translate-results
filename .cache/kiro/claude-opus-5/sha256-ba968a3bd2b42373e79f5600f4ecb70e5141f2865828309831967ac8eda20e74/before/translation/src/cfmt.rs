//! Minimal re-implementation of the C library's `printf` `%g` conversion,
//! matching glibc byte for byte (including round-half-to-even on exact ties,
//! `nan`/`inf` spellings and the two-digit minimum exponent field).

/// Number of fractional digits requested from Rust's exact float formatter.
/// A `f64` that came from a `f32` needs at most ~113 significant decimal
/// digits to be written exactly (the worst case is the smallest subnormal),
/// so 160 fractional digits is always exact, with zero padding beyond.
const EXACT_FRAC_DIGITS: usize = 160;

/// `printf("%.*g", precision, value)` for a `double`.
pub fn format_g(value: f64, precision: usize) -> String {
    // glibc: a precision of 0 is treated as 1.
    let p = if precision == 0 { 1 } else { precision };

    if value.is_nan() {
        return if value.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }

    let neg = value.is_sign_negative();
    let mag = if neg { -value } else { value };

    let (digits, exp) = exact_decimal(mag);
    let (digits, exp) = round_significant(&digits, exp, p);

    let body = if exp < -4 || exp >= p as i32 {
        // %e style with precision p-1, trailing zeros removed.
        let mut frac: Vec<u8> = digits[1..p].to_vec();
        while frac.last() == Some(&b'0') {
            frac.pop();
        }
        let mut s = String::new();
        s.push(digits[0] as char);
        if !frac.is_empty() {
            s.push('.');
            s.push_str(std::str::from_utf8(&frac).unwrap());
        }
        let (esign, eabs) = if exp < 0 {
            ('-', (-(exp as i64)) as u64)
        } else {
            ('+', exp as u64)
        };
        s.push('e');
        s.push(esign);
        if eabs < 10 {
            s.push('0');
        }
        s.push_str(&eabs.to_string());
        s
    } else {
        // %f style with precision p-1-exp, trailing zeros removed.
        let mut s = String::new();
        if exp >= 0 {
            let int_len = (exp as usize) + 1;
            s.push_str(std::str::from_utf8(&digits[..int_len]).unwrap());
            let mut frac: Vec<u8> = digits[int_len..p].to_vec();
            while frac.last() == Some(&b'0') {
                frac.pop();
            }
            if !frac.is_empty() {
                s.push('.');
                s.push_str(std::str::from_utf8(&frac).unwrap());
            }
        } else {
            let mut frac: Vec<u8> = Vec::new();
            for _ in 0..(-exp - 1) {
                frac.push(b'0');
            }
            frac.extend_from_slice(&digits[..p]);
            while frac.last() == Some(&b'0') {
                frac.pop();
            }
            s.push('0');
            if !frac.is_empty() {
                s.push('.');
                s.push_str(std::str::from_utf8(&frac).unwrap());
            }
        }
        s
    };

    if neg {
        format!("-{}", body)
    } else {
        body
    }
}

/// Returns the exact decimal digits of `mag` (>= 0, finite) together with the
/// decimal exponent `e` such that `mag == 0.d0 d1 ... * 10^(e+1)`, i.e. the
/// value is `d0.d1 d2 ... * 10^e`.
fn exact_decimal(mag: f64) -> (Vec<u8>, i32) {
    if mag == 0.0 {
        return (vec![b'0'; EXACT_FRAC_DIGITS + 1], 0);
    }
    // Rust's `{:.*e}` uses an exact (big-integer) algorithm, so the digits are
    // the true decimal expansion, zero padded once it terminates.
    let s = format!("{:.*e}", EXACT_FRAC_DIGITS, mag);
    let (mantissa, exponent) = s.split_once('e').expect("exponential form");
    let digits: Vec<u8> = mantissa.bytes().filter(|b| *b != b'.').collect();
    let exp: i32 = exponent.parse().expect("decimal exponent");
    (digits, exp)
}

/// Rounds `digits` (value `digits[0].digits[1..] * 10^exp`) to `p` significant
/// digits using round-to-nearest, ties-to-even, as glibc does.
fn round_significant(digits: &[u8], exp: i32, p: usize) -> (Vec<u8>, i32) {
    let mut out: Vec<u8> = Vec::with_capacity(p + 1);
    out.extend_from_slice(&digits[..digits.len().min(p)]);
    while out.len() < p {
        out.push(b'0');
    }
    if digits.len() <= p {
        return (out, exp);
    }

    let round_up = {
        let first = digits[p];
        match first.cmp(&b'5') {
            std::cmp::Ordering::Greater => true,
            std::cmp::Ordering::Less => false,
            std::cmp::Ordering::Equal => {
                if digits[p + 1..].iter().any(|d| *d != b'0') {
                    true
                } else {
                    // Exact tie: round so that the last kept digit is even.
                    (out[p - 1] - b'0') % 2 == 1
                }
            }
        }
    };

    if !round_up {
        return (out, exp);
    }

    let mut i = p;
    loop {
        if i == 0 {
            // Carry out of the most significant digit: 999.. -> 1000..
            out.insert(0, b'1');
            out.truncate(p);
            return (out, exp + 1);
        }
        i -= 1;
        if out[i] == b'9' {
            out[i] = b'0';
        } else {
            out[i] += 1;
            return (out, exp);
        }
    }
}
