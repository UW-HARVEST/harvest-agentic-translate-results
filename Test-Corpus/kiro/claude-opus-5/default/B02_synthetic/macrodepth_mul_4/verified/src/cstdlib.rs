//! `atoi` with C semantics, for `mdmain.c`'s argument parsing.

use core::ffi::c_int;

/// `int atoi(const char *nptr)`.
///
/// glibc implements this as `(int) strtol(nptr, NULL, 10)`, so the C behaviour
/// this must reproduce is `strtol`'s, then a narrowing cast:
///
/// * leading whitespace (C locale `isspace`) is skipped;
/// * an optional `+`/`-` is consumed;
/// * decimal digits are accumulated until the first non-digit; a string with no
///   digits at all yields 0;
/// * `strtol` accumulates the *magnitude* in an `unsigned long` and clamps
///   against a sign-dependent cutoff: `LONG_MAX` (2^63 - 1) when positive,
///   `-(unsigned long)LONG_MIN` (2^63) when negative. Overflow therefore
///   saturates at `LONG_MAX` for positive input and at `LONG_MIN` for negative
///   input -- the negative limit reaches one further than the positive one, so
///   `"-9223372036854775808"` is exactly representable and is *not* an overflow;
/// * the `(int)` cast then keeps the low 32 bits.
///
/// The asymmetric cutoff is observable: `atoi("-9223372036854775808")` is
/// `(int)LONG_MIN == 0`, whereas clamping the magnitude at `LONG_MAX` first and
/// negating would give `(int)(-LONG_MAX) == 1`.
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

    // `strtol`'s cutoff, expressed as a magnitude.
    //   positive: LONG_MAX                 == 2^63 - 1
    //   negative: -(unsigned long)LONG_MIN == 2^63
    let cutoff: u64 = if negative {
        1u64 << 63
    } else {
        i64::MAX as u64
    };

    // Accumulate the magnitude the way `strtol` does, in unsigned arithmetic,
    // latching overflow instead of wrapping.
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
                _ => overflow = true,
            }
        }
        idx += 1;
    }

    let value: i64 = if overflow {
        // ERANGE in C; the returned value is the clamp, which atoi then casts.
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        // `magnitude` may be exactly 2^63, whose negation is `i64::MIN`; go
        // through `u64` so that case does not overflow.
        magnitude.wrapping_neg() as i64
    } else {
        magnitude as i64
    };

    // (int) of a long: keep the low 32 bits.
    value as c_int
}
