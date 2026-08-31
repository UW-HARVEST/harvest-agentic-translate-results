// Rust translation of c_src/src/main.c
//
// Original C:
//     float x = 0.f;
//     scanf("%f", &x);
//     driver(x);          // memcpy the float into a char[4] and hexdump it
//
// The program prints the four raw bytes of the float as "%02x" each, followed
// by a newline. If `scanf` fails to match anything, `x` keeps its initial
// value of 0.0f, so "00000000" is printed.
//
// Everything below reproduces the observable behaviour of glibc's
// `scanf("%f", ...)`: leading whitespace is skipped (including newlines),
// decimal and hexadecimal floats are accepted, as are `inf`/`infinity` and
// `nan`/`nan(n-char-sequence)`, all case-insensitively. Reading is done one
// byte at a time so no more input is consumed than `scanf` would need.

use std::io::{self, Read, Write};

// ---------------------------------------------------------------------------
// Byte-at-a-time stdin reader with one byte of lookahead (like getc/ungetc).
// ---------------------------------------------------------------------------

struct Input {
    inner: io::Stdin,
    peeked: Option<u8>,
    eof: bool,
}

impl Input {
    fn new() -> Self {
        Input {
            inner: io::stdin(),
            peeked: None,
            eof: false,
        }
    }

    fn peek(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        loop {
            match self.inner.read(&mut buf) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(_) => {
                    self.peeked = Some(buf[0]);
                    return Some(buf[0]);
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    /// Consume the byte returned by the most recent `peek`.
    fn bump(&mut self) {
        self.peeked = None;
    }

    /// Consume `expected` (case-insensitively) if it is next. Returns true on match.
    fn eat_ci(&mut self, expected: u8) -> bool {
        match self.peek() {
            Some(c) if c.eq_ignore_ascii_case(&expected) => {
                self.bump();
                true
            }
            _ => false,
        }
    }
}

fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn is_hex_digit(c: u8) -> bool {
    c.is_ascii_hexdigit()
}

fn hex_val(c: u8) -> u32 {
    match c {
        b'0'..=b'9' => (c - b'0') as u32,
        b'a'..=b'f' => (c - b'a') as u32 + 10,
        _ => (c - b'A') as u32 + 10,
    }
}

// ---------------------------------------------------------------------------
// scanf("%f") emulation. Returns the resulting f32 bit pattern, or None on a
// matching failure (in which case the C program leaves x == 0.0f).
// ---------------------------------------------------------------------------

fn scan_float(inp: &mut Input) -> Option<u32> {
    // Skip leading whitespace (scanf's %f skips it, crossing newlines).
    while let Some(c) = inp.peek() {
        if is_c_space(c) {
            inp.bump();
        } else {
            break;
        }
    }

    // Optional sign.
    let mut sign: u32 = 0;
    match inp.peek() {
        Some(b'+') => inp.bump(),
        Some(b'-') => {
            sign = 0x8000_0000;
            inp.bump();
        }
        _ => {}
    }

    match inp.peek() {
        Some(c) if c.eq_ignore_ascii_case(&b'i') => return scan_inf(inp, sign),
        Some(c) if c.eq_ignore_ascii_case(&b'n') => return scan_nan(inp, sign),
        _ => {}
    }

    // Possible "0x"/"0X" hexadecimal prefix.
    let mut leading_zero = false;
    if let Some(b'0') = inp.peek() {
        inp.bump();
        leading_zero = true;
        if let Some(c) = inp.peek() {
            if c == b'x' || c == b'X' {
                inp.bump();
                return scan_hex(inp, sign);
            }
        }
    }

    scan_decimal(inp, sign, leading_zero)
}

/// "inf", optionally spelled "infinity". glibc treats a partial "infinity"
/// (e.g. "infi", "infinit") as a matching failure, not as "inf".
fn scan_inf(inp: &mut Input, sign: u32) -> Option<u32> {
    for &c in b"inf" {
        if !inp.eat_ci(c) {
            return None;
        }
    }
    if inp.eat_ci(b'i') {
        for &c in b"nity" {
            if !inp.eat_ci(c) {
                return None;
            }
        }
    }
    Some(sign | 0x7f80_0000)
}

/// "nan", optionally followed by "(n-char-sequence)". The sequence is consumed
/// but does not affect the value: glibc's scanf yields the default quiet NaN.
fn scan_nan(inp: &mut Input, sign: u32) -> Option<u32> {
    for &c in b"nan" {
        if !inp.eat_ci(c) {
            return None;
        }
    }

    if let Some(b'(') = inp.peek() {
        inp.bump();
        while let Some(c) = inp.peek() {
            if c.is_ascii_alphanumeric() || c == b'_' {
                inp.bump();
            } else {
                break;
            }
        }
        if let Some(b')') = inp.peek() {
            inp.bump();
        }
    }

    Some(sign | 0x7fc0_0000)
}

/// Decimal float. `leading_zero` means one '0' digit was already consumed.
fn scan_decimal(inp: &mut Input, sign: u32, leading_zero: bool) -> Option<u32> {
    let mut int_part: Vec<u8> = Vec::new();
    let mut frac_part: Vec<u8> = Vec::new();
    let mut any_digit = leading_zero;
    if leading_zero {
        int_part.push(b'0');
    }

    while let Some(c) = inp.peek() {
        if is_digit(c) {
            int_part.push(c);
            any_digit = true;
            inp.bump();
        } else {
            break;
        }
    }

    let mut got_dot = false;
    if let Some(b'.') = inp.peek() {
        got_dot = true;
        inp.bump();
        while let Some(c) = inp.peek() {
            if is_digit(c) {
                frac_part.push(c);
                any_digit = true;
                inp.bump();
            } else {
                break;
            }
        }
    }

    if !any_digit {
        return None;
    }

    let exp = scan_exponent(inp, b'e');

    // Build a string accepted by Rust's f32 parser (same grammar subset,
    // correctly rounded ties-to-even, matching glibc's strtof).
    let mut text = String::new();
    if int_part.is_empty() {
        text.push('0');
    } else {
        text.push_str(std::str::from_utf8(&int_part).unwrap());
    }
    if got_dot || !frac_part.is_empty() {
        text.push('.');
        text.push_str(std::str::from_utf8(&frac_part).unwrap());
    }
    if let Some(e) = exp {
        text.push('e');
        text.push_str(&e.to_string());
    }

    let magnitude: f32 = text.parse().unwrap_or(0.0);
    Some(sign | (magnitude.to_bits() & 0x7fff_ffff))
}

/// Optional exponent: [eE]/[pP] followed by an optional sign and digits.
/// Returns None when there is no valid exponent. Saturating to keep the
/// arithmetic in range; the extremes already mean overflow/underflow.
fn scan_exponent(inp: &mut Input, marker: u8) -> Option<i64> {
    match inp.peek() {
        Some(c) if c.eq_ignore_ascii_case(&marker) => {}
        _ => return None,
    }
    inp.bump();

    let mut negative = false;
    match inp.peek() {
        Some(b'+') => inp.bump(),
        Some(b'-') => {
            negative = true;
            inp.bump();
        }
        _ => {}
    }

    let mut digits: Vec<u8> = Vec::new();
    while let Some(c) = inp.peek() {
        if is_digit(c) {
            digits.push(c);
            inp.bump();
        } else {
            break;
        }
    }
    if digits.is_empty() {
        // Not a valid exponent; glibc drops it and keeps the mantissa value.
        return None;
    }

    let mut value: i64 = 0;
    for c in digits {
        value = value
            .saturating_mul(10)
            .saturating_add((c - b'0') as i64)
            .min(1_000_000);
    }
    Some(if negative { -value } else { value })
}

/// Hexadecimal float, called after the "0x" prefix has been consumed.
/// With no hex digits after the prefix glibc reports a matching failure, so the
/// C program keeps x == 0.0f.
fn scan_hex(inp: &mut Input, sign: u32) -> Option<u32> {
    // Significand as m * 2^(4*skipped) with the dropped digits' non-zeroness
    // recorded in `sticky`, so rounding stays exact.
    let mut m: u128 = 0;
    let mut skipped: i64 = 0;
    let mut sticky = false;
    let mut frac_digits: i64 = 0;
    let mut any_digit = false;

    let push = |d: u32, m: &mut u128, skipped: &mut i64, sticky: &mut bool| {
        if *m < (1u128 << 124) {
            *m = (*m << 4) | d as u128;
        } else {
            if d != 0 {
                *sticky = true;
            }
            *skipped += 1;
        }
    };

    while let Some(c) = inp.peek() {
        if is_hex_digit(c) {
            push(hex_val(c), &mut m, &mut skipped, &mut sticky);
            any_digit = true;
            inp.bump();
        } else {
            break;
        }
    }

    if let Some(b'.') = inp.peek() {
        inp.bump();
        while let Some(c) = inp.peek() {
            if is_hex_digit(c) {
                push(hex_val(c), &mut m, &mut skipped, &mut sticky);
                frac_digits += 1;
                any_digit = true;
                inp.bump();
            } else {
                break;
            }
        }
    }

    if !any_digit {
        return None; // "0x" with no hex digits: matching failure
    }

    let p = scan_exponent(inp, b'p').unwrap_or(0);
    let exp2 = p - 4 * frac_digits + 4 * skipped;

    Some(sign | round_to_f32(m, exp2, sticky))
}

// ---------------------------------------------------------------------------
// Round m * 2^exp2 (plus sticky low bits) to the f32 magnitude bit pattern.
// ---------------------------------------------------------------------------

fn shr_sat(m: u128, s: u32) -> u128 {
    if s >= 128 {
        0
    } else {
        m >> s
    }
}

fn bit_at(m: u128, i: u32) -> u32 {
    if i >= 128 {
        0
    } else {
        ((m >> i) & 1) as u32
    }
}

fn low_bits_nonzero(m: u128, s: u32) -> bool {
    if s == 0 {
        false
    } else if s >= 128 {
        m != 0
    } else {
        m & ((1u128 << s) - 1) != 0
    }
}

fn round_to_f32(m: u128, exp2: i64, sticky: bool) -> u32 {
    if m == 0 {
        return 0;
    }
    let bl = (128 - m.leading_zeros()) as i64; // bit length of m
    let e = bl - 1 + exp2; // floor(log2(value))

    if e > 200 {
        return 0x7f80_0000; // infinity
    }
    if e < -200 {
        return 0; // zero
    }

    let mut target_exp = if e - 23 > -149 { e - 23 } else { -149 };
    let shift = target_exp - exp2;

    let (mut q, round_bit, rest_nonzero) = if shift <= 0 {
        (m << ((-shift) as u32), 0u32, false)
    } else {
        let s = shift as u32;
        (shr_sat(m, s), bit_at(m, s - 1), low_bits_nonzero(m, s - 1))
    };

    if round_bit == 1 && (rest_nonzero || sticky || (q & 1) == 1) {
        q += 1;
    }

    if q >= (1u128 << 24) {
        q >>= 1;
        target_exp += 1;
    }

    if target_exp <= -149 && q < (1u128 << 23) {
        return q as u32; // zero or subnormal
    }

    let e_final = target_exp + 23;
    if e_final > 127 {
        return 0x7f80_0000; // infinity
    }
    (((e_final + 127) as u32) << 23) | ((q as u32) & 0x007f_ffff)
}

// ---------------------------------------------------------------------------
// print_hex / driver / main
// ---------------------------------------------------------------------------

fn print_hex(out: &mut impl Write, bytes: &[u8]) {
    let mut s = String::with_capacity(bytes.len() * 2 + 1);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s.push('\n');
    let _ = out.write_all(s.as_bytes());
}

fn driver(out: &mut impl Write, x: f32) {
    let raw = x.to_bits().to_le_bytes(); // memcpy of the float's storage
    print_hex(out, &raw);
}

// ---------------------------------------------------------------------------
// The Rust runtime installs SIG_IGN for SIGPIPE before `main` runs, which the C
// program does not do. Without restoring the default disposition, a write to a
// broken stdout pipe would make this program ignore EPIPE and exit 0, whereas
// the C program is killed by SIGPIPE. Restore SIG_DFL so both die identically.
// `signal` comes from the libc that is already linked into every Rust binary,
// so no extra crate dependency is needed.
// ---------------------------------------------------------------------------

fn restore_default_sigpipe() {
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    const SIGPIPE: i32 = 13; // Linux
    const SIG_DFL: usize = 0;
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn main() {
    restore_default_sigpipe();

    let mut inp = Input::new();
    let mut x: f32 = 0.0;
    if let Some(bits) = scan_float(&mut inp) {
        x = f32::from_bits(bits);
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    driver(&mut out, x);
    let _ = out.flush();
}
