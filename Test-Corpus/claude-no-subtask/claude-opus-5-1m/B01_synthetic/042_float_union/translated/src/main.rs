use std::io::Read;

fn format_hex_double(f: f64) -> String {
    let bits = f.to_bits();
    let sign = (bits >> 63) & 1;
    let exp = ((bits >> 52) & 0x7FF) as i32;
    let mantissa = bits & 0x000F_FFFF_FFFF_FFFF;

    let mut result = String::new();

    // Handle inf/nan
    if exp == 0x7FF {
        if mantissa == 0 {
            if sign == 1 {
                result.push('-');
            }
            result.push_str("inf");
        } else {
            result.push_str("nan");
        }
        return result;
    }

    if sign == 1 {
        result.push('-');
    }

    // Handle zero
    if exp == 0 && mantissa == 0 {
        result.push_str("0x0p+0");
        return result;
    }

    let (lead, real_exp) = if exp == 0 {
        // subnormal
        ('0', -1022i32)
    } else {
        // normal
        ('1', exp - 1023)
    };

    result.push_str("0x");
    result.push(lead);

    if mantissa != 0 {
        result.push('.');
        // 52 bits = 13 hex digits
        let hex_str = format!("{:013x}", mantissa);
        let trimmed = hex_str.trim_end_matches('0');
        result.push_str(trimmed);
    }

    result.push('p');
    if real_exp >= 0 {
        result.push('+');
    } else {
        result.push('-');
    }
    result.push_str(&real_exp.unsigned_abs().to_string());

    result
}

fn format_f4(f: f64) -> String {
    if f.is_nan() {
        return "nan".to_string();
    }
    if f.is_infinite() {
        if f.is_sign_negative() {
            return "-inf".to_string();
        }
        return "inf".to_string();
    }
    format!("{:.4}", f)
}

fn read_double() -> f64 {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok();

    // scanf("%lf") skips leading whitespace
    let s = input.trim_start();
    let bytes = s.as_bytes();
    let mut i: usize = 0;

    // Optional sign
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }

    // Check for special values (inf/infinity/nan), case-insensitive
    let rest = &s[i..];
    let rest_lower_8: String = rest
        .chars()
        .take(8)
        .collect::<String>()
        .to_ascii_lowercase();

    if rest_lower_8.starts_with("infinity") {
        i += 8;
    } else if rest_lower_8.starts_with("inf") {
        i += 3;
    } else if rest_lower_8.starts_with("nan") {
        i += 3;
    } else {
        let start = i;
        // Integer part digits
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        // Fractional part
        let mut had_dot = false;
        if i < bytes.len() && bytes[i] == b'.' {
            had_dot = true;
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }
        // No digits at all
        if i == start || (had_dot && i == start + 1) {
            return 0.0;
        }
        // Optional exponent
        if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
            let saved = i;
            i += 1;
            if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
                i += 1;
            }
            let digit_start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i == digit_start {
                // No exponent digits, revert
                i = saved;
            }
        }
    }

    if i == 0 {
        return 0.0;
    }

    s[..i].parse::<f64>().unwrap_or(0.0)
}

fn driver(f: f64) {
    let bits = f.to_bits();
    println!(
        "{:x} {} {}",
        bits,
        format_hex_double(f),
        format_f4(f)
    );
}

fn main() {
    let f = read_double();
    driver(f);
}
