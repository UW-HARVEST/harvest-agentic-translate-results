use std::env;
use std::process;

/// Mimic C strtol: skip leading whitespace, optional sign, parse digits.
/// Returns (parsed_value_as_i32, chars_consumed).
/// If no digits found, chars_consumed == 0.
fn strtol_like(s: &str) -> (i32, usize) {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    // skip whitespace
    while i < len && (bytes[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    let start = i;
    // optional sign
    let negative = if i < len && bytes[i] == b'-' {
        i += 1;
        true
    } else {
        if i < len && bytes[i] == b'+' {
            i += 1;
        }
        false
    };
    let digit_start = i;
    let mut val: i64 = 0;
    while i < len && bytes[i].is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    if i == digit_start {
        // no digits parsed
        return (0, 0);
    }
    if negative {
        val = val.wrapping_neg();
    }
    (val as i32, i - start)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Error: should only be a single (integer) argument!");
        process::exit(1);
    }

    let (mut val, consumed) = strtol_like(&args[1]);
    if consumed == 0 {
        println!("Error: first argument must be an integer!");
        process::exit(1);
    }

    loop {
        println!("{}", val);
        if val % 10 == 9 {
            break;
        }
        val = val.wrapping_add(1);
    }
}
