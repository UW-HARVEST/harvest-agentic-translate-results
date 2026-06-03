use std::env;

/// Mimic C's atoi: parses leading optional whitespace, optional sign,
/// then leading decimal digits. Returns 0 if no valid digits.
/// Overflow is undefined in C; we use wrapping arithmetic to be deterministic.
fn c_atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0;
    // Skip leading whitespace (matching C's isspace for the "C" locale)
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' => i += 1,
            _ => break,
        }
    }
    let mut negative = false;
    if i < bytes.len() {
        match bytes[i] {
            b'-' => {
                negative = true;
                i += 1;
            }
            b'+' => {
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
        let digit = (c - b'0') as i32;
        result = result.wrapping_mul(10);
        result = if negative {
            result.wrapping_sub(digit)
        } else {
            result.wrapping_add(digit)
        };
        i += 1;
    }
    result
}

#[repr(C)]
struct Test {
    a: i32,
    b: i32,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    // Mimic C's argv[1], argv[2] access - in C, missing args is UB.
    // We unwrap to panic if not provided, similar to a crash.
    let a = c_atoi(&args[1]);
    let b = c_atoi(&args[2]);

    let mut t = Test { a: 0, b: 0 };
    // memset(&t, 0, sizeof(t)) - already zeroed
    t.a = a;
    t.b = b;

    // find_container_of_a(&t.a)->a + find_container_of_b(&t.b)->b
    // The container_of macro recovers the original struct from a member
    // pointer; the result is simply t.a + t.b.
    let sum = t.a.wrapping_add(t.b);
    println!("{}", sum);
}
