// cstdio.rs
//
// Small helpers that reproduce the exact behaviour of the C standard library
// functions used by the original program: printf-style float formatting,
// fgets() line reading and sscanf(buf, "%d", &x).

use std::io::BufRead;

/// Write formatted text to the output stream, ignoring I/O errors the way
/// printf() does (the C program never checks printf's return value).
macro_rules! p {
    ($out:expr, $($arg:tt)*) => {{
        let _ = ::std::io::Write::write_fmt($out, format_args!($($arg)*));
    }};
}

/// Write raw bytes (used for `%s` of a fixed-size C char buffer).
macro_rules! pb {
    ($out:expr, $bytes:expr) => {{
        let _ = ::std::io::Write::write_all($out, $bytes);
    }};
}

/// The bytes of a NUL-terminated C string held in a fixed-size buffer.
pub fn cstr(buf: &[u8]) -> &[u8] {
    match buf.iter().position(|&b| b == 0) {
        Some(end) => &buf[..end],
        None => buf,
    }
}

/// strncpy(dst, src, dst.len() - 1) followed by dst[dst.len() - 1] = '\0',
/// i.e. exactly what create_item()/create_order() do.
pub fn strncpy_truncate(dst: &mut [u8], src: &[u8]) {
    for b in dst.iter_mut() {
        *b = 0;
    }
    let limit = dst.len() - 1;
    for (i, &b) in src.iter().take(limit).enumerate() {
        if b == 0 {
            break;
        }
        dst[i] = b;
    }
}

/// C's "%.*f" conversion, including glibc's spellings of the specials.
pub fn fmt_f(value: f64, precision: usize) -> String {
    if value.is_nan() {
        return if value.is_sign_negative() {
            String::from("-nan")
        } else {
            String::from("nan")
        };
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            String::from("-inf")
        } else {
            String::from("inf")
        };
    }
    format!("{:.*}", precision, value)
}

/// isspace() in the "C" locale.
fn is_c_space(b: u8) -> bool {
    b == b' ' || b == b'\t' || b == b'\n' || b == 0x0b || b == 0x0c || b == b'\r'
}

/// fgets(buf, size, stdin).
///
/// Reads at most `size - 1` bytes, stopping after a newline (which is kept in
/// the returned buffer).  Returns None at end of file with nothing read, just
/// as fgets() returns NULL.
pub fn fgets<R: BufRead + ?Sized>(reader: &mut R, size: usize) -> Option<Vec<u8>> {
    let max = size - 1;
    let mut out: Vec<u8> = Vec::new();

    while out.len() < max {
        let (chunk, newline_at) = {
            let available = loop {
                match reader.fill_buf() {
                    Ok(buf) => break buf,
                    Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => return None,
                }
            };
            if available.is_empty() {
                break; // EOF
            }
            let room = max - out.len();
            let take = if available.len() < room {
                available.len()
            } else {
                room
            };
            let slice = &available[..take];
            (slice.to_vec(), slice.iter().position(|&b| b == b'\n'))
        };

        match newline_at {
            Some(i) => {
                out.extend_from_slice(&chunk[..=i]);
                reader.consume(i + 1);
                break;
            }
            None => {
                let n = chunk.len();
                out.extend_from_slice(&chunk);
                reader.consume(n);
            }
        }
    }

    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// sscanf(input, "%d", &value): returns None when the conversion fails (the C
/// code then reports "Invalid input").
///
/// Mirrors glibc: leading whitespace is skipped, an optional sign is accepted,
/// decimal digits are accumulated into a `long` that saturates at
/// LONG_MAX/LONG_MIN on overflow, and the result is stored into an `int`
/// (truncating the upper bits).
pub fn sscanf_int(input: &[u8]) -> Option<i32> {
    let s = cstr(input);
    let mut i = 0usize;

    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    let mut acc: i64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = i64::from(s[i] - b'0');
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        return None; // matching failure
    }

    let as_long: i64 = if overflow {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -acc
    } else {
        acc
    };

    Some(as_long as i32)
}
