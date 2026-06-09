// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust preserving byte-identical output behavior.

use std::ffi::c_char;
use std::ffi::c_int;
use std::io::{self, Read, Write};
use std::ptr;

/// Mimic the libc `printf("%s\n", line);` followed by a flush-on-newline.
/// We print to stdout and flush, since C's stdout is line-buffered for TTYs
/// and fully buffered for pipes; flushing on `\n` keeps behavior consistent.
fn print_c_string_with_newline(line: *const c_char) {
    if line.is_null() {
        return;
    }
    // Read C string up to NUL
    let mut len = 0usize;
    unsafe {
        while *line.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(line as *const u8, len);
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.write_all(slice);
        let _ = handle.write_all(b"\n");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        print_c_string_with_newline(line);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    // Mimic C: `char *data;` (uninitialized) then `printLine(data);`.
    // Reading uninitialized memory is undefined behavior in both languages.
    // To produce deterministic, byte-identical output across runs, we use
    // a null pointer here, which matches the common observed behavior of
    // the original C program on Linux (where the indeterminate pointer
    // happens to be NULL, causing `printLine` to silently return) and
    // avoids invoking Rust's stricter UB rules.
    let data: *const c_char = ptr::null();
    printLine(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    // `char *data; data = "string";`
    static STRING: &[u8] = b"string\0";
    let data: *const c_char = STRING.as_ptr() as *const c_char;
    printLine(data);
}

/// Read an integer from stdin in the same manner as `scanf("%d", &x)`.
/// Returns 0 if no integer could be parsed (matches the initial value of x).
fn scanf_int_from_stdin(initial: c_int) -> c_int {
    let mut buf = Vec::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    if handle.read_to_end(&mut buf).is_err() {
        return initial;
    }

    let mut i = 0usize;
    // Skip leading whitespace per scanf semantics
    while i < buf.len() && (buf[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    if i >= buf.len() {
        return initial;
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
    while i < buf.len() && (buf[i] as char).is_ascii_digit() {
        value = value.wrapping_mul(10).wrapping_add((buf[i] - b'0') as i64);
        i += 1;
    }
    if i == start {
        // No digits matched; scanf would not assign x, so x retains its initial value.
        return initial;
    }

    let result = sign.wrapping_mul(value);
    result as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> c_int {
    let mut x: c_int = 0;
    x = scanf_int_from_stdin(x);

    if x != 0 {
        good();
    } else {
        bad();
    }
    0
}

