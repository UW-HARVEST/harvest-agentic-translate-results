//! `atoi` with C semantics, for `mdmain.c`'s argument parsing.

use core::ffi::c_int;

/// `int atoi(const char *nptr)`.
///
/// glibc implements this as `(int) strtol(nptr, NULL, 10)`: leading whitespace is
/// skipped, an optional sign is consumed, digits are accumulated until the first
/// non-digit, and a string with no digits at all yields 0.
///
/// On out-of-range input `strtol` clamps to `LONG_MAX` **or `LONG_MIN`**, and the
/// clamp is applied to the *signed* result -- not to the magnitude. That
/// distinction is observable after the truncating cast to `int`:
/// `(int)LONG_MAX == -1` but `(int)LONG_MIN == 0`, whereas negating a clamped
/// magnitude would give `-LONG_MAX`, i.e. `(int)1`. The magnitude is therefore
/// accumulated in `u64` against a sign-dependent cutoff (`2^63` for negative,
/// `2^63 - 1` for positive) so that `"-9223372036854775808"` is represented
/// exactly and anything beyond it clamps to `LONG_MIN`.
///
/// Formally the overflow case is unspecified for `atoi`; matching glibc keeps the
/// observable behaviour aligned with the reference binary.
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

    // strtol's cutoff: the largest magnitude representable with this sign.
    let cutoff: u64 = if negative {
        1u64 << 63 // -(2^63) == LONG_MIN
    } else {
        i64::MAX as u64 // 2^63 - 1 == LONG_MAX
    };

    let mut magnitude: u64 = 0;
    let mut overflow = false;
    while idx < s.len() && s[idx].is_ascii_digit() {
        let digit = u64::from(s[idx] - b'0');
        if !overflow {
            match magnitude
                .checked_mul(10)
                .and_then(|m| m.checked_add(digit))
            {
                Some(m) if m <= cutoff => magnitude = m,
                // strtol keeps consuming digits after detecting overflow; only
                // the flag matters for the value.
                _ => overflow = true,
            }
        }
        idx += 1;
    }

    let value: i64 = if overflow {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        // `magnitude <= 2^63`; at exactly 2^63 the `as i64` is already LONG_MIN
        // and `wrapping_neg` leaves it there, which is the value strtol returns.
        (magnitude as i64).wrapping_neg()
    } else {
        magnitude as i64
    };

    // (int) of a long: truncate the low 32 bits.
    value as c_int
}
