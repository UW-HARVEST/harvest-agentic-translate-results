// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Reproduces the original program's behavior,
// including the intentional undefined behavior in `bad()` (CWE-457:
// Use of Uninitialized Variable / dereference of an uninitialized pointer).

use std::io::{self, Read, Write};

fn print_int_ptr_line(int_number: &i32) {
    // Match printf("%d\n", *intNumber)
    println!("{}", *int_number);
}

fn bad() {
    // The original C declares `int *data;` (uninitialized pointer) and
    // dereferences it via printIntPtrLine(data). This is undefined behavior
    // and in practice causes a segmentation fault before anything is written
    // to stdout. We reproduce the "no stdout output, abnormal termination"
    // observable behavior here.
    //
    // Flush stdout (matching what would have happened in C up to this point —
    // nothing has been printed) and abort with a signal-like termination.
    let _ = io::stdout().flush();
    std::process::abort();
}

fn good() {
    let data: i32 = 5;
    let data_addr: &i32 = &data;
    print_int_ptr_line(data_addr);
}

/// Read an integer from stdin matching C's `scanf("%d", &x)` behavior:
///   - Skip leading whitespace (including newlines, spaces, tabs).
///   - Optional leading sign.
///   - Consume as many decimal digits as possible.
///   - If no integer can be parsed, leave the destination unchanged (here, 0).
fn scanf_int(x: &mut i32) {
    let mut buf = [0u8; 1];
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    // Skip leading whitespace.
    let mut c: u8 = loop {
        match handle.read(&mut buf) {
            Ok(0) => return, // EOF, x unchanged
            Ok(_) => {
                let ch = buf[0];
                if !is_c_whitespace(ch) {
                    break ch;
                }
            }
            Err(_) => return,
        }
    };

    // Optional sign.
    let mut negative = false;
    if c == b'-' || c == b'+' {
        negative = c == b'-';
        match handle.read(&mut buf) {
            Ok(0) => return, // No digits read; per C scanf, x is left unchanged.
            Ok(_) => c = buf[0],
            Err(_) => return,
        }
    }

    if !c.is_ascii_digit() {
        // No digits parsed — scanf would not assign and x stays unchanged.
        return;
    }

    // Parse digits using i64 to match C int with wrap-style overflow (here we
    // use saturating since UB is acceptable; tests presumably stay in range).
    let mut value: i64 = 0;
    loop {
        if c.is_ascii_digit() {
            let digit = (c - b'0') as i64;
            value = value.saturating_mul(10).saturating_add(digit);
            match handle.read(&mut buf) {
                Ok(0) => break,
                Ok(_) => c = buf[0],
                Err(_) => break,
            }
        } else {
            break;
        }
    }

    if negative {
        value = -value;
    }

    *x = value as i32;
}

fn is_c_whitespace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
}

fn main() {
    let mut x: i32 = 0;
    scanf_int(&mut x);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
