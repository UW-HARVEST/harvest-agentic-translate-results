use std::env;

/// C-style atoi: parses leading optional whitespace, optional sign, then digits.
/// Returns 0 on parse failure (matching C's atoi behavior).
fn c_atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0;

    // Skip leading whitespace (matches C's isspace for typical ASCII)
    while i < bytes.len() {
        let c = bytes[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\x0b' || c == b'\x0c' || c == b'\r' {
            i += 1;
        } else {
            break;
        }
    }

    let mut sign: i32 = 1;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }

    let mut result: i32 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let digit = (bytes[i] - b'0') as i32;
        // Use wrapping arithmetic to mimic C's undefined-but-typical overflow behavior
        result = result.wrapping_mul(10).wrapping_add(digit);
        i += 1;
    }

    result.wrapping_mul(sign)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Mimic C: argv[1] and argv[2] dereferenced unconditionally.
    // If absent, this would be undefined behavior in C, so we panic here.
    let a = c_atoi(&args[1]);
    let b = c_atoi(&args[2]);

    // The original C builds a struct, zeroes it, sets t.a = a, t.b = b,
    // then prints find_container_of_a(&t.a)->a + find_container_of_b(&t.b)->b
    // which is just t.a + t.b == a + b.
    let sum = a.wrapping_add(b);

    println!("{}", sum);
}
