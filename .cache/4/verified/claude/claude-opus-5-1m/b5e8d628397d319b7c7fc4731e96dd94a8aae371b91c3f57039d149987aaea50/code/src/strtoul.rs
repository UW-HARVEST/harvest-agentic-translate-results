//! Faithful reimplementation of C `strtoul(nptr, &endptr, 10)` (glibc behaviour)
//! operating on raw bytes, so that argument validation matches the C program
//! byte for byte.

/// Result of a `strtoul` call.
pub struct StrtoulResult {
    /// The converted value (wrapped/negated exactly like C).
    pub value: u64,
    /// Number of bytes consumed; `endptr` == &nptr[consumed].
    pub consumed: usize,
    /// True if the conversion overflowed (glibc sets `errno = ERANGE`).
    pub erange: bool,
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// `strtoul(s, &endptr, 10)`.
///
/// On "no conversion" glibc leaves `errno` untouched, returns 0 and sets
/// `endptr = nptr` (i.e. `consumed == 0`); that behaviour is reproduced here.
pub fn strtoul_base10(s: &[u8]) -> StrtoulResult {
    let mut i = 0usize;

    // Skip leading white space.
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    // Optional sign.
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    // Digits.
    let digits_start = i;
    let mut value: u64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = u64::from(s[i] - b'0');
        match value.checked_mul(10).and_then(|v| v.checked_add(digit)) {
            Some(v) => value = v,
            None => overflow = true,
        }
        i += 1;
    }

    if i == digits_start {
        // No conversion performed: endptr = nptr, return 0.
        return StrtoulResult {
            value: 0,
            consumed: 0,
            erange: false,
        };
    }

    if overflow {
        // glibc: errno = ERANGE, return ULONG_MAX (regardless of sign).
        return StrtoulResult {
            value: u64::MAX,
            consumed: i,
            erange: true,
        };
    }

    let value = if negative { 0u64.wrapping_sub(value) } else { value };

    StrtoulResult {
        value,
        consumed: i,
        erange: false,
    }
}
