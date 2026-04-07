use std::io::{self, Read};

fn format_hex_float(f: f64) -> String {
    if f.is_nan() {
        return "nan".to_string();
    }
    if f.is_infinite() {
        return if f < 0.0 { "-inf".to_string() } else { "inf".to_string() };
    }
    let bits = f.to_bits();
    let sign = bits >> 63;
    let exp_bits = ((bits >> 52) & 0x7ff) as i64;
    let mantissa = bits & 0x000f_ffff_ffff_ffff;
    let prefix = if sign != 0 { "-" } else { "" };

    if exp_bits == 0 && mantissa == 0 {
        return format!("{}0x0p+0", prefix);
    }

    if exp_bits == 0 {
        let hex = format!("{:013x}", mantissa);
        let trimmed = hex.trim_end_matches('0');
        return format!("{}0x0.{}p-1022", prefix, trimmed);
    }

    let exp = exp_bits - 1023;
    let exp_sign = if exp >= 0 { "+" } else { "" };
    if mantissa == 0 {
        format!("{}0x1p{}{}", prefix, exp_sign, exp)
    } else {
        let hex = format!("{:013x}", mantissa);
        let trimmed = hex.trim_end_matches('0');
        format!("{}0x1.{}p{}{}", prefix, trimmed, exp_sign, exp)
    }
}

fn format_f64(f: f64, prec: usize) -> String {
    if f.is_nan() { return "nan".to_string(); }
    if f.is_infinite() { return if f < 0.0 { "-inf".to_string() } else { "inf".to_string() }; }
    format!("{:.prec$}", f, prec = prec)
}

#[no_mangle]
pub extern "C" fn driver(f: f64) {
    let bits = f.to_bits();
    let hex_float = format_hex_float(f);
    println!("{:x} {} {}", bits, hex_float, format_f64(f, 4));
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let f: f64 = input.trim().parse().unwrap_or(0.0);
    driver(f);
}
