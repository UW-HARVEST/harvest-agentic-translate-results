use std::io::{self, Read};

fn is_negative(f: f64) -> bool {
    f.to_bits() >> 63 != 0
}

fn format_hex_float(f: f64) -> String {
    if f.is_nan() {
        return if is_negative(f) { "-nan".to_string() } else { "nan".to_string() };
    }
    if f.is_infinite() {
        return if is_negative(f) { "-inf".to_string() } else { "inf".to_string() };
    }
    let bits = f.to_bits();
    let sign = bits >> 63;
    let exp_bits = ((bits >> 52) & 0x7ff) as i64;
    let mantissa = bits & 0x000f_ffff_ffff_ffff;
    let prefix = if sign != 0 { "-" } else { "" };

    if exp_bits == 0 && mantissa == 0 {
        // +-0
        return format!("{}0x0p+0", prefix);
    }

    if exp_bits == 0 {
        // subnormal: 0x0.<mantissa hex>p-1022
        let hex = format!("{:013x}", mantissa);
        let trimmed = hex.trim_end_matches('0');
        return format!("{}0x0.{}p-1022", prefix, trimmed);
    }

    // normal: 0x1.<mantissa hex>p<+-><exp>
    let exp = exp_bits - 1023;
    let hex = format!("{:013x}", mantissa);
    let trimmed = hex.trim_end_matches('0');
    if trimmed.is_empty() {
        format!("{}0x1p{:+}", prefix, exp)
    } else {
        format!("{}0x1.{}p{:+}", prefix, trimmed, exp)
    }
}

fn format_f4(f: f64) -> String {
    if f.is_nan() {
        return if is_negative(f) { "-nan".to_string() } else { "nan".to_string() };
    }
    if f.is_infinite() {
        return if is_negative(f) { "-inf".to_string() } else { "inf".to_string() };
    }
    format!("{:.4}", f)
}

fn driver(f: f64) {
    let bits = f.to_bits();
    println!("{:x} {} {}", bits, format_hex_float(f), format_f4(f));
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let f: f64 = input.trim().parse().unwrap_or(0.0);
    driver(f);
}
