// Library crate exposing the same FFI surface as the C shared library.
//
// The C source defines two externally-visible symbols:
//   - `driver(int x)`: prints `2*x + 300` followed by a newline.
//   - `main()`: reads an int via scanf and calls driver.
//
// To remain symbol-compatible with the C .so, we re-export both with
// `#[no_mangle]`.

use std::io::{self, Read, Write};

/// Replicates `void driver(int x)` from the C source:
///     int y = 2 * x;
///     y += 300;
///     printf("%d\n", y);
#[no_mangle]
pub extern "C" fn driver(x: i32) {
    // Use wrapping arithmetic to match C's int overflow behavior.
    let mut y: i32 = x.wrapping_mul(2);
    y = y.wrapping_add(300);
    // Use libc-style printf via Rust's stdout to keep the output byte-identical
    // (`%d\n` -> "<value>\n").
    let s = format!("{}\n", y);
    let _ = io::stdout().write_all(s.as_bytes());
    let _ = io::stdout().flush();
}

/// Mimic scanf("%d", &x):
/// - Skip leading whitespace (including newlines).
/// - Optionally read a leading '+' or '-'.
/// - Read decimal digits until a non-digit byte or EOF.
/// - On match failure (no digits), the variable is unchanged.
/// Returns Some(value) on success, None on match failure.
#[cfg(not(test))]
fn scanf_int(stdin_bytes: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip whitespace
    while *pos < stdin_bytes.len() {
        let c = stdin_bytes[*pos];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0B || c == 0x0C {
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos >= stdin_bytes.len() {
        return None;
    }

    let start = *pos;
    let mut sign: i64 = 1;
    if stdin_bytes[*pos] == b'+' {
        *pos += 1;
    } else if stdin_bytes[*pos] == b'-' {
        sign = -1;
        *pos += 1;
    }

    let digit_start = *pos;
    let mut value: i64 = 0;
    while *pos < stdin_bytes.len() {
        let c = stdin_bytes[*pos];
        if c.is_ascii_digit() {
            value = value.wrapping_mul(10).wrapping_add((c - b'0') as i64);
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos == digit_start {
        // No digits matched; rewind to the start so caller knows nothing was consumed.
        *pos = start;
        return None;
    }

    let signed = sign.wrapping_mul(value);
    Some(signed as i32)
}

/// Replicates the C `main()`: reads an int from stdin and calls `driver`.
/// Exported with the C `main` symbol name to mirror the C shared library.
///
/// Hidden from `cfg(test)` builds because the integration test harness
/// generates its own `main` and the linker rejects two entry symbols.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> i32 {
    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() {
        // If reading fails, fall back to default behavior
    }

    let mut pos: usize = 0;
    let x: i32 = scanf_int(&input, &mut pos).unwrap_or(0);
    driver(x);

    let _ = io::stdout().flush();
    0
}
