//! A minimal re-implementation of the exact subset of C `scanf` semantics
//! (as implemented by glibc) that `c_src/src/luggage.c` relies upon.
//!
//! Only what the original program uses is modelled:
//!   * `"%d "`                     -- signed decimal + trailing whitespace skip
//!   * `"%8[A-Z0-9] %6[A-Z0-9] "`  -- two scansets, whitespace skips
//!   * `"%3[A-Z] %3[A-Z]"`         -- two scansets
//!   * `"%80[^\n]"`                -- negated scanset
//!
//! Return-value rules reproduced from C:
//!   * A call returns `EOF` (-1) only when an input failure (end of input)
//!     happens *before* the first successful conversion.
//!   * A matching failure aborts the remainder of the format string and the
//!     call returns the number of successful assignments so far.
//!   * An input failure *after* at least one assignment returns that count.
//!   * Arguments belonging to conversions that never ran are left untouched
//!     (this is what makes stale buffer contents observable in C).

use std::io::Read;

/// C's `EOF`.
pub const EOF: i32 = -1;

/// Result of a single scanset (`%[`) directive.
enum Sc {
    /// End of input reached before reading any character.
    Eof,
    /// No character of the set was present (matching failure).
    Fail,
    /// At least one character was converted and stored.
    Ok,
}

/// `isspace()` in the "C" locale.
fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// The `[A-Z0-9]` scanset.
fn is_upper_alnum(c: u8) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_digit()
}

/// The `[A-Z]` scanset.
fn is_upper(c: u8) -> bool {
    c.is_ascii_uppercase()
}

/// The `[^\n]` scanset.
fn is_not_newline(c: u8) -> bool {
    c != b'\n'
}

/// Byte-oriented view of `stdin`, providing the single-character push-back that
/// `scanf` needs.  The program never writes output before end of input is
/// reached, so slurping the whole stream up front is observationally
/// equivalent to C's incremental, buffered reads.
pub struct Reader {
    buf: Vec<u8>,
    pos: usize,
}

impl Reader {
    /// Reads all of `stdin`.  A read error is treated like end of input, which
    /// mirrors `scanf` returning `EOF` on an input failure.
    pub fn from_stdin() -> Reader {
        let mut buf = Vec::new();
        let _ = std::io::stdin().read_to_end(&mut buf);
        Reader { buf, pos: 0 }
    }

    #[cfg(test)]
    pub fn from_bytes(bytes: &[u8]) -> Reader {
        Reader {
            buf: bytes.to_vec(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.buf.get(self.pos).copied()
    }

    fn bump(&mut self) {
        if self.pos < self.buf.len() {
            self.pos += 1;
        }
    }

    fn at_eof(&self) -> bool {
        self.pos >= self.buf.len()
    }

    /// A whitespace directive in a format string: matches any amount of
    /// whitespace, including none.  Hitting end of input here is *not* an
    /// error by itself.
    fn skip_space(&mut self) {
        while let Some(c) = self.peek() {
            if is_space(c) {
                self.bump();
            } else {
                break;
            }
        }
    }

    /// A `%<width>[set]` directive.  Never skips leading whitespace.
    ///
    /// C copies every matched byte into the caller's `char` array and appends a
    /// NUL.  A matched NUL byte (possible for `[^\n]`) is still consumed from
    /// the stream and still counts towards the width, but it terminates the
    /// resulting C string, so everything after it is invisible to `strcpy`,
    /// `strcmp` and `%s`.  `dst` therefore holds the bytes up to the first NUL,
    /// while the stream position accounts for all of them.
    fn scanset(&mut self, width: usize, dst: &mut Vec<u8>, in_set: fn(u8) -> bool) -> Sc {
        if self.at_eof() {
            return Sc::Eof;
        }
        let mut matched: Vec<u8> = Vec::new();
        while matched.len() < width {
            match self.peek() {
                Some(c) if in_set(c) => {
                    matched.push(c);
                    self.bump();
                }
                _ => break,
            }
        }
        if matched.is_empty() {
            // Matching failure: the destination keeps its previous contents.
            return Sc::Fail;
        }
        let c_string_len = matched
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(matched.len());
        matched.truncate(c_string_len);
        *dst = matched;
        Sc::Ok
    }
}

/// `scanf("%d ", &time_stamp)` where `time_stamp` is an `unsigned int`.
///
/// glibc converts `%d` with `strtol` into a `long`, saturating at
/// `LONG_MAX`/`LONG_MIN` on overflow, then stores the low 32 bits through an
/// `int *`; the original program passes an `unsigned int *`, so the same 32
/// bits land in the unsigned slot.
pub fn scan_time_stamp(r: &mut Reader, dst: &mut u32) -> i32 {
    // `%d` skips leading whitespace itself.
    r.skip_space();
    if r.at_eof() {
        // Input failure before the first conversion.
        return EOF;
    }

    let negative = match r.peek() {
        Some(b'-') => {
            r.bump();
            true
        }
        Some(b'+') => {
            r.bump();
            false
        }
        _ => false,
    };

    let mut magnitude: i64 = 0;
    let mut overflow = false;
    let mut digits = 0usize;
    while let Some(c) = r.peek() {
        if !c.is_ascii_digit() {
            break;
        }
        r.bump();
        digits += 1;
        if !overflow {
            match magnitude
                .checked_mul(10)
                .and_then(|v| v.checked_add((c - b'0') as i64))
            {
                Some(v) => magnitude = v,
                None => overflow = true,
            }
        }
    }

    if digits == 0 {
        // Matching failure: `dst` untouched, rest of the format abandoned.
        return 0;
    }

    let as_long: i64 = if overflow {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -magnitude
    } else {
        magnitude
    };

    *dst = (as_long as i32) as u32;

    // The trailing ' ' in the format eats the whitespace after the number;
    // reaching end of input while doing so is not an error.
    r.skip_space();
    1
}

/// `scanf("%8[A-Z0-9] %6[A-Z0-9] ", luggage_id, flight_id)`
pub fn scan_ids(
    r: &mut Reader,
    luggage_id: &mut Vec<u8>,
    luggage_id_width: usize,
    flight_id: &mut Vec<u8>,
    flight_id_width: usize,
) -> i32 {
    let mut done = 0;

    match r.scanset(luggage_id_width, luggage_id, is_upper_alnum) {
        Sc::Eof => return EOF, // input failure before any conversion
        Sc::Fail => return done,
        Sc::Ok => done = 1,
    }

    r.skip_space();

    match r.scanset(flight_id_width, flight_id, is_upper_alnum) {
        // Input failure / matching failure after an assignment: return count.
        Sc::Eof | Sc::Fail => return done,
        Sc::Ok => done = 2,
    }

    r.skip_space();
    done
}

/// `scanf("%3[A-Z] %3[A-Z]", departure, arrival)`
pub fn scan_airports(
    r: &mut Reader,
    departure: &mut Vec<u8>,
    departure_width: usize,
    arrival: &mut Vec<u8>,
    arrival_width: usize,
) -> i32 {
    let mut done = 0;

    match r.scanset(departure_width, departure, is_upper) {
        Sc::Eof => return EOF,
        Sc::Fail => return done,
        Sc::Ok => done = 1,
    }

    r.skip_space();

    match r.scanset(arrival_width, arrival, is_upper) {
        Sc::Eof | Sc::Fail => return done,
        Sc::Ok => done = 2,
    }

    done
}

/// `scanf("%80[^\n]", comments)`
pub fn scan_comments(r: &mut Reader, comments: &mut Vec<u8>, width: usize) -> i32 {
    match r.scanset(width, comments, is_not_newline) {
        Sc::Eof => EOF,
        Sc::Fail => 0,
        Sc::Ok => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_eats_trailing_whitespace_but_not_more() {
        let mut r = Reader::from_bytes(b"42   AB");
        let mut ts = 7u32;
        assert_eq!(scan_time_stamp(&mut r, &mut ts), 1);
        assert_eq!(ts, 42);
        let mut a = Vec::new();
        let mut b = vec![b'o', b'l', b'd'];
        assert_eq!(scan_ids(&mut r, &mut a, 8, &mut b, 6), 1);
        assert_eq!(a, b"AB");
        assert_eq!(b, b"old"); // untouched by the failed conversion
    }

    #[test]
    fn negative_and_overflow_timestamps_wrap_like_c() {
        let mut ts = 0u32;
        let mut r = Reader::from_bytes(b"-5 ");
        assert_eq!(scan_time_stamp(&mut r, &mut ts), 1);
        assert_eq!(ts, 4294967291);

        let mut r = Reader::from_bytes(b"99999999999999999999 ");
        assert_eq!(scan_time_stamp(&mut r, &mut ts), 1);
        assert_eq!(ts, u32::MAX); // LONG_MAX truncated to int

        let mut r = Reader::from_bytes(b"12345678901 ");
        assert_eq!(scan_time_stamp(&mut r, &mut ts), 1);
        assert_eq!(ts, 3755744309); // 12345678901 mod 2^32
    }

    #[test]
    fn eof_only_before_first_conversion() {
        let mut ts = 0u32;
        let mut r = Reader::from_bytes(b"   \n");
        assert_eq!(scan_time_stamp(&mut r, &mut ts), EOF);

        let mut a = Vec::new();
        let mut b = Vec::new();
        let mut r = Reader::from_bytes(b"XY");
        assert_eq!(scan_ids(&mut r, &mut a, 8, &mut b, 6), 1);
    }

    #[test]
    fn comments_stop_at_newline_and_fail_when_empty() {
        let mut c = Vec::new();
        let mut r = Reader::from_bytes(b"\nrest");
        assert_eq!(scan_comments(&mut r, &mut c, 80), 0);
        assert!(c.is_empty());

        let mut r = Reader::from_bytes(b" hi there\n");
        assert_eq!(scan_comments(&mut r, &mut c, 80), 1);
        assert_eq!(c, b" hi there");

        let mut r = Reader::from_bytes(b"");
        assert_eq!(scan_comments(&mut r, &mut c, 80), EOF);
    }

    #[test]
    fn scanset_width_limits_are_honoured() {
        let mut a = Vec::new();
        let mut b = Vec::new();
        let mut r = Reader::from_bytes(b"ABCDEFGHIJKLMNOP QRS");
        assert_eq!(scan_ids(&mut r, &mut a, 8, &mut b, 6), 2);
        assert_eq!(a, b"ABCDEFGH");
        assert_eq!(b, b"IJKLMN");
    }
}
