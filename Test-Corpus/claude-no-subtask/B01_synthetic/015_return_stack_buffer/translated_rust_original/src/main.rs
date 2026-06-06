// Translated from C - reproduces behavior including dangling pointer "bug"
// In practice, on common platforms, the bad() path still prints "helperBad string"
// because the stack memory still contains the string at the time of the printf call.

use std::io::{self, Read, Write};

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

fn helper_bad() -> Option<&'static str> {
    // The original C returns a pointer to a stack-allocated array (undefined behavior).
    // In practice, the typical observed output is "helperBad string", so we reproduce that.
    Some("helperBad string")
}

fn bad() {
    print_line(helper_bad());
}

fn helper_good1() -> Option<&'static str> {
    Some("helperGood1 string")
}

fn good() {
    print_line(helper_good1());
}

/// Read an integer from stdin using C scanf("%d", ...) semantics:
/// skip leading whitespace (including newlines), then parse optional sign
/// followed by decimal digits. Returns 0 on no match (matching the
/// `int x = 0;` initialization in the C code).
fn scanf_int() -> i32 {
    let mut buf = Vec::new();
    if io::stdin().read_to_end(&mut buf).is_err() {
        return 0;
    }

    let mut i = 0usize;
    // Skip whitespace
    while i < buf.len() {
        let c = buf[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0B || c == 0x0C {
            i += 1;
        } else {
            break;
        }
    }

    if i >= buf.len() {
        return 0;
    }

    let mut negative = false;
    if buf[i] == b'+' {
        i += 1;
    } else if buf[i] == b'-' {
        negative = true;
        i += 1;
    }

    let start = i;
    let mut value: i64 = 0;
    while i < buf.len() && buf[i].is_ascii_digit() {
        value = value
            .saturating_mul(10)
            .saturating_add((buf[i] - b'0') as i64);
        i += 1;
    }

    if i == start {
        // No digits matched; scanf would not assign, so x stays 0.
        return 0;
    }

    let result = if negative { -value } else { value };
    // Cast to i32 to match C's int. Saturate to avoid undefined behavior in Rust.
    if result > i32::MAX as i64 {
        i32::MAX
    } else if result < i32::MIN as i64 {
        i32::MIN
    } else {
        result as i32
    }
}

fn main() {
    let x = scanf_int();

    if x != 0 {
        good();
    } else {
        bad();
    }

    // Ensure stdout is flushed
    let _ = io::stdout().flush();
}
