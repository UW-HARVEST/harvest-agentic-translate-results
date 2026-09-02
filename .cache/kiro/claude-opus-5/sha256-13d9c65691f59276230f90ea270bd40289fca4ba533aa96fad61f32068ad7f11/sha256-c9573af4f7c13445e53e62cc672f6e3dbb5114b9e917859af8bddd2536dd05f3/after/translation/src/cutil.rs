//! Small helpers that reproduce the exact semantics of the C standard library
//! functions used by the original sources (glibc / x86-64 Linux).

use std::ffi::{c_char, c_int, OsString};

/// Bytes of a NUL-terminated C string.
///
/// `printf`/`fprintf` with `%s` in glibc prints the literal text `(null)` when
/// handed a NULL pointer, so mirror that instead of crashing.
///
/// # Safety
/// `s` must be NULL or a valid pointer to a NUL-terminated string.
pub unsafe fn c_str_bytes(s: *const c_char) -> Vec<u8> {
    if s.is_null() {
        return b"(null)".to_vec();
    }
    let mut out = Vec::new();
    let mut p = s as *const u8;
    while *p != 0 {
        out.push(*p);
        p = p.add(1);
    }
    out
}

/// Raw bytes of an environment variable, as `getenv()` would see them.
pub fn getenv_bytes(name: &str) -> Option<Vec<u8>> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        std::env::var_os(name).map(|v: OsString| v.into_vec())
    }
    #[cfg(not(unix))]
    {
        std::env::var_os(name).map(|v: OsString| v.to_string_lossy().into_owned().into_bytes())
    }
}

/// glibc `atoi`, which is defined as `(int)strtol(s, NULL, 10)`.
///
/// That means: leading whitespace is skipped, an optional sign is accepted,
/// digits are accumulated, out-of-range values saturate to `LONG_MAX` /
/// `LONG_MIN` and the resulting `long` is then *truncated* to `int`
/// (so e.g. "99999999999999999999" yields -1 and "4294967296" yields 0).
pub fn c_atoi(bytes: &[u8]) -> c_int {
    let mut i = 0usize;

    // isspace(): ' ', '\t', '\n', '\v', '\f', '\r'
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }

    let mut negative = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        negative = bytes[i] == b'-';
        i += 1;
    }

    let mut magnitude: u64 = 0;
    let mut overflow = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let digit = (bytes[i] - b'0') as u64;
        if !overflow {
            match magnitude.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => magnitude = v,
                None => overflow = true,
            }
        }
        i += 1;
    }

    // Saturate into `long` exactly like strtol does.
    let as_long: i64 = if negative {
        if overflow || magnitude > (i64::MAX as u64) + 1 {
            i64::MIN
        } else if magnitude == (i64::MAX as u64) + 1 {
            i64::MIN
        } else {
            -(magnitude as i64)
        }
    } else if overflow || magnitude > i64::MAX as u64 {
        i64::MAX
    } else {
        magnitude as i64
    };

    // Implicit long -> int conversion performed by atoi().
    as_long as c_int
}

/// `strncpy(dst, src, n)` followed by nothing else: copies at most `n` bytes,
/// stopping after a NUL, and zero-pads the remainder of the `n` bytes.
///
/// # Safety
/// `dst` must be writable for `n` bytes; `src` must be a valid NUL-terminated
/// string (a NULL `src` faults, exactly as the C original does).
pub unsafe fn strncpy(dst: *mut u8, src: *const c_char, n: usize) {
    let src = src as *const u8;
    let mut i = 0usize;
    while i < n {
        let byte = *src.add(i);
        *dst.add(i) = byte;
        if byte == 0 {
            break;
        }
        i += 1;
    }
    // strncpy pads the rest of the n bytes with NUL.
    while i < n {
        *dst.add(i) = 0;
        i += 1;
    }
}
