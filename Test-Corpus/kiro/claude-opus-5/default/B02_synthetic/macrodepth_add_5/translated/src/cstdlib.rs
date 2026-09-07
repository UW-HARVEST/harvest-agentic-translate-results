//! `atoi` with C semantics, for `mdmain.c`'s argument parsing.

use core::ffi::c_int;

/// `int atoi(const char *nptr)`.
///
/// glibc implements this as `(int) strtol(nptr, NULL, 10)`: leading whitespace is
/// skipped, an optional sign is consumed, digits are accumulated until the first
/// non-digit, and a string with no digits at all yields 0. Out-of-range input
/// saturates at `LONG_MAX`/`LONG_MIN` inside `strtol` and is then truncated by the
/// cast to `int`, which is what the `i64` saturation plus `as c_int` below
/// reproduce. Formally the overflow case is unspecified for `atoi`; matching
/// glibc keeps the observable behaviour aligned with the reference binary.
pub fn atoi(s: &[u8]) -> c_int {
    let mut idx = 0usize;

    // isspace() in the C locale.
    while idx < s.len()
        && matches!(
            s[idx],
            b' ' | b'\t' | b'\n' | 0x0b /* \v */ | 0x0c /* \f */ | b'\r'
        )
    {
        idx += 1;
    }

    let mut negative = false;
    if idx < s.len() && (s[idx] == b'+' || s[idx] == b'-') {
        negative = s[idx] == b'-';
        idx += 1;
    }

    // Accumulate the magnitude, saturating the way strtol clamps to LONG_MAX.
    let mut magnitude: i64 = 0;
    while idx < s.len() && s[idx].is_ascii_digit() {
        let digit = i64::from(s[idx] - b'0');
        magnitude = magnitude.saturating_mul(10).saturating_add(digit);
        idx += 1;
    }

    let value: i64 = if negative {
        // LONG_MIN is one further out than -LONG_MAX; saturating_neg keeps the
        // clamp on the negative side too.
        magnitude.saturating_neg()
    } else {
        magnitude
    };

    // (int) of a long: truncate the low 32 bits.
    value as c_int
}
