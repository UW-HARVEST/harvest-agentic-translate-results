use std::env;

/// Mimics C's atoi: skips leading whitespace, optional sign, parses
/// digits until non-digit, returns 0 on parse failure. Wraps on overflow
/// (UB in C, but we use wrapping arithmetic for stability).
fn c_atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0usize;

    // Skip leading whitespace as per isspace
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' => i += 1,
            _ => break,
        }
    }

    let mut sign: i32 = 1;
    if i < bytes.len() {
        match bytes[i] {
            b'+' => i += 1,
            b'-' => {
                sign = -1;
                i += 1;
            }
            _ => {}
        }
    }

    let mut result: i32 = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if !c.is_ascii_digit() {
            break;
        }
        let d = (c - b'0') as i32;
        result = result.wrapping_mul(10).wrapping_add(d);
        i += 1;
    }

    result.wrapping_mul(sign)
}

#[repr(C)]
struct Test {
    a: i32,
    b: i32,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    // C would dereference argv[1]/argv[2] unconditionally; mimic by panicking if missing.
    let a = c_atoi(&args[1]);
    let b = c_atoi(&args[2]);

    let mut t = Test { a: 0, b: 0 };
    // memset to 0 then set fields
    t.a = a;
    t.b = b;

    // container_of(&t.a, struct test, a)->a is just t.a
    // container_of(&t.b, struct test, b)->b is just t.b
    let sum = t.a.wrapping_add(t.b);
    println!("{}", sum);
}
