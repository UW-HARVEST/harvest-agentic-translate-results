//! Faithful re-implementations of the C library primitives the original
//! program relies on: `strtol(.., .., 10)` and `fgets`.

use std::io::BufRead;

/// `isspace()` for the C locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// `strtol(s, &end, 10)` over a NUL-terminated string whose contents are `s`.
///
/// Returns `(value, end_index)` where `end_index` is the offset of the byte
/// that `endptr` would point at. When no conversion is performed, C sets
/// `endptr == nptr`, i.e. index 0 -- this is what makes an *empty* argv entry
/// parse as the value `0` in the original program.
pub fn c_strtol(s: &[u8]) -> (i64, usize) {
    let mut i = 0usize;
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    let mut acc: u64 = 0;
    let mut overflow = false;
    // Magnitude limit: LONG_MAX, or LONG_MAX + 1 when negative.
    let limit: u64 = if neg {
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };
    while i < s.len() && s[i].is_ascii_digit() {
        let d = u64::from(s[i] - b'0');
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) if v <= limit => acc = v,
                _ => overflow = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits consumed: `endptr = nptr`, return 0.
        return (0, 0);
    }

    let val = if overflow {
        if neg {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if neg {
        (acc as i64).wrapping_neg()
    } else {
        acc as i64
    };
    (val, i)
}

/// `fgets(buf, size, stream)`.
///
/// Reads at most `size - 1` bytes, stopping after a newline (which is kept).
/// Returns `None` when nothing at all could be read (EOF/error), matching the
/// NULL return that terminates the C read loop.
pub fn c_fgets<R: BufRead>(r: &mut R, size: usize) -> Option<Vec<u8>> {
    let max_chars = size - 1;
    let mut out: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    while out.len() < max_chars {
        match r.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                out.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}
