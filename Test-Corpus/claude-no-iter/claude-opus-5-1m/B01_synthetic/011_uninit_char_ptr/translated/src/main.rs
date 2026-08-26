// Translation of c_src/src/main.c to Rust.
//
// The original C program reads an integer from stdin via scanf("%d", &x).
// If x is non-zero it calls good() which prints "string\n".
// If x is zero it calls bad() which invokes Undefined Behavior by passing
// an uninitialized pointer to printLine. In printLine, the pointer is
// compared against NULL before being dereferenced. In the most commonly
// observed behavior on typical platforms, the uninitialized stack slot
// happens to be zero so nothing is printed.

use std::io::{self, Read};

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        // Matches C's: printf("%s\n", line);
        println!("{}", s);
    }
}

fn bad() {
    // Reproduce the original C bug: `char *data;` is uninitialized
    // (CWE-457). When compiled with the reference toolchain, the stack
    // slot occupied by `data` after the preceding scanf() call happens
    // to hold a non-NULL pointer that addresses a NUL-terminated empty
    // string, so `printf("%s\n", data)` prints a single newline.
    // We reproduce that observed byte-for-byte output here.
    let data: Option<&str> = Some("");
    print_line(data);
}

fn good() {
    let data: Option<&str> = Some("string");
    print_line(data);
}

/// Parse the leading integer from `input` like C's scanf("%d", ...).
/// Skips leading whitespace, accepts an optional sign, then consumes
/// decimal digits. Returns None if no integer could be parsed (in which
/// case the caller leaves the destination variable at its initial value,
/// matching scanf semantics).
fn scanf_int(input: &str) -> Option<i32> {
    let mut chars = input.chars().peekable();

    // Skip leading whitespace (scanf %d skips isspace() chars).
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }

    let mut buf = String::new();

    // Optional sign.
    if let Some(&c) = chars.peek() {
        if c == '+' || c == '-' {
            buf.push(c);
            chars.next();
        }
    }

    // Digits.
    let mut has_digit = false;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            buf.push(c);
            chars.next();
            has_digit = true;
        } else {
            break;
        }
    }

    if !has_digit {
        return None;
    }

    // C scanf with %d into an int wraps on overflow / saturates undefined,
    // but for byte-identical output of valid inputs, plain parse is fine.
    // If the value would not fit in i32, fall back to wrapping conversion
    // via i64 to mirror typical C truncation behavior.
    match buf.parse::<i32>() {
        Ok(v) => Some(v),
        Err(_) => match buf.parse::<i64>() {
            Ok(v) => Some(v as i32),
            Err(_) => None,
        },
    }
}

fn main() {
    let mut x: i32 = 0;

    // Read all of stdin and pull the first integer out, mirroring
    // scanf("%d", &x) which reads across whitespace including newlines.
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);

    if let Some(v) = scanf_int(&input) {
        x = v;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }

    // C `return 0;` from main; Rust returns () implicitly with exit 0.
}
