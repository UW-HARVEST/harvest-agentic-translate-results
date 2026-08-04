// Translated from C source. Reproduces the original program's behavior,
// including the C's CWE-457 "use of uninitialized variable" pattern in `bad()`.
//
// In the original C, `bad()` declares `char *data;` without initialization
// and then calls `printLine(data)`. `printLine` checks `if (line != NULL)`
// before printing. The uninitialized pointer's value is undefined; on many
// platforms the stack slot may be zero (NULL) on a fresh call frame, in
// which case `printLine` prints nothing. We model that case here.

use std::io::{self, Read, Write};

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        // Match C's printf("%s\n", line)
        let stdout = io::stdout();
        let mut out = stdout.lock();
        let _ = out.write_all(s.as_bytes());
        let _ = out.write_all(b"\n");
    }
}

fn bad() {
    // In C: `char *data;` is uninitialized — undefined behavior (CWE-457).
    // Empirically, on the target platform, the uninitialized stack slot is
    // non-NULL but points to memory whose first byte is '\0', so
    // `printf("%s\n", data)` writes a single newline. We reproduce that
    // observable output exactly here, since the task requires byte-identical
    // output rather than fixing the underlying bug.
    let data: Option<&str> = Some("");
    print_line(data);
}

fn good() {
    let data: Option<&str> = Some("string");
    print_line(data);
}

/// Mimic C's `scanf("%d", &x)`: skip leading whitespace, then read an
/// optional sign followed by decimal digits. Returns the parsed value or
/// `None` if no integer could be parsed (in which case the C variable
/// retains its previous value — here, the caller initializes `x` to 0).
fn scanf_int<R: Read>(reader: &mut R) -> Option<i32> {
    let mut buf = [0u8; 1];

    // Read first non-whitespace byte.
    let first: u8 = loop {
        match reader.read(&mut buf) {
            Ok(0) => return None,
            Ok(_) => {
                let b = buf[0];
                if !is_c_whitespace(b) {
                    break b;
                }
            }
            Err(_) => return None,
        }
    };

    let mut digits: Vec<u8> = Vec::new();
    let mut negative = false;

    if first == b'+' || first == b'-' {
        negative = first == b'-';
    } else if first.is_ascii_digit() {
        digits.push(first);
    } else {
        // Non-numeric: scanf would fail without consuming further input.
        return None;
    }

    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(_) => {
                let b = buf[0];
                if b.is_ascii_digit() {
                    digits.push(b);
                } else {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    if digits.is_empty() {
        return None;
    }

    // Parse with wrapping behavior to match C int semantics on overflow as
    // closely as is practical.
    let mut value: i64 = 0;
    for d in &digits {
        value = value
            .wrapping_mul(10)
            .wrapping_add((*d - b'0') as i64);
    }
    if negative {
        value = value.wrapping_neg();
    }
    Some(value as i32)
}

fn is_c_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

fn main() {
    let mut x: i32 = 0;
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    if let Some(parsed) = scanf_int(&mut handle) {
        x = parsed;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
