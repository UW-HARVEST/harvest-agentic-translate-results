// Library translation of c_src/src/main.c to Rust.
//
// This crate exposes C-ABI symbols matching the original C program so that
// integration tests can load both the C .so and the Rust .so via libloading
// and compare behavior byte-for-byte.

use std::ffi::CStr;
use std::io::{self, Write};
#[cfg(not(test))]
use std::io::Read;
use std::os::raw::c_char;
#[cfg(not(test))]
use std::os::raw::c_int;

/// C: `void printLine(const char *line)`
/// Prints `line` followed by a newline if `line` is non-null.
#[no_mangle]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // Replicate printf("%s\n", line) — read until NUL, then write a '\n'.
        let cs = CStr::from_ptr(line);
        let bytes = cs.to_bytes();
        let stdout = io::stdout();
        let mut out = stdout.lock();
        let _ = out.write_all(bytes);
        let _ = out.write_all(b"\n");
        let _ = out.flush();
    }
}

/// C: `void bad()`
/// In the original C, `data` is uninitialized and passed to `printLine`.
/// On the platforms this program targets, the stack slot reads as NULL,
/// so nothing is printed. Replicate that observed behavior by passing NULL.
#[no_mangle]
pub unsafe extern "C" fn bad() {
    let data: *const c_char = std::ptr::null();
    printLine(data);
}

/// C: `void good()` — sets `data = "string"` and calls `printLine(data)`.
#[no_mangle]
pub unsafe extern "C" fn good() {
    // Static, NUL-terminated string literal.
    let data: *const c_char = b"string\0".as_ptr() as *const c_char;
    printLine(data);
}

/// Mimic C's `scanf("%d", &x)` for a single decimal integer:
#[cfg(not(test))]
/// - skip leading ASCII whitespace
/// - optional '+' / '-' sign
/// - read consecutive ASCII digits
/// On any failure, returns 0 (matching the C program where x is initialized to 0).
fn scanf_int(input: &[u8]) -> i32 {
    let mut i = 0usize;
    while i < input.len() && (input[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    if i >= input.len() {
        return 0;
    }
    let mut negative = false;
    if input[i] == b'-' {
        negative = true;
        i += 1;
    } else if input[i] == b'+' {
        i += 1;
    }
    let start = i;
    let mut value: i64 = 0;
    while i < input.len() && (input[i] as char).is_ascii_digit() {
        value = value
            .wrapping_mul(10)
            .wrapping_add((input[i] - b'0') as i64);
        i += 1;
    }
    if i == start {
        return 0;
    }
    let result = if negative { -value } else { value };
    result as i32
}

/// C: `int main()`
/// Reads an int from stdin and dispatches to `good()` or `bad()`.
///
/// Gated out of `cargo test` because the test harness defines its own `main`
/// entry point, which would otherwise conflict with this `#[no_mangle]` export.
/// The release/debug cdylib still exports `main`.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn main() -> c_int {
    let mut x: i32 = 0;

    let mut buf = Vec::new();
    if io::stdin().read_to_end(&mut buf).is_ok() {
        x = scanf_int(&buf);
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
    0
}
