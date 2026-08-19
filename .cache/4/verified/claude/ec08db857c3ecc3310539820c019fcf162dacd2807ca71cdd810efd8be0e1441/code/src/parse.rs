// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Translation of `parse_val()` plus the `fgets()` call that feeds it.

use std::io::Read;

/// True for the characters C's `isspace()` accepts in the "C" locale, which is
/// exactly the set of leading characters `strtol` skips.  The C program never
/// calls `setlocale`, so the "C" locale always applies.
pub fn c_isspace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Result of an emulated base-10 `strtol` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrtolResult {
    /// The converted value (saturated to `LONG_MIN`/`LONG_MAX` on `ERANGE`).
    pub value: i64,
    /// Number of bytes consumed; `0` means "no conversion", i.e. `endp == str`.
    pub consumed: usize,
    /// Whether `errno` would have been set to `ERANGE`.
    pub erange: bool,
}

/// Emulates `strtol(str, &endp, 10)` for a NUL-terminated C string given as the
/// bytes preceding the terminator.
///
/// `long` is 64 bits on the LP64 targets the C is built for, so the saturation
/// bounds are `i64::MIN` / `i64::MAX`.
pub fn strtol_base10(s: &[u8]) -> StrtolResult {
    let mut i = 0usize;

    // Skip leading whitespace.
    while i < s.len() && c_isspace(s[i]) {
        i += 1;
    }

    // Optional sign.
    let negative = match s.get(i) {
        Some(b'-') => {
            i += 1;
            true
        }
        Some(b'+') => {
            i += 1;
            false
        }
        _ => false,
    };

    let digits_start = i;
    // Magnitude limit: |LONG_MIN| == 2^63 for negatives, LONG_MAX for positives.
    let limit: u64 = if negative {
        i64::MAX as u64 + 1
    } else {
        i64::MAX as u64
    };

    let mut acc: u64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = u64::from(s[i] - b'0');
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) if v <= limit => acc = v,
                _ => overflow = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits consumed: `strtol` performs no conversion, stores `nptr`
        // into `endptr` and returns 0.  glibc leaves `errno` untouched here, so
        // the `errno == 0` half of `parse_val`'s test still passes and the
        // rejection comes from `endp != str` being false.
        return StrtolResult {
            value: 0,
            consumed: 0,
            erange: false,
        };
    }

    if overflow {
        return StrtolResult {
            value: if negative { i64::MIN } else { i64::MAX },
            consumed: i,
            erange: true,
        };
    }

    let value = if negative {
        // Two's-complement negation, correct even for acc == 2^63 (LONG_MIN).
        (acc as i64).wrapping_neg()
    } else {
        acc as i64
    };

    StrtolResult {
        value,
        consumed: i,
        erange: false,
    }
}

/// `static bool parse_val(const char *str, int *val)`
///
/// ```c
/// errno = 0;
/// char *endp = (char *)str;
/// long tmp = strtol(str, &endp, 10);
/// if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX) { ... }
/// ```
///
/// The order of the four conditions is preserved; because they are all pure
/// predicates over the same `strtol` result, evaluating them as one `&&` chain
/// is observationally identical.
pub fn parse_val(str_bytes: &[u8]) -> Option<i32> {
    // errno = 0;
    let r = strtol_base10(str_bytes);

    if r.consumed != 0
        && !r.erange
        && r.value >= i64::from(i32::MIN)
        && r.value <= i64::from(i32::MAX)
    {
        Some(r.value as i32)
    } else {
        None
    }
}

/// Emulates `fgets(in, buf_size, stdin)` over a `char in[buf_size]` buffer that
/// was zero-initialised with `= ""`.
///
/// Returns the bytes the resulting C string consists of, i.e. everything up to
/// (but excluding) the first NUL terminator — exactly what `strtol` observes.
///
/// * At most `buf_size - 1` bytes are consumed.
/// * Reading stops right after the first `'\n'`, which stays in the buffer.
/// * On immediate EOF (or a read error) `fgets` returns `NULL` and leaves the
///   zero-initialised buffer alone, which yields the empty C string.
pub fn fgets_line(buf_size: usize) -> Vec<u8> {
    let mut stdin = std::io::stdin().lock();
    fgets_from(&mut stdin, buf_size)
}

/// `fgets_line` against an arbitrary reader (used by the tests).
pub fn fgets_from(reader: &mut impl Read, buf_size: usize) -> Vec<u8> {
    debug_assert!(buf_size > 0);
    let max_bytes = buf_size - 1; // Room for the NUL terminator.

    let mut buf: Vec<u8> = Vec::with_capacity(max_bytes);
    let mut byte = [0u8; 1];

    while buf.len() < max_bytes {
        match reader.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break; // fgets keeps the newline and stops.
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break, // fgets() returns NULL; the buffer stays as read.
        }
    }

    // The buffer is a C string: everything from the first NUL byte on is
    // invisible to `strtol`.
    if let Some(nul) = buf.iter().position(|&b| b == 0) {
        buf.truncate(nul);
    }

    buf
}
