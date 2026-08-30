//! Faithful re-implementations of the small pieces of the C standard library
//! that the original program relies on:
//!
//! * `strtoul` (base 10) including its `endptr` / `ERANGE` behaviour, and
//! * `srand` / `rand` as implemented by glibc (the TYPE_3 additive-feedback
//!   generator with a degree of 31 and a separation of 3).
//!
//! Reproducing glibc's generator bit-for-bit is required because the program's
//! only output is derived from the `rand()` stream.

/// Result of a `strtoul` call.
pub struct StrToULResult {
    /// The converted value (already truncated/wrapped exactly like C does).
    pub value: u64,
    /// Index of the first unconsumed byte, i.e. what `endptr` would point at.
    pub end_index: usize,
    /// `true` when the conversion would have set `errno` to `ERANGE`.
    pub range_error: bool,
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// `strtoul(s, &endptr, 10)` for a NUL-terminated string whose bytes (without
/// the terminator) are `s`.
///
/// As in C, when no digits could be converted the value is `0` and `endptr` is
/// left pointing at the very beginning of the string.
pub fn strtoul_base10(s: &[u8]) -> StrToULResult {
    let mut i = 0usize;

    // Leading white space is skipped.
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    // An optional sign may follow.
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

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
        // No conversion could be performed.
        return StrToULResult {
            value: 0,
            end_index: 0,
            range_error: false,
        };
    }

    if overflow {
        return StrToULResult {
            value: u64::MAX, // ULONG_MAX
            end_index: i,
            range_error: true,
        };
    }

    // A negative value is negated modulo 2^64, exactly like C.
    let value = if negative { value.wrapping_neg() } else { value };

    StrToULResult {
        value,
        end_index: i,
        range_error: false,
    }
}

const DEG: usize = 31;
const SEP: usize = 3;

/// glibc's default `rand()` generator (TYPE_3).
pub struct GlibcRand {
    state: [i32; DEG],
    fptr: usize,
    rptr: usize,
}

impl GlibcRand {
    /// Equivalent of `srand(seed)`.
    pub fn new(seed: u32) -> Self {
        // glibc replaces a seed of 0 with 1 so the state never becomes all-zero.
        let seed = if seed == 0 { 1 } else { seed };

        let mut state = [0i32; DEG];
        state[0] = seed as i32;
        for i in 1..DEG {
            // state[i] = (16807 * state[i - 1]) % 2147483647 computed without
            // overflowing a 32 bit signed integer (Schrage's method).
            let prev = i64::from(state[i - 1]);
            let hi = prev / 127773;
            let lo = prev % 127773;
            let mut word = 16807 * lo - 2836 * hi;
            if word < 0 {
                word += 2147483647;
            }
            state[i] = word as i32;
        }

        let mut rng = GlibcRand {
            state,
            fptr: SEP,
            rptr: 0,
        };

        // glibc discards the first 10 * DEG outputs.
        for _ in 0..(10 * DEG) {
            rng.next();
        }

        rng
    }

    /// Equivalent of `rand()`.
    pub fn next(&mut self) -> i32 {
        let val = self.state[self.fptr].wrapping_add(self.state[self.rptr]);
        self.state[self.fptr] = val;

        self.fptr += 1;
        if self.fptr >= DEG {
            self.fptr = 0;
        }
        self.rptr += 1;
        if self.rptr >= DEG {
            self.rptr = 0;
        }

        (val >> 1) & 0x7fff_ffff
    }
}
