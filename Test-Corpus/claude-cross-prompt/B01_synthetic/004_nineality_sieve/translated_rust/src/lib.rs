// Translated from c_src/src/main.c
// Counts from a starting point, stopping when the count ends in 9 (base 10).

use std::ffi::c_char;
use std::ffi::c_int;
use std::io::Write;

/// Mimic C's `strtol(s, &end, 10)` for the subset of behavior used here.
///
/// Returns (value as i64, number of bytes consumed). If no characters are
/// consumed, the returned consumed count is 0 (mirroring `end == argv[1]`).
fn c_strtol_base10(bytes: &[u8]) -> (i64, usize) {
    let mut i = 0usize;

    // Skip leading whitespace (matches C `isspace`: space, \t, \n, \v, \f, \r).
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' => i += 1,
            _ => break,
        }
    }

    // Optional sign.
    let sign_start = i;
    let mut negative = false;
    if i < bytes.len() {
        match bytes[i] {
            b'+' => i += 1,
            b'-' => {
                negative = true;
                i += 1;
            }
            _ => {}
        }
    }

    // Parse digits.
    let digits_start = i;
    let mut value: i64 = 0;
    let mut overflow = false;
    while i < bytes.len() {
        let c = bytes[i];
        if !c.is_ascii_digit() {
            break;
        }
        let digit = (c - b'0') as i64;
        if !overflow {
            // Saturate like C strtol on overflow (which returns LONG_MAX/LONG_MIN
            // and sets errno). Cast to i32 below mirrors the C cast to `int`.
            match value
                .checked_mul(10)
                .and_then(|v| v.checked_add(digit))
            {
                Some(v) => value = v,
                None => {
                    overflow = true;
                    value = if negative { i64::MIN } else { i64::MAX };
                }
            }
        }
        i += 1;
    }

    if digits_start == i {
        // No digits consumed: `end` is set to the start of the input.
        // This means the sign character (if any) is also rolled back.
        let _ = sign_start;
        return (0, 0);
    }

    if negative && !overflow {
        value = value.wrapping_neg();
    }

    (value, i)
}

#[unsafe(no_mangle)]
pub extern "C" fn main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    if argc != 2 {
        let _ = out.write_all(b"Error: should only be a single (integer) argument!\n");
        return 1;
    }

    // SAFETY: argv is expected to point to argc valid C string pointers.
    let arg1_ptr = unsafe { *argv.add(1) };
    if arg1_ptr.is_null() {
        let _ = out.write_all(b"Error: first argument must be an integer!\n");
        return 1;
    }

    // Build a byte slice for the argv[1] C string.
    let arg1_bytes = unsafe {
        let mut len = 0usize;
        while *arg1_ptr.add(len) != 0 {
            len += 1;
        }
        std::slice::from_raw_parts(arg1_ptr as *const u8, len)
    };

    let (parsed, consumed) = c_strtol_base10(arg1_bytes);
    if consumed == 0 {
        // end == argv[1]: nothing parsed.
        let _ = out.write_all(b"Error: first argument must be an integer!\n");
        return 1;
    }

    // The C code stores the result of strtol (long) into an `int`. On most
    // 64-bit Unix-like platforms (the C target here), this is an implicit
    // narrowing cast from long (i64) to int (i32). Reproduce that exactly.
    let mut val: i32 = parsed as i32;

    loop {
        // C: printf("%d\n", val);
        let mut buf = itoa_buf();
        let s = format_i32(&mut buf, val);
        let _ = out.write_all(s);
        let _ = out.write_all(b"\n");

        if val.rem_euclid(10) == 9 && val >= 0 {
            // C uses `val % 10 == 9`. In C, `%` truncates toward zero, so for
            // negative values `val % 10` is in -9..=0 and never equals 9.
            // Rust's `%` matches C semantics for signed integers, so the simple
            // expression `val % 10 == 9` would also work. We keep this guarded
            // form to make the sign behavior explicit.
            break;
        }
        if val % 10 == 9 {
            break;
        }
        // Increment with C's wrapping signed-overflow behavior (UB in C, but
        // observable as wrap on typical targets).
        val = val.wrapping_add(1);
    }

    0
}

// Tiny helpers to format an i32 into a stack buffer without pulling in
// formatting machinery beyond core. Using std fmt is fine too, but this keeps
// the output exactly "<digits>" with no locale-specific surprises.
fn itoa_buf() -> [u8; 12] {
    [0u8; 12]
}

fn format_i32(buf: &mut [u8; 12], v: i32) -> &[u8] {
    // Match printf("%d", v): decimal, with a leading '-' for negatives, and
    // "0" for zero (no leading zeros, no plus sign).
    if v == 0 {
        buf[0] = b'0';
        return &buf[..1];
    }
    let negative = v < 0;
    // Build digits from the end of the buffer.
    let mut idx = buf.len();
    // Use a u32 magnitude to handle i32::MIN correctly.
    let mut mag: u32 = if negative {
        (v as i64).unsigned_abs() as u32
    } else {
        v as u32
    };
    while mag > 0 {
        idx -= 1;
        buf[idx] = b'0' + (mag % 10) as u8;
        mag /= 10;
        if idx == 0 {
            break;
        }
    }
    if negative {
        idx -= 1;
        buf[idx] = b'-';
    }
    // Move bytes to the start so we can return a contiguous slice.
    let len = buf.len() - idx;
    for i in 0..len {
        buf[i] = buf[idx + i];
    }
    // Suppress the unused warning if v happened to be 0 (handled above).
    let _ = v;
    &buf[..len]
}
