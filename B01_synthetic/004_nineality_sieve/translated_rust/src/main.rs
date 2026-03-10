use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Error: should only be a single (integer) argument!");
        std::process::exit(1);
    }

    // Match C's strtol behavior: parse leading integer, fail only if nothing parsed
    let s = &args[1];
    let mut end = 0;
    for (i, c) in s.char_indices() {
        if i == 0 && (c == '-' || c == '+') {
            continue;
        }
        if c.is_ascii_digit() {
            end = i + c.len_utf8();
        } else {
            break;
        }
    }

    if end == 0 {
        println!("Error: first argument must be an integer!");
        std::process::exit(1);
    }

    let val: i64 = s[..end].parse().unwrap();
    let mut val = val as i32;

    loop {
        println!("{}", val);
        if val % 10 == 9 {
            break;
        }
        val += 1;
    }
}
