use std::ffi::c_int;
use std::io::{self, Read, Write};

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut y: c_int = x.wrapping_mul(2);
    y = y.wrapping_add(300);
    // Match printf("%d\n", y) — write decimal followed by newline to stdout.
    let s = format!("{}\n", y);
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(s.as_bytes());
    let _ = handle.flush();
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> c_int {
    let mut x: c_int = 0;
    // Mimic scanf("%d", &x): read whitespace-separated decimal integer from stdin.
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_ok() {
        // Find first integer token
        let trimmed = input.trim_start();
        let bytes = trimmed.as_bytes();
        let mut i = 0usize;
        // optional sign
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let end = i;
        if end > 0 {
            if let Ok(parsed) = trimmed[..end].parse::<c_int>() {
                x = parsed;
            }
        }
    }
    driver(x);
    0
}
