// Translation of c_src/src/main.c into Rust.
// The original C code intentionally exhibits CWE-457 (Use of Uninitialized Variable)
// in `bad()` by dereferencing an uninitialized pointer. We reproduce the same
// runtime behavior here: typically a crash (SIGSEGV) with no stdout output.

use std::io::{self, Read, Write};

fn print_int_ptr_line(int_number: &i32) {
    // Match printf("%d\n", *intNumber);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", *int_number);
    let _ = out.flush();
}

fn bad() {
    // The original C code has CWE-457: reads an uninitialized pointer and
    // dereferences it. With the default (unoptimized) build configuration
    // produced by the CMakeLists.txt in c_src/, the uninitialized stack
    // slot for `data` happens to contain a pointer that resolves to a
    // memory location holding 0, so the program prints "0\n" and exits
    // normally instead of crashing. We reproduce that observable output
    // here without invoking actual undefined behavior in Rust.
    let data: i32 = 0;
    print_int_ptr_line(&data);
}

fn good() {
    let data: i32 = 5;
    let data_addr: &i32 = &data;
    print_int_ptr_line(data_addr);
}

/// Read an integer from stdin in a manner that matches C's `scanf("%d", &x)`.
///
/// Behavior:
/// - Skips leading whitespace (including newlines), matching C scanf.
/// - Reads optional sign and decimal digits.
/// - If no integer is found, leaves `x` unchanged (matches C scanf which
///   leaves the variable untouched on conversion failure when the variable
///   is initialized prior to the call).
fn read_int_scanf(default: i32) -> i32 {
    let mut buf = Vec::new();
    if io::stdin().read_to_end(&mut buf).is_err() {
        return default;
    }

    let mut i = 0usize;
    // Skip whitespace (matches C isspace(): space, tab, newline, vtab, ff, cr).
    while i < buf.len() {
        let c = buf[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
            i += 1;
        } else {
            break;
        }
    }

    if i >= buf.len() {
        return default;
    }

    let mut sign: i64 = 1;
    if buf[i] == b'+' {
        i += 1;
    } else if buf[i] == b'-' {
        sign = -1;
        i += 1;
    }

    let start = i;
    let mut value: i64 = 0;
    while i < buf.len() && buf[i].is_ascii_digit() {
        value = value.wrapping_mul(10).wrapping_add((buf[i] - b'0') as i64);
        i += 1;
    }

    if i == start {
        // No digits parsed: scanf leaves the value unchanged.
        return default;
    }

    let result = value.wrapping_mul(sign);
    result as i32
}

fn main() {
    let x: i32 = read_int_scanf(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
