// Translated from c_src/src/main.c
// Preserves the buggy behavior of the original C code: bad() reads from an
// uninitialized pointer, just like the original.

use std::ffi::c_int;
use std::io::{self, Read, Write};

// ---------------------------------------------------------------------------
// Minimal printf("%d\n", ...) and scanf("%d", ...) reproduction so that the
// library prints to stdout and reads from stdin in the same byte-for-byte way
// as the C program.
// ---------------------------------------------------------------------------

fn print_int_line(value: c_int) {
    let s = format!("{}\n", value);
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(s.as_bytes());
    let _ = handle.flush();
}

/// Reads a single integer from stdin in the same way that `scanf("%d", &x)`
/// does: skip leading whitespace, then read an optional sign followed by
/// decimal digits. Returns `None` if no integer could be parsed (matching
/// scanf's failure behavior, which leaves the destination unmodified).
fn scan_int() -> Option<c_int> {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];

    // Skip leading whitespace
    loop {
        match handle.read(&mut byte) {
            Ok(0) => return None,
            Ok(_) => {
                if !byte[0].is_ascii_whitespace() {
                    break;
                }
            }
            Err(_) => return None,
        }
    }

    let mut buf: Vec<u8> = Vec::new();

    // Optional sign
    if byte[0] == b'+' || byte[0] == b'-' {
        buf.push(byte[0]);
        match handle.read(&mut byte) {
            Ok(0) => return None,
            Ok(_) => {}
            Err(_) => return None,
        }
    }

    // Must have at least one digit
    if !byte[0].is_ascii_digit() {
        return None;
    }

    while byte[0].is_ascii_digit() {
        buf.push(byte[0]);
        match handle.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }

    // We've consumed one byte too many (the non-digit) but scanf would have
    // ungetc'd it. We don't bother since the program does not read further.

    let s = std::str::from_utf8(&buf).ok()?;
    s.parse::<c_int>().ok()
}

// ---------------------------------------------------------------------------
// Translation of the original C functions.
// ---------------------------------------------------------------------------

fn print_int_ptr_line(int_number: *const c_int) {
    // Mirrors: printf("%d\n", *intNumber);
    unsafe {
        print_int_line(*int_number);
    }
}

fn bad() {
    // Mirrors:
    //     int *data;
    //     printIntPtrLine(data);
    // `data` is an uninitialized pointer; dereferencing it is undefined
    // behavior. We reproduce the bug by leaving the pointer uninitialized.
    let data: *const c_int = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
    print_int_ptr_line(data);
}

fn good() {
    // Mirrors:
    //     int data;
    //     data = 5;
    //     int *data_addr;
    //     data_addr = &data;
    //     printIntPtrLine(data_addr);
    let data: c_int = 5;
    let data_addr: *const c_int = &data;
    print_int_ptr_line(data_addr);
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> c_int {
    let mut x: c_int = 0;
    if let Some(v) = scan_int() {
        x = v;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
    0
}
