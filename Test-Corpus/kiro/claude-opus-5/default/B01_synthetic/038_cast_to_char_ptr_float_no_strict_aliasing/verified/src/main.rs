// Rust translation of c_src/src/main.c
//
// Original C:
//   float x = 0.f;
//   scanf("%f", &x);          // x is left at 0.f on a matching failure
//   driver(x);                // prints the 4 bytes of x in memory order
//                             // as lowercase hex, then a newline
//
// The observable output is the exact bit pattern of the parsed float, so the
// `%f` conversion has to follow the same grammar, the same accept/reject
// decisions and the same rounding as the C library (glibc's `vfscanf` plus
// `strtof`).

use std::io::{Read, Write};

// ---------------------------------------------------------------------------
// Lazy stdin, mirroring scanf: bytes are pulled only as the conversion needs
// them, so the program never waits for end-of-file the way a full read would.
// ---------------------------------------------------------------------------

struct Input {
    data: Vec<u8>,
    eof: bool,
}

impl Input {
    fn new() -> Self {
        Input {
            data: Vec::new(),
            eof: false,
        }
    }

    /// Byte at absolute position `i`, or `None` at end of file.
    fn at(&mut self, i: usize) -> Option<u8> {
        while !self.eof && self.data.len() <= i {
            let mut byte = [0u8; 1];
            match std::io::stdin().read(&mut byte) {
                Ok(0) => self.eof = true,
                Ok(_) => self.data.push(byte[0]),
                Err(_) => self.eof = true,
            }
        }
        self.data.get(i).copied()
    }

    fn is(&mut self, i: usize, c: u8) -> bool {
        self.at(i) == Some(c)
    }

    fn is_digit(&mut self, i: usize) -> bool {
        matches!(self.at(i), Some(c) if c.is_ascii_digit())
    }

    /// Case-insensitive comparison of `word` (already lowercase) at `pos`.
    fn matches_ci(&mut self, pos: usize, word: &[u8]) -> bool {
        for (k, w) in word.iter().enumerate() {
            match self.at(pos + k) {
                Some(c) if lower(c) == *w => {}
                _ => return false,
            }
        }
        true
    }

    fn slice(&self, a: usize, b: usize) -> &[u8] {
        &self.data[a..b]
    }
}

// ---------------------------------------------------------------------------
// C-side helpers
// ---------------------------------------------------------------------------

/// `static void print_hex(unsigned char *p, int len)`
fn print_hex(p: &[u8], len: usize) {
    let mut buf = String::with_capacity(2 * len + 1);
    for i in 0..len {
        // printf("%02x", p[i]);
        buf.push(hex_digit(p[i] >> 4));
        buf.push(hex_digit(p[i] & 0x0f));
    }
    buf.push('\n'); // printf("\n");

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(buf.as_bytes());
    let _ = out.flush();
}

fn hex_digit(v: u8) -> char {
    if v < 10 {
        (b'0' + v) as char
    } else {
        (b'a' + (v - 10)) as char
    }
}

/// `void driver(float x)`
fn driver(x: f32) {
    // char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));
    let raw = x.to_ne_bytes();
    print_hex(&raw, raw.len());
}

// ---------------------------------------------------------------------------
// scanf("%f", &x)
// ---------------------------------------------------------------------------

/// `isspace` in the "C" locale.
fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn lower(c: u8) -> u8 {
    if c.is_ascii_uppercase() {
        c + 32
    } else {
        c
    }
}

fn hex_val(c: u8) -> Option<u32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as u32),
        b'a'..=b'f' => Some((c - b'a') as u32 + 10),
        b'A'..=b'F' => Some((c - b'A') as u32 + 10),
        _ => None,
    }
}

/// Applies the sign the way the C library does, including for zero and NaN.
fn with_sign(v: f32, neg: bool) -> f32 {
    if neg {
        f32::from_bits(v.to_bits() | 0x8000_0000)
    } else {
        f32::from_bits(v.to_bits() & 0x7fff_ffff)
    }
}

/// Reads an exponent (`[+-]digits`) starting at `i`, which must point at the
/// `e`/`p` marker.  Returns the exponent value and the position after it, or
/// `None` when no digit follows, in which case the marker is not consumed.
fn scan_exponent(inp: &mut Input, i: usize) -> Option<(i64, usize)> {
    let mut j = i + 1;
    let mut eneg = false;
    if inp.is(j, b'+') || inp.is(j, b'-') {
        eneg = inp.is(j, b'-');
        j += 1;
    }
    let dstart = j;
    let mut val: i64 = 0;
    while inp.is_digit(j) {
        let d = (inp.at(j).unwrap() - b'0') as i64;
        // Clamped: beyond this magnitude the result is inf or 0 either way.
        if val < 1_000_000_000 {
            val = val * 10 + d;
        }
        j += 1;
    }
    if j == dstart {
        None
    } else {
        Some((if eneg { -val } else { val }, j))
    }
}

/// Emulates `scanf("%f", &x)`.  `None` means matching/input failure, in which
/// case the caller leaves the destination untouched.
fn scan_float(inp: &mut Input) -> Option<f32> {
    let mut i = 0usize;

    // Leading white space is skipped; end of file here is an input failure.
    while matches!(inp.at(i), Some(c) if is_space(c)) {
        i += 1;
    }
    inp.at(i)?;

    let mut neg = false;
    if inp.is(i, b'+') || inp.is(i, b'-') {
        neg = inp.is(i, b'-');
        i += 1;
    }

    // "inf" / "infinity".  glibc consumes the letters one at a time and cannot
    // back out of a partial match, so "infi" .. "infinit" are failures.
    if inp.matches_ci(i, b"inf") {
        i += 3;
        let tail = b"inity";
        let mut k = 0usize;
        while k < tail.len() && inp.matches_ci(i + k, &tail[k..k + 1]) {
            k += 1;
        }
        if k != 0 && k != tail.len() {
            return None;
        }
        return Some(with_sign(f32::INFINITY, neg));
    }

    // "nan".  glibc's scanf stops after the three letters; a following
    // "(n-char-sequence)" stays in the stream and never becomes a payload.
    if inp.matches_ci(i, b"nan") {
        return Some(with_sign(f32::from_bits(0x7fc0_0000), neg));
    }

    // Hexadecimal form: "0x" / "0X" immediately after the optional sign.
    if inp.is(i, b'0') && (inp.matches_ci(i + 1, b"x")) {
        i += 2;

        // Significand digits are folded into `m` (top bits) plus a sticky
        // flag for everything that falls off the bottom; `exp` is the binary
        // exponent of `m`'s unit position.
        let mut m: u64 = 0;
        let mut sticky = false;
        let mut exp: i64 = 0;
        let mut digits = 0usize;

        let push = |d: u32, m: &mut u64, sticky: &mut bool, exp: &mut i64| {
            if (*m >> 60) != 0 {
                if d != 0 {
                    *sticky = true;
                }
                *exp += 4;
            } else {
                *m = (*m << 4) | d as u64;
            }
        };

        while let Some(d) = inp.at(i).and_then(hex_val) {
            push(d, &mut m, &mut sticky, &mut exp);
            digits += 1;
            i += 1;
        }
        let mut saw_dot = false;
        if inp.is(i, b'.') {
            saw_dot = true;
            i += 1;
            while let Some(d) = inp.at(i).and_then(hex_val) {
                push(d, &mut m, &mut sticky, &mut exp);
                exp -= 4;
                digits += 1;
                i += 1;
            }
        }

        if digits == 0 {
            // glibc rejects a bare "0x"/"0X" prefix, but "0x." still reaches
            // strtof, which converts just the leading "0".
            if saw_dot {
                return Some(with_sign(0.0, neg));
            }
            return None;
        }

        if inp.matches_ci(i, b"p") {
            if let Some((pexp, _next)) = scan_exponent(inp, i) {
                exp = exp.saturating_add(pexp);
            }
        }

        return Some(with_sign(hex_to_f32(m, sticky, exp), neg));
    }

    // Decimal form.
    let int_start = i;
    while inp.is_digit(i) {
        i += 1;
    }
    let int_end = i;

    let mut frac_start = i;
    let mut frac_end = i;
    if inp.is(i, b'.') {
        i += 1;
        frac_start = i;
        while inp.is_digit(i) {
            i += 1;
        }
        frac_end = i;
    }
    if int_start == int_end && frac_start == frac_end {
        // No digits at all: "e5", ".", "-", "abc", ... -> nothing converted,
        // and strtof's zero return is not signed, so x is simply left alone.
        return None;
    }

    let mut exp10: i64 = 0;
    if inp.matches_ci(i, b"e") {
        if let Some((e, _next)) = scan_exponent(inp, i) {
            exp10 = e;
        }
    }

    // Rebuild a literal that Rust's correctly rounded parser accepts verbatim;
    // the value is unchanged, only the syntax is normalised.
    let int_digits = inp.slice(int_start, int_end);
    let frac_digits = inp.slice(frac_start, frac_end);
    let mut s = String::with_capacity(int_digits.len() + frac_digits.len() + 24);
    if int_digits.is_empty() {
        s.push('0');
    } else {
        s.push_str(std::str::from_utf8(int_digits).unwrap());
    }
    s.push('.');
    if frac_digits.is_empty() {
        s.push('0');
    } else {
        s.push_str(std::str::from_utf8(frac_digits).unwrap());
    }
    s.push('e');
    s.push_str(&exp10.to_string());

    let magnitude: f32 = s.parse().unwrap_or(0.0);
    Some(with_sign(magnitude, neg))
}

/// Rounds `m * 2^exp` to the nearest `f32`, ties to even.  `sticky` means
/// "there are further nonzero bits below the ones kept in `m`".
fn hex_to_f32(m: u64, sticky: bool, exp: i64) -> f32 {
    if m == 0 {
        return 0.0;
    }
    let nbits = 64 - m.leading_zeros() as i64;
    let e = exp + nbits - 1; // exponent of the leading bit
    if e > 127 {
        return f32::INFINITY;
    }

    // Exponent of the least significant bit the result can represent.
    let target_lsb = if e - 23 > -149 { e - 23 } else { -149 };
    let shift = target_lsb - exp;

    let mm = m as u128;
    let mut q: u128;
    if shift <= 0 {
        q = mm << ((-shift) as u32);
    } else if shift >= 128 {
        return 0.0; // far below half of the smallest subnormal
    } else {
        let s = shift as u32;
        q = mm >> s;
        let discarded = mm & ((1u128 << s) - 1);
        let half = 1u128 << (s - 1);
        let round_up =
            discarded > half || (discarded == half && (sticky || (q & 1) == 1));
        if round_up {
            q += 1;
        }
    }

    let mut lsb = target_lsb;
    if q >= (1u128 << 24) {
        // Rounding carried into a new bit position.
        q >>= 1;
        lsb += 1;
    }

    if lsb == -149 && q < (1u128 << 23) {
        return f32::from_bits(q as u32); // subnormal (or zero)
    }
    let unbiased = lsb + 23;
    if unbiased > 127 {
        return f32::INFINITY;
    }
    let expfield = (unbiased + 127) as u32;
    f32::from_bits((expfield << 23) | ((q as u32) & 0x007f_ffff))
}

// ---------------------------------------------------------------------------

/// The Rust runtime installs `SIG_IGN` for `SIGPIPE` before `main`, which the C
/// program does not: a C `printf` to a pipe whose reader has gone away kills the
/// process with signal 13, while the Rust version would merely get `EPIPE` and
/// exit 0.  Restoring the default disposition keeps the exit status identical.
#[cfg(unix)]
fn restore_default_sigpipe() {
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

fn main() {
    restore_default_sigpipe();

    let mut input = Input::new();

    let mut x: f32 = 0.0; // float x = 0.f;
    if let Some(v) = scan_float(&mut input) {
        x = v;
    }
    driver(x);
}
