use std::env;

/// Mimics C's `atoi`: skips leading ASCII whitespace, optional sign, then
/// parses a sequence of decimal digits. Returns 0 if no digits are found.
/// Wraps on overflow (matching glibc's atoi semantics for typical inputs).
fn c_atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0usize;

    // Skip leading whitespace (matches isspace for ASCII).
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r' => i += 1,
            _ => break,
        }
    }

    let mut sign: i32 = 1;
    if i < bytes.len() {
        match bytes[i] {
            b'-' => {
                sign = -1;
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
        if !(b'0'..=b'9').contains(&c) {
            break;
        }
        let d = (c - b'0') as i32;
        // Use wrapping arithmetic to mirror C's overflow behavior.
        result = result.wrapping_mul(10).wrapping_add(sign * d);
        i += 1;
    }

    result
}

#[derive(Default, Clone, Copy)]
struct Test {
    a: i32,
    b: i32,
}

// In the original C, container_of recovers the parent struct pointer from a
// pointer to one of its fields. Because we operate over a single owned `Test`
// value here (no raw pointer arithmetic needed), we model the behavior by
// passing the parent struct by reference and reading the appropriate field.

fn find_container_of_a(t: &Test) -> &Test {
    t
}

fn find_container_of_b(t: &Test) -> &Test {
    t
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Match the C: it indexes argv[1] and argv[2] without checking argc.
    // If those are missing, the C program invokes undefined behavior; we
    // mirror by panicking on out-of-bounds access.
    let a = c_atoi(&args[1]);
    let b = c_atoi(&args[2]);

    let mut t = Test::default();
    t.a = a;
    t.b = b;

    let sum = find_container_of_a(&t).a.wrapping_add(find_container_of_b(&t).b);
    // The C source prints with a leading space: " printf(\"%d\\n\", ...)".
    // Wait — re-check: the leading space is in source indentation only,
    // the format string itself is "%d\n", so output has no leading space.
    println!("{}", sum);
}
