use std::env;
use std::process;

/// Mirrors the C `static int inner = 1;` persistent state.
static mut INNER: i32 = 1;

/// Which location `running_sum` currently points to.
enum Ptr {
    Outer,
    Inner,
}

/// Reproduces the C `static_alias` function's pointer-aliasing behavior.
/// Returns which location was written to.
fn static_alias(outer: &mut i32, target: Ptr) -> Ptr {
    unsafe {
        let cur_outer = match target {
            Ptr::Outer => *outer,
            Ptr::Inner => INNER,
        };
        if cur_outer >= INNER {
            INNER += cur_outer;
            Ptr::Inner
        } else {
            match target {
                Ptr::Outer => *outer += INNER,
                Ptr::Inner => INNER += INNER,
            }
            target
        }
    }
}

fn read_val(target: &Ptr, outer: &i32) -> i32 {
    match target {
        Ptr::Outer => *outer,
        Ptr::Inner => unsafe { INNER },
    }
}

/// Parses like C strtol: leading whitespace skipped, optional sign,
/// then digits. Returns None if no digits were parsed.
fn parse_int(s: &str) -> Option<i32> {
    let s = s.trim_start();
    let mut chars = s.chars().peekable();
    let neg = match chars.peek() {
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
    if neg { val = -val; }
    Some(val as i32)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        println!("Error: should only be two (integer) arguments!");
        process::exit(1);
    }

    let initial_value = match parse_int(&args[1]) {
        Some(v) => v,
        None => {
            println!("Error: first argument must be an integer!");
            process::exit(1);
        }
    };

    let iterations = match parse_int(&args[2]) {
        Some(v) => v,
        None => {
            println!("Error: second argument must be an integer!");
            process::exit(1);
        }
    };

    let mut outer = initial_value;
    let mut running_sum = Ptr::Outer;
    for _ in 0..iterations {
        running_sum = static_alias(&mut outer, running_sum);
        println!("{}", read_val(&running_sum, &outer));
    }
}
