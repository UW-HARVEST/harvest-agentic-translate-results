use std::env;
use std::process;

/// Which storage location `running_sum` currently points to.
#[derive(Clone, Copy)]
enum Ptr {
    Outer,
    Inner,
}

/// Mirrors the C `static_alias` function.
/// `inner` is the persistent static, `outer` is the stack variable.
/// Returns which one the pointer now refers to.
fn static_alias(outer: &mut i32, inner: &mut i32, which: Ptr) -> Ptr {
    let outer_val = match which {
        Ptr::Outer => *outer,
        Ptr::Inner => *inner,
    };
    if outer_val >= *inner {
        *inner += outer_val;
        Ptr::Inner
    } else {
        match which {
            Ptr::Outer => *outer += *inner,
            Ptr::Inner => *inner += *inner,
        }
        which
    }
}

fn deref(outer: &i32, inner: &i32, which: Ptr) -> i32 {
    match which {
        Ptr::Outer => *outer,
        Ptr::Inner => *inner,
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        println!("Error: should only be two (integer) arguments!");
        process::exit(1);
    }

    // strtol accepts leading whitespace and a prefix of the string;
    // replicate: parse as much leading integer as possible.
    let initial_value = match parse_strtol(&args[1]) {
        Some(v) => v as i32,
        None => {
            println!("Error: first argument must be an integer!");
            process::exit(1);
        }
    };

    let iterations = match parse_strtol(&args[2]) {
        Some(v) => v as i32,
        None => {
            println!("Error: second argument must be an integer!");
            process::exit(1);
        }
    };

    let mut outer = initial_value;
    let mut inner: i32 = 1; // the C static
    let mut running_sum = Ptr::Outer;

    for _ in 0..iterations {
        running_sum = static_alias(&mut outer, &mut inner, running_sum);
        println!("{}", deref(&outer, &inner, running_sum));
    }
}

/// Mimics C strtol: skip leading whitespace, optional sign, then digits.
/// Returns None if no digits were parsed (end == start in C).
fn parse_strtol(s: &str) -> Option<i64> {
    let bytes = s.as_bytes();
    let mut i = 0;
    // skip whitespace
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    // optional sign
    let neg = if i < bytes.len() && bytes[i] == b'-' {
        i += 1;
        true
    } else {
        if i < bytes.len() && bytes[i] == b'+' {
            i += 1;
        }
        false
    };
    // need at least one digit
    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
        return None;
    }
    let mut val: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    Some(if neg { -val } else { val })
}
