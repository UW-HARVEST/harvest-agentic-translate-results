// Translation of the C driver code into Rust.
//
// The original C code is a library exposing `driver(int useGood)`. We wrap
// it in an executable that reads a single integer from stdin (matching C's
// scanf("%d", ...) semantics, which skips leading whitespace including
// newlines) and invokes `driver` with that value.

use std::io::{self, Read, Write};

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        // C: printf("%s\n", line);
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        // Write directly as bytes so output is byte-identical.
        let _ = handle.write_all(s.as_bytes());
        let _ = handle.write_all(b"\n");
    }
}

// In the original C, helperBad returns a pointer to a stack-allocated buffer
// (undefined behavior). On typical compilers without optimization the bytes
// are still intact when printf reads them, so the observable behavior is
// printing "helperBad string". We reproduce that observable output.
fn helper_bad() -> Option<&'static str> {
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

fn driver(use_good: i32) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}

/// Read a single integer from stdin in a manner that mimics C's
/// scanf("%d", ...): skip leading whitespace (spaces, tabs, newlines, etc.),
/// then consume an optional sign and a run of decimal digits. Wraps using
/// two's-complement on overflow, matching common C int (32-bit) behavior.
fn scanf_int<R: Read>(reader: &mut R) -> Option<i32> {
    let mut buf = [0u8; 1];

    // Skip whitespace.
    let mut c;
    loop {
        let n = reader.read(&mut buf).ok()?;
        if n == 0 {
            return None;
        }
        c = buf[0];
        if !(c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0B || c == 0x0C) {
            break;
        }
    }

    let mut negative = false;
    if c == b'-' {
        negative = true;
        let n = reader.read(&mut buf).ok()?;
        if n == 0 {
            return None;
        }
        c = buf[0];
    } else if c == b'+' {
        let n = reader.read(&mut buf).ok()?;
        if n == 0 {
            return None;
        }
        c = buf[0];
    }

    if !c.is_ascii_digit() {
        return None;
    }

    let mut value: i32 = 0;
    loop {
        if c.is_ascii_digit() {
            let digit = (c - b'0') as i32;
            value = value.wrapping_mul(10).wrapping_add(digit);
            let n = reader.read(&mut buf).ok()?;
            if n == 0 {
                break;
            }
            c = buf[0];
        } else {
            break;
        }
    }

    if negative {
        value = value.wrapping_neg();
    }
    Some(value)
}

fn main() {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let use_good = scanf_int(&mut handle).unwrap_or(0);
    driver(use_good);
}
