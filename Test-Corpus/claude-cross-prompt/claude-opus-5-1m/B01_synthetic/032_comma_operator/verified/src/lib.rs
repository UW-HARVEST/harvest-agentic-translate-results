use std::ffi::c_int;
use std::io::{self, Read, Write};

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut i: c_int = 0;
    let mut j: c_int = 0;
    while i < x {
        // Match printf("%d %d\n", i, j);
        let _ = write!(out, "{} {}\n", i, j);
        i += 1;
        j += 2;
    }
    let _ = out.flush();
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> c_int {
    let mut x: c_int = 0;

    // Mimic scanf("%d", &x): read whitespace-separated signed decimal integer
    // from stdin, parsing as much as possible.
    let mut buf = String::new();
    let _ = io::stdin().read_to_string(&mut buf);

    let bytes = buf.as_bytes();
    let mut idx = 0;
    // Skip leading whitespace (space, tab, newline, etc., as scanf does).
    while idx < bytes.len() && (bytes[idx] as char).is_ascii_whitespace() {
        idx += 1;
    }
    let mut sign: i64 = 1;
    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        if bytes[idx] == b'-' {
            sign = -1;
        }
        idx += 1;
    }
    let mut got_digit = false;
    let mut value: i64 = 0;
    while idx < bytes.len() && (bytes[idx] as char).is_ascii_digit() {
        got_digit = true;
        value = value.wrapping_mul(10).wrapping_add((bytes[idx] - b'0') as i64);
        idx += 1;
    }
    if got_digit {
        let signed = value.wrapping_mul(sign);
        x = signed as c_int;
    }

    driver(x);
    0
}
