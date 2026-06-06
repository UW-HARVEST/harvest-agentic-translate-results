// Use libc snprintf to get byte-identical output to C's printf "%.{precision}g".
// This is the simplest and safest way to match the C reference formatting exactly.

use std::os::raw::{c_char, c_int};

extern "C" {
    fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
}

pub fn format_g(value: f64, precision: usize) -> String {
    // Build a NUL-terminated format string like "%.9g".
    let mut fmt_bytes: Vec<u8> = Vec::new();
    fmt_bytes.extend_from_slice(b"%.");
    fmt_bytes.extend_from_slice(precision.to_string().as_bytes());
    fmt_bytes.push(b'g');
    fmt_bytes.push(0);

    // First call: query how much we need (or write into 64-byte buffer).
    let mut buf: Vec<u8> = vec![0u8; 64];
    let needed: c_int = unsafe {
        snprintf(
            buf.as_mut_ptr() as *mut c_char,
            buf.len(),
            fmt_bytes.as_ptr() as *const c_char,
            value,
        )
    };

    if needed < 0 {
        return String::new();
    }
    let needed = needed as usize;
    if needed >= buf.len() {
        // Reallocate and try again.
        buf.resize(needed + 1, 0);
        let _ = unsafe {
            snprintf(
                buf.as_mut_ptr() as *mut c_char,
                buf.len(),
                fmt_bytes.as_ptr() as *const c_char,
                value,
            )
        };
    }

    // Truncate at NUL.
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..len]).into_owned()
}
