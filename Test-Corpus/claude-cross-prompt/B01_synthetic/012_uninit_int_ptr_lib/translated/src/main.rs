// Translation of c_src/src/driver.c into Rust.
//
// The original C exposes a `driver(int useGood)` function. To make this an
// executable, the program reads a single integer (matching scanf("%d", ...))
// from stdin and uses it as the `useGood` flag, then calls the equivalent of
// driver().
//
// `bad()` in the C version dereferences an uninitialized pointer (undefined
// behavior). We faithfully reproduce that with an unsafe pointer dereference
// here so the runtime behavior matches the C program (typically a crash).

use std::io::{self, Read, Write};
use std::mem::MaybeUninit;
use std::process::ExitCode;

fn print_int_ptr_line(int_number: *const i32) {
    // Matches the C: printf("%d\n", *intNumber);
    let value = unsafe { *int_number };
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", value);
}

fn bad() {
    // Mirror C: `int *data;  printIntPtrLine(data);` — uninitialized pointer.
    let data: MaybeUninit<*const i32> = MaybeUninit::uninit();
    let ptr = unsafe { data.assume_init() };
    print_int_ptr_line(ptr);
}

fn good() {
    // Mirror C: `int data = 5; int *data_addr = &data; printIntPtrLine(...)`.
    let data: i32 = 5;
    let data_addr: *const i32 = &data;
    print_int_ptr_line(data_addr);
}

fn driver(use_good: i32) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}

/// Read all remaining stdin bytes, then parse a single integer from the front
/// of the buffer using scanf("%d", ...) semantics: skip leading whitespace
/// (including newlines), accept an optional sign, then consume decimal digits.
fn read_int_scanf(buf: &[u8]) -> Option<i32> {
    let mut i = 0usize;
    // scanf %d skips any leading whitespace.
    while i < buf.len() && (buf[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    if i >= buf.len() {
        return None;
    }
    let mut negative = false;
    if buf[i] == b'+' {
        i += 1;
    } else if buf[i] == b'-' {
        negative = true;
        i += 1;
    }
    let start = i;
    while i < buf.len() && (buf[i] as char).is_ascii_digit() {
        i += 1;
    }
    if i == start {
        return None;
    }
    // Parse digits using wrapping arithmetic to match C int overflow semantics
    // for the typical 32-bit int.
    let mut value: i32 = 0;
    for &b in &buf[start..i] {
        let d = (b - b'0') as i32;
        value = value.wrapping_mul(10);
        if negative {
            value = value.wrapping_sub(d);
        } else {
            value = value.wrapping_add(d);
        }
    }
    Some(value)
}

fn main() -> ExitCode {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return ExitCode::from(1);
    }
    let use_good = match read_int_scanf(input.as_bytes()) {
        Some(v) => v,
        None => return ExitCode::from(1),
    };
    driver(use_good);
    ExitCode::from(0)
}
