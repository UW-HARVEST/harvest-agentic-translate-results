// Translation of c_src/src/driver.c (and a wrapper main) to Rust.
//
// The original C only exposes:
//     void driver(const char *s1, const char *s2);
// which prints strcspn(s1, s2) followed by '\n'. There is no main() in C,
// so this Rust program provides a minimal main that reads two lines from
// stdin (fgets-style: keeps the trailing newline if present, then we
// strip the final '\n' so the C-style C-string equivalent matches the
// "logical" line content), and invokes driver().

use std::io::{self, Read, Write};

/// Reproduce C's `strcspn(s1, s2)`:
/// Returns the length of the longest initial prefix of `s1` that contains
/// none of the bytes that appear in `s2`.
///
/// Operates on raw bytes (matching C semantics on `char *`).
fn strcspn(s1: &[u8], s2: &[u8]) -> usize {
    // Build a 256-entry byte set for O(1) lookup, equivalent to scanning
    // s2 for each candidate byte in s1.
    let mut set = [false; 256];
    for &b in s2 {
        set[b as usize] = true;
    }
    let mut count: usize = 0;
    for &b in s1 {
        if set[b as usize] {
            break;
        }
        count += 1;
    }
    count
}

/// Direct translation of `void driver(const char *s1, const char *s2)`.
fn driver(s1: &[u8], s2: &[u8]) {
    // C: printf("%zu\n", strcspn(s1, s2));
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", strcspn(s1, s2));
}

/// Read one line from `data` starting at `pos`, fgets-style (consume
/// through and including a '\n' if present, else through EOF). Returns
/// the line bytes WITHOUT the trailing newline (so they form a clean
/// C-style string when null-terminated logically).
fn read_line<'a>(data: &'a [u8], pos: &mut usize) -> &'a [u8] {
    let start = *pos;
    let mut end = start;
    while end < data.len() && data[end] != b'\n' {
        end += 1;
    }
    let line = &data[start..end];
    // Advance past the newline if present.
    *pos = if end < data.len() { end + 1 } else { end };
    line
}

fn main() {
    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() {
        return;
    }
    let mut pos = 0usize;
    let s1 = read_line(&input, &mut pos);
    let s2 = read_line(&input, &mut pos);
    driver(s1, s2);
}
