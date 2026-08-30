//! Faithful re-implementation of C's `strtoul(nptr, &endptr, 10)` as provided
//! by glibc, including the quirks the original program depends on:
//!
//! * leading whitespace is skipped,
//! * an optional `+` / `-` sign is accepted, and a negated value wraps modulo
//!   2^64 (so `"-1"` yields `ULONG_MAX`),
//! * when no digits are present at all, no conversion happens: the result is 0
//!   and `endptr` is left pointing at the *start* of the input (which is why
//!   the original program accepts the empty string as seed 0),
//! * on overflow the result is `ULONG_MAX` and `ERANGE` is reported.

/// Result of a `strtoul` call.
pub struct StrToUlResult {
    /// The converted value (`unsigned long`, i.e. 64-bit on Linux/x86-64).
    pub value: u64,
    /// Byte offset of `endptr` relative to the start of the input.
    pub end_offset: usize,
    /// Whether `errno` was set to `ERANGE`.
    pub erange: bool,
}

/// `isspace()` in the C locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// `strtoul(s, &endptr, 10)`.
pub fn strtoul_base10(s: &[u8]) -> StrToUlResult {
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
    let mut magnitude: u64 = 0;
    let mut overflow = false;

    while i < s.len() && s[i].is_ascii_digit() {
        let digit = u64::from(s[i] - b'0');
        match magnitude
            .checked_mul(10)
            .and_then(|m| m.checked_add(digit))
        {
            Some(m) => magnitude = m,
            None => overflow = true,
        }
        i += 1;
    }

    if i == digits_start {
        // No conversion could be performed: endptr == nptr, value == 0, and
        // glibc leaves errno untouched in this case.
        return StrToUlResult {
            value: 0,
            end_offset: 0,
            erange: false,
        };
    }

    if overflow {
        return StrToUlResult {
            value: u64::MAX,
            end_offset: i,
            erange: true,
        };
    }

    let value = if negative {
        magnitude.wrapping_neg()
    } else {
        magnitude
    };

    StrToUlResult {
        value,
        end_offset: i,
        erange: false,
    }
}
