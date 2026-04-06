use std::env;

/// Mimics C strtol: parse leading decimal integer from string.
/// Returns Some(value) if at least one digit was consumed, None otherwise.
fn strtol_leading(s: &str) -> Option<i64> {
    let s = s.trim_start();
    if s.is_empty() {
        return None;
    }
    let mut chars = s.chars().peekable();
    let negative = match chars.peek() {
        Some('+') => { chars.next(); false }
        Some('-') => { chars.next(); true }
        _ => false,
    };
    let mut found = false;
    let mut val: i64 = 0;
    while let Some(&c) = chars.peek() {
        if let Some(d) = c.to_digit(10) {
            found = true;
            val = val.wrapping_mul(10).wrapping_add(d as i64);
            chars.next();
        } else {
            break;
        }
    }
    if !found {
        return None;
    }
    Some(if negative { -val } else { val })
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Error: should only be a single (integer) argument!");
        std::process::exit(1);
    }

    let stride: i32 = match strtol_leading(&args[1]) {
        Some(v) => v as i32,
        None => {
            println!("Error: first argument must be an integer!");
            std::process::exit(1);
        }
    };

    let mut sum: i32 = 0;
    for i in 0..10 {
        sum = sum.wrapping_add((i as i32).wrapping_mul(stride));
        println!("{}", sum);
    }
}
