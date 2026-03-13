use std::io::{self, Read};

/// Format a f64 matching C's printf %a (glibc behavior).
fn format_hex_float(f: f64) -> String {
    if f.is_nan() {
        return if f.is_sign_negative() { "-nan".into() } else { "nan".into() };
    }
    if f.is_infinite() {
        return if f.is_sign_negative() { "-inf".into() } else { "inf".into() };
    }

    let bits = f.to_bits();
    let sign = bits >> 63;
    let exponent = ((bits >> 52) & 0x7ff) as i64;
    let mantissa = bits & 0x000f_ffff_ffff_ffff;

    let prefix = if sign != 0 { "-" } else { "" };

    if exponent == 0 {
        // Zero or subnormal
        if mantissa == 0 {
            return format!("{}0x0p+0", prefix);
        }
        // Subnormal: digit before dot is 0, exponent is -1022
        let hex = format!("{:013x}", mantissa);
        let trimmed = hex.trim_end_matches('0');
        return format!("{}0x0.{}p-1022", prefix, trimmed);
    }

    // Normal number
    let biased = exponent - 1023;
    let sign_char = if biased >= 0 { '+' } else { '-' };
    let abs_exp = biased.unsigned_abs();

    if mantissa == 0 {
        format!("{}0x1p{}{}", prefix, sign_char, abs_exp)
    } else {
        let hex = format!("{:013x}", mantissa);
        let trimmed = hex.trim_end_matches('0');
        format!("{}0x1.{}p{}{}", prefix, trimmed, sign_char, abs_exp)
    }
}

fn driver(f: f64) {
    let bits = f.to_bits();
    let hex_a = format_hex_float(f);
    if f.is_nan() {
        let sign = if (f.to_bits() >> 63) != 0 { "-" } else { "" };
        println!("{:x} {} {}nan", bits, hex_a, sign);
    } else if f.is_infinite() {
        let inf_str = if f.is_sign_negative() { "-inf" } else { "inf" };
        println!("{:x} {} {}", bits, hex_a, inf_str);
    } else {
        println!("{:x} {} {:.4}", bits, hex_a, f);
    }
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    // Match scanf("%lf") behavior: skip whitespace, parse first float
    let f: f64 = input.split_whitespace().next().unwrap().parse().unwrap();
    driver(f);
}
