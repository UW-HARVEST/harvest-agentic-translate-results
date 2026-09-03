//! `atoi` with C semantics, for `mdmain.c`'s argument parsing.

use core::ffi::c_int;

/// `int atoi(const char *nptr)`.
///
/// glibc implements this as `(int) strtol(nptr, NULL, 10)`: leading whitespace is
/// skipped, an optional sign is consumed, digits are accumulated until the first
/// non-digit, and a string with no digits at all yields 0.
///
/// On overflow `strtol` clamps to `LONG_MAX` or `LONG_MIN` and the cast to `int`
/// then keeps the low 32 bits. The clamp is *sign-specific* and asymmetric:
/// `strtol` compares the accumulated magnitude against `LONG_MAX` for a positive
/// number but against `-(unsigned long)LONG_MIN` (= 2^63) for a negative one, so
/// `atoi("-99999999999999999999")` is `(int)LONG_MIN` == 0, not
/// `(int)(-LONG_MAX)` == 1. Clamping the magnitude in `i64` and negating loses
/// that extra step on the negative side, so the magnitude is accumulated in
/// `u64` against a sign-dependent limit instead. Formally the overflow case is
/// unspecified for `atoi`; matching glibc keeps the observable behaviour aligned
/// with the reference binary.
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

    // Accumulate the magnitude the way strtol does: against a limit that depends
    // on the sign, because the negative range reaches one further out.
    // LONG_MAX == 2^63 - 1, -(unsigned long)LONG_MIN == 2^63.
    let limit: u64 = if negative { 1u64 << 63 } else { i64::MAX as u64 };

    let mut magnitude: u64 = 0;
    let mut out_of_range = false;
    while idx < s.len() && s[idx].is_ascii_digit() {
        let digit = u64::from(s[idx] - b'0');
        if !out_of_range {
            match magnitude.checked_mul(10).and_then(|m| m.checked_add(digit)) {
                Some(m) if m <= limit => magnitude = m,
                // Either the u64 accumulator wrapped (necessarily past `limit`,
                // which is at most 2^63) or the value passed `limit`; strtol
                // keeps scanning the remaining digits either way.
                _ => out_of_range = true,
            }
        }
        idx += 1;
    }

    let value: i64 = if out_of_range {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        // wrapping_neg in u64 then reinterpret: magnitude == 2^63 maps to
        // LONG_MIN exactly, which a plain `-(magnitude as i64)` could not express.
        magnitude.wrapping_neg() as i64
    } else {
        magnitude as i64
    };

    // (int) of a long: truncate the low 32 bits.
    value as c_int
}
