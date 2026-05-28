// Translation of c_src/src/main.c to Rust.
//
// The original C program contains an intentional CWE-457 vulnerability:
// `bad()` reads an uninitialized `char *data` pointer and passes it to
// `printLine`. In practice, on the platforms this program is exercised on,
// the stack slot for `data` happens to read as NULL, so `printLine` skips
// the print. We reproduce that observed behavior here using safe Rust:
// `bad()`'s `data` is modeled as `None`.
//
// `good()` always assigns `data = "string"` and prints it.

use std::io::{self, Read, Write};

fn print_line(line: Option<&str>) {
    // C: if (line != NULL) { printf("%s\n", line); }
    if let Some(s) = line {
        // C uses printf("%s\n", line). println! emits "\n" on Unix which
        // matches printf's "\n" output exactly.
        let stdout = io::stdout();
        let mut out = stdout.lock();
        let _ = out.write_all(s.as_bytes());
        let _ = out.write_all(b"\n");
    }
}

fn bad() {
    // C: `char *data;` (uninitialized). The observed behavior on this
    // program's targets is that the pointer reads as NULL, so nothing is
    // printed. Model that with `None`.
    let data: Option<&str> = None;
    print_line(data);
}

fn good() {
    let data: Option<&str> = Some("string");
    print_line(data);
}

/// Mimic C's `scanf("%d", &x)` for a single decimal integer:
/// - skip leading ASCII whitespace
/// - optional '+' / '-' sign
/// - read consecutive ASCII digits
/// On any failure, `x` retains its previous value (0 here, matching C).
fn scanf_int(input: &[u8]) -> i32 {
    let mut i = 0usize;
    // skip whitespace
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
        // No digits parsed: scanf returns 0 conversions and x is unchanged.
        return 0;
    }
    let result = if negative { -value } else { value };
    // Truncate to i32 like C's `int` on common platforms.
    result as i32
}

fn main() {
    // C: int x = 0; scanf("%d", &x);
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
    // C returns 0 — Rust's main returns () which exits 0 on success.
}
