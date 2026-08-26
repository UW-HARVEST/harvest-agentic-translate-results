use std::env;

/// Mimics C strtol(s, &end, 10): skips leading whitespace, optional sign,
/// parses as many decimal digits as possible. Returns (value as i32, chars_consumed).
/// If no digits were parsed, chars_consumed == 0 (after skipping whitespace/sign).
fn strtol_partial(s: &str) -> (i32, bool) {
    let bytes = s.as_bytes();
    let mut i = 0;
    // skip leading whitespace
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    // optional sign
    let negative = if i < bytes.len() && bytes[i] == b'-' {
        i += 1;
        true
    } else {
        if i < bytes.len() && bytes[i] == b'+' {
            i += 1;
        }
        false
    };
    // parse digits
    let digit_start = i;
    let mut val: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        val = val * 10 + (bytes[i] - b'0') as i64;
        i += 1;
    }
    let parsed_any = i > digit_start;
    if negative {
        val = -val;
    }
    (val as i32, parsed_any)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Error: should only be a single (integer) argument!");
        std::process::exit(1);
    }

    let (mut val, parsed) = strtol_partial(&args[1]);
    if !parsed {
        println!("Error: first argument must be an integer!");
        std::process::exit(1);
    }

    loop {
        println!("{}", val);
        if val % 10 == 9 {
            break;
        }
        val = val.wrapping_add(1);
    }
}
