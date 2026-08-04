// FFI exports mirroring the C library symbols.
//
// The C source defines:
//   void driver(int x, int y);  -- prints (x | ~y) followed by a newline
//   int  main(void);            -- reads two ints from stdin then calls driver
//
// We expose the same `driver` symbol and a `main` symbol so that `nm -D` on the
// Rust .so matches the C .so. The `main` here is exposed as a `no_mangle`
// function and reads from stdin in the same way (for parity), although only
// `driver` is exercised through the FFI tests.

use std::io::{self, Write};
#[cfg(not(test))]
use std::io::Read as _;

/// Mimics C's scanf("%d", ...).
#[cfg_attr(test, allow(dead_code))]
fn scanf_i32(bytes: &[u8], pos: &mut usize) -> Option<i32> {
    while *pos < bytes.len() {
        let c = bytes[*pos];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == b'\x0b' || c == b'\x0c' {
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos >= bytes.len() {
        return None;
    }

    let start = *pos;
    let mut negative = false;

    if bytes[*pos] == b'+' {
        *pos += 1;
    } else if bytes[*pos] == b'-' {
        negative = true;
        *pos += 1;
    }

    let digits_start = *pos;
    while *pos < bytes.len() && bytes[*pos].is_ascii_digit() {
        *pos += 1;
    }

    if *pos == digits_start {
        *pos = start;
        return None;
    }

    let mut value: i32 = 0;
    for &d in &bytes[digits_start..*pos] {
        let digit = (d - b'0') as i32;
        value = value.wrapping_mul(10);
        if negative {
            value = value.wrapping_sub(digit);
        } else {
            value = value.wrapping_add(digit);
        }
    }

    Some(value)
}

/// Pure Rust translation of the C `driver` function: prints `(x | ~y)`
/// followed by a newline to stdout.
fn driver_inner(x: i32, y: i32, out: &mut impl Write) -> io::Result<()> {
    let result = x | !y;
    write!(out, "{}", result)?;
    writeln!(out)?;
    Ok(())
}

/// FFI export matching `void driver(int x, int y)` in the C source.
#[no_mangle]
pub extern "C" fn driver(x: std::os::raw::c_int, y: std::os::raw::c_int) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = driver_inner(x as i32, y as i32, &mut handle);
    let _ = handle.flush();
}

/// FFI export matching `int main(void)` in the C source.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> std::os::raw::c_int {
    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() {
        return 0;
    }

    let mut pos = 0usize;
    let mut x: i32 = 0;
    let mut y: i32 = 0;

    if let Some(v) = scanf_i32(&input, &mut pos) {
        x = v;
    }
    if let Some(v) = scanf_i32(&input, &mut pos) {
        y = v;
    }

    driver(x, y);
    0
}
