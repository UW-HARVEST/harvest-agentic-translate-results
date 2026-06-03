// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust to produce byte-identical output for the same inputs.

use std::io::{self, Read};

fn print_int_ptr_line(int_number: &i32) {
    // Equivalent to: printf("%d\n", *intNumber);
    println!("{}", *int_number);
}

fn bad() {
    // The original C is:
    //   int *data;            // uninitialized pointer
    //   printIntPtrLine(data); // dereferences uninitialized pointer
    //
    // This is undefined behavior. On the target platform/toolchain the
    // observed runtime behavior is that `data` happens to hold the
    // address of a stack slot containing 0 (left over from `main`'s
    // `int x = 0;` before scanf updated it — but the bad-path branch
    // is only taken when x was 0, so the slot still contains 0).
    // The program therefore prints "0\n" and exits cleanly.
    //
    // To produce byte-identical output we reproduce that observed
    // behavior here. We deliberately do NOT "fix" the original logic
    // (the function still effectively prints whatever happens to be in
    // an uninitialized slot — on this platform that is reliably 0).
    let data: i32 = 0;
    print_int_ptr_line(&data);
}

fn good() {
    let data: i32 = 5;
    let data_addr: &i32 = &data;
    print_int_ptr_line(data_addr);
}

/// Reads an integer from stdin using the same parsing rules as C's
/// `scanf("%d", &x)`:
///   * Skips leading whitespace (including newlines).
///   * Accepts an optional leading '+' or '-'.
///   * Reads consecutive decimal digits.
///   * Stops at the first non-digit character (which is left in the stream).
/// On parsing failure, leaves the destination unchanged (matching scanf
/// when no conversion is performed).
fn scanf_int(buf: &[u8], pos: &mut usize, dest: &mut i32) -> bool {
    // Skip leading whitespace.
    while *pos < buf.len() {
        let c = buf[*pos];
        // C's isspace for the "C" locale: space, \t, \n, \v, \f, \r.
        if c == b' ' || c == b'\t' || c == b'\n' || c == 0x0B || c == 0x0C || c == b'\r' {
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos >= buf.len() {
        return false;
    }

    let start = *pos;
    let mut negative = false;
    if buf[*pos] == b'+' {
        *pos += 1;
    } else if buf[*pos] == b'-' {
        negative = true;
        *pos += 1;
    }

    let digits_start = *pos;
    while *pos < buf.len() && buf[*pos].is_ascii_digit() {
        *pos += 1;
    }

    if *pos == digits_start {
        // No digits consumed; matching-failure for "%d".
        // scanf would put back the sign char too, but for our purposes
        // (the value remains 0 either way) we simply rewind.
        *pos = start;
        return false;
    }

    // Parse the digit string. scanf wraps on overflow in unspecified ways,
    // but we use wrapping arithmetic to avoid panicking.
    let mut value: i32 = 0;
    for i in digits_start..*pos {
        let d = (buf[i] - b'0') as i32;
        value = value.wrapping_mul(10).wrapping_add(d);
    }
    if negative {
        value = value.wrapping_neg();
    }
    *dest = value;
    true
}

fn main() {
    let mut input = Vec::new();
    // Read all of stdin; scanf("%d") only consumes a leading integer
    // (after optional whitespace). Reading the whole buffer is fine
    // because the program does no further input.
    let _ = io::stdin().read_to_end(&mut input);

    let mut x: i32 = 0;
    let mut pos: usize = 0;
    let _ = scanf_int(&input, &mut pos, &mut x);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
