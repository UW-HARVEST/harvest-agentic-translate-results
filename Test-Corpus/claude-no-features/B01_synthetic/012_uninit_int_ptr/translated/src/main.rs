// Translation of c_src/src/main.c to Rust
// Reproduces the C program's behavior, including the deliberate use of an
// uninitialized pointer in `bad()`.

use std::io::{self, Read, Write};
use std::mem::MaybeUninit;
use std::process::ExitCode;

fn print_int_ptr_line(int_number: *const i32) {
    // Equivalent to: printf("%d\n", *intNumber);
    let value = unsafe { *int_number };
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = write!(handle, "{}\n", value);
    let _ = handle.flush();
}

fn bad() {
    // C code:
    //     int *data;            // uninitialized pointer
    //     printIntPtrLine(data);
    //
    // Reproduce by leaving the pointer uninitialized and passing it on.
    let data: MaybeUninit<*const i32> = MaybeUninit::uninit();
    let data_ptr = unsafe { data.assume_init() };
    print_int_ptr_line(data_ptr);
}

fn good() {
    let data: i32 = 5;
    let data_addr: *const i32 = &data;
    print_int_ptr_line(data_addr);
}

/// Reads the next integer from stdin in a way that mimics C's `scanf("%d", ...)`.
/// - Skips leading whitespace (including newlines).
/// - Parses an optional sign followed by decimal digits.
/// - Returns 0 if no integer can be parsed (matches the C code where `x` is
///   initialized to 0 and `scanf` may leave it untouched on EOF).
fn scanf_int() -> i32 {
    let mut buf = Vec::new();
    if io::stdin().read_to_end(&mut buf).is_err() {
        return 0;
    }

    let mut i = 0usize;
    // Skip leading whitespace as scanf does.
    while i < buf.len() && (buf[i] as char).is_whitespace() {
        i += 1;
    }

    if i >= buf.len() {
        return 0;
    }

    let mut negative = false;
    if buf[i] == b'-' {
        negative = true;
        i += 1;
    } else if buf[i] == b'+' {
        i += 1;
    }

    let start = i;
    let mut value: i64 = 0;
    while i < buf.len() && (buf[i] as char).is_ascii_digit() {
        value = value.wrapping_mul(10).wrapping_add((buf[i] - b'0') as i64);
        i += 1;
    }

    if i == start {
        // No digits parsed; scanf would not assign, leaving x at its initial 0.
        return 0;
    }

    if negative {
        value = value.wrapping_neg();
    }

    // Truncate to 32-bit int as C would.
    value as i32
}

fn main() -> ExitCode {
    let x: i32 = scanf_int();

    if x != 0 {
        good();
    } else {
        bad();
    }

    ExitCode::from(0)
}
