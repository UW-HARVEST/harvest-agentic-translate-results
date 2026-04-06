use std::env;
use std::process;

/// Mimic C strtol: skip leading whitespace, optional sign, then digits.
/// Returns Some(value) if at least one digit was consumed, None otherwise.
fn strtol_parse(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut i = 0;
    // skip whitespace
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    // optional sign
    let neg = bytes[i] == b'-';
    if bytes[i] == b'+' || bytes[i] == b'-' {
        i += 1;
    }
    // need at least one digit
    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
        return None;
    }
    let mut val: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    if neg {
        val = val.wrapping_neg();
    }
    Some(val as i32)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Error: should only be a single (integer) argument!");
        process::exit(1);
    }

    let val = match strtol_parse(&args[1]) {
        Some(v) => v,
        None => {
            println!("Error: first argument must be an integer!");
            process::exit(1);
        }
    };

    let mut val = val;
    loop {
        println!("{}", val);
        if val % 10 == 9 {
            break;
        }
        val = val.wrapping_add(1);
    }
}
