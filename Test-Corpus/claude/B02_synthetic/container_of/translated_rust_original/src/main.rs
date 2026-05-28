use std::env;

fn c_atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    // skip leading whitespace (matching C's isspace for default locale)
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' => i += 1,
            _ => break,
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
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_digit() {
            // Wrapping arithmetic to mimic C's overflow behavior
            result = result.wrapping_mul(10).wrapping_add((c - b'0') as i32);
            i += 1;
        } else {
            break;
        }
    }
    result.wrapping_mul(sign)
}

struct Test {
    a: i32,
    b: i32,
}

fn find_container_of_a(t: &Test) -> &Test {
    t
}

fn find_container_of_b(t: &Test) -> &Test {
    t
}

fn main() {
    let args: Vec<String> = env::args().collect();
    // C accesses argv[1] and argv[2] directly; replicate the indexing behavior.
    let a = c_atoi(&args[1]);
    let b = c_atoi(&args[2]);

    let mut t = Test { a: 0, b: 0 };
    // memset zero already done; assign:
    t.a = a;
    t.b = b;

    let sum = find_container_of_a(&t).a.wrapping_add(find_container_of_b(&t).b);
    println!("{}", sum);
}
