//! C `printf` conversion specifiers used by the original program, reproduced
//! with glibc's exact output shape.

/// `%llx` for a `uint64_t`: lowercase hex, no padding, no `0x` prefix.
pub fn format_llx(x: u64) -> String {
    format!("{:x}", x)
}

/// `%a` for a `double`, as produced by glibc's `__printf_fphex`.
///
/// * zero            -> `0x0p+0`
/// * subnormal       -> leading digit `0`, exponent fixed at `-1022`
/// * normal          -> leading digit `1`, exponent = unbiased exponent
/// * mantissa is printed as 13 hex digits with trailing zeros suppressed;
///   the `.` disappears entirely when nothing is left.
/// * infinity / NaN  -> `inf` / `nan` (sign from the sign bit)
pub fn format_a(f: f64) -> String {
    let bits = f.to_bits();
    let negative = (bits >> 63) != 0;
    let exp_field = ((bits >> 52) & 0x7ff) as i32;
    let mantissa = bits & 0x000f_ffff_ffff_ffff;

    let sign = if negative { "-" } else { "" };

    if exp_field == 0x7ff {
        return if mantissa == 0 {
            format!("{}inf", sign)
        } else {
            format!("{}nan", sign)
        };
    }

    let (leading, exponent) = if exp_field == 0 {
        if mantissa == 0 {
            ('0', 0) // glibc special-cases zero: "0x0p+0"
        } else {
            ('0', -1022) // denormalized
        }
    } else {
        ('1', exp_field - 1023)
    };

    let mut digits = format!("{:013x}", mantissa);
    while digits.ends_with('0') {
        digits.pop();
    }

    let mut out = String::new();
    out.push_str(sign);
    out.push_str("0x");
    out.push(leading);
    if !digits.is_empty() {
        out.push('.');
        out.push_str(&digits);
    }
    out.push('p');
    if exponent < 0 {
        out.push('-');
    } else {
        out.push('+');
    }
    // `exponent` is at worst -1074/+1023, so the negation is safe.
    out.push_str(&format!("{}", (exponent as i64).abs()));
    out
}

/// `%.<prec>f` for a `double`.
///
/// Finite values are delegated to Rust's exact fixed-point formatter, which
/// emits the same correctly rounded digit string as glibc (a decimal tie is
/// impossible at a fixed number of fractional digits, so the tie-breaking rule
/// never becomes observable). Non-finite values use C's spelling.
pub fn format_f(f: f64, prec: usize) -> String {
    if f.is_nan() {
        // glibc looks at the sign bit even for NaN.
        return if (f.to_bits() >> 63) != 0 {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if f.is_infinite() {
        return if f < 0.0 {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    format!("{:.*}", prec, f)
}
