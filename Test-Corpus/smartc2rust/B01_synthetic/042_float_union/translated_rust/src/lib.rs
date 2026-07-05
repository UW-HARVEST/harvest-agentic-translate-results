
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

fn rust_driver(f: f64) {
    let x: u64 = f.to_bits();
    println!("{:x} {} {:.4}", x, rust_format_hex_float(f), f);
}

fn rust_format_hex_float(f: f64) -> String {
    if f.is_nan() {
        return String::from("nan");
    }
    if f.is_infinite() {
        return if f.is_sign_negative() {
            String::from("-inf")
        } else {
            String::from("inf")
        };
    }
    if f == 0.0 {
        return if f.is_sign_negative() {
            String::from("-0x0p+0")
        } else {
            String::from("0x0p+0")
        };
    }

    let bits = f.to_bits();
    let sign = (bits >> 63) & 1;
    let exp = ((bits >> 52) & 0x7ff) as i64;
    let mant = bits & 0xf_ffff_ffff_ffff;

    let (exp_val, leading) = if exp == 0 {
        (-1022i64, 0u64)
    } else {
        (exp - 1023, 1u64)
    };

    let sign_str = if sign == 1 { "-" } else { "" };

    let hex_str = format!("{:013x}", mant);
    let trimmed = hex_str.trim_end_matches('0');
    let mantissa_part = if trimmed.is_empty() {
        String::new()
    } else {
        format!(".{}", trimmed)
    };

    let exp_sign = if exp_val >= 0 { "+" } else { "-" };
    let exp_abs = exp_val.abs();

    format!(
        "{}0x{}{}p{}{}",
        sign_str, leading, mantissa_part, exp_sign, exp_abs
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn main_c_main() -> i32 {
    use std::io::BufRead;

    let stdin = std::io::stdin();
    let mut f: f64 = 0.0;
    for line in stdin.lock().lines().map_while(Result::ok) {
        if let Ok(parsed) = line.trim().parse::<f64>() {
            f = parsed;
            break;
        }
    }
    rust_driver(f);
    0
}