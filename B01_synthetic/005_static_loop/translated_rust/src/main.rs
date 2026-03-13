use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Error: should only be a single (integer) argument!");
        process::exit(1);
    }

    // Match C strtol behavior: parse leading digits, error only if nothing parsed
    let arg = &args[1];
    let stride: i32 = match parse_leading_int(arg) {
        Some(v) => v as i32,
        None => {
            println!("Error: first argument must be an integer!");
            process::exit(1);
        }
    };

    let mut sum: i32 = 0;
    for i in 0..10 {
        sum = sum.wrapping_add((i as i32).wrapping_mul(stride));
        println!("{}", sum);
    }
}

/// Mimics C strtol: parse as many leading chars as form a valid integer.
/// Returns None if no digits were consumed (equivalent to end == str).
fn parse_leading_int(s: &str) -> Option<i64> {
    let s = s.as_bytes();
    let mut i = 0;
    // skip whitespace (strtol does this)
    while i < s.len() && (s[i] == b' ' || s[i] == b'\t' || s[i] == b'\n' || s[i] == b'\r') {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let start = i;
    let mut val: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((s[i] - b'0') as i64);
        i += 1;
    }
    if i == start {
        return None; // nothing parsed
    }
    Some(if neg { -val } else { val })
}
