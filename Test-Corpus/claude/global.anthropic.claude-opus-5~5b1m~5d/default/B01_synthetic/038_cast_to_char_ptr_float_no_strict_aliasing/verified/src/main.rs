// Rust translation of c_src/src/main.c
//
// Original copyright notice from the C source:
//
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

use std::io::{Read, Write};

// ---------------------------------------------------------------------------
// print_hex / driver (direct translations)
// ---------------------------------------------------------------------------

fn print_hex(out: &mut dyn Write, p: &[u8], len: usize) {
    for i in 0..len {
        let _ = write!(out, "{:02x}", p[i]);
    }
    let _ = write!(out, "\n");
}

fn driver(out: &mut dyn Write, x: f32) {
    // char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));
    let raw: [u8; 4] = x.to_ne_bytes();
    print_hex(out, &raw, raw.len());
}

// ---------------------------------------------------------------------------
// scanf("%f", &x) emulation
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, PartialEq, Eq)]
enum St {
    Start,
    Sign,
    Zero,
    Int,
    Dot,
    LeadDot,
    Frac,
    E,
    ESign,
    Exp,
    HexPrefix,
    HexInt,
    HexDot,
    HexLeadDot,
    HexFrac,
    P,
    PSign,
    PExp,
    I,
    In,
    Inf,
    Infi,
    Infin,
    Infini,
    Infinit,
    Infinity,
    N,
    Na,
    Nan,
    NanOpen,
    NanClose,
}

fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn is_hex(c: u8) -> bool {
    c.is_ascii_hexdigit()
}

fn accepting(s: St) -> bool {
    matches!(
        s,
        St::Zero
            | St::Int
            | St::Dot
            | St::Frac
            | St::Exp
            | St::HexInt
            | St::HexDot
            | St::HexFrac
            | St::PExp
            | St::Inf
            | St::Infinity
            | St::Nan
            | St::NanClose
    )
}

/// Transition of the "expected form of a floating point subject sequence" DFA
/// (as described by C's strtof / scanf %f). Returns `None` when the character
/// cannot continue any valid prefix.
fn step(s: St, c: u8) -> Option<St> {
    let lc = c.to_ascii_lowercase();
    Some(match s {
        St::Start | St::Sign => match lc {
            b'+' | b'-' if s == St::Start => St::Sign,
            b'0' => St::Zero,
            b'1'..=b'9' => St::Int,
            b'.' => St::LeadDot,
            b'i' => St::I,
            b'n' => St::N,
            _ => return None,
        },
        St::Zero => match lc {
            b'x' => St::HexPrefix,
            b'.' => St::Dot,
            b'e' => St::E,
            _ if is_digit(c) => St::Int,
            _ => return None,
        },
        St::Int => match lc {
            b'.' => St::Dot,
            b'e' => St::E,
            _ if is_digit(c) => St::Int,
            _ => return None,
        },
        St::Dot => match lc {
            b'e' => St::E,
            _ if is_digit(c) => St::Frac,
            _ => return None,
        },
        St::LeadDot => {
            if is_digit(c) {
                St::Frac
            } else {
                return None;
            }
        }
        St::Frac => match lc {
            b'e' => St::E,
            _ if is_digit(c) => St::Frac,
            _ => return None,
        },
        St::E => match lc {
            b'+' | b'-' => St::ESign,
            _ if is_digit(c) => St::Exp,
            _ => return None,
        },
        St::ESign | St::Exp => {
            if is_digit(c) {
                St::Exp
            } else {
                return None;
            }
        }
        St::HexPrefix => {
            if is_hex(c) {
                St::HexInt
            } else if c == b'.' {
                St::HexLeadDot
            } else {
                return None;
            }
        }
        St::HexInt => {
            if is_hex(c) {
                St::HexInt
            } else if c == b'.' {
                St::HexDot
            } else if lc == b'p' {
                St::P
            } else {
                return None;
            }
        }
        St::HexDot => {
            if is_hex(c) {
                St::HexFrac
            } else if lc == b'p' {
                St::P
            } else {
                return None;
            }
        }
        St::HexLeadDot => {
            if is_hex(c) {
                St::HexFrac
            } else {
                return None;
            }
        }
        St::HexFrac => {
            if is_hex(c) {
                St::HexFrac
            } else if lc == b'p' {
                St::P
            } else {
                return None;
            }
        }
        St::P => match lc {
            b'+' | b'-' => St::PSign,
            _ if is_digit(c) => St::PExp,
            _ => return None,
        },
        St::PSign | St::PExp => {
            if is_digit(c) {
                St::PExp
            } else {
                return None;
            }
        }
        St::I => {
            if lc == b'n' {
                St::In
            } else {
                return None;
            }
        }
        St::In => {
            if lc == b'f' {
                St::Inf
            } else {
                return None;
            }
        }
        St::Inf => {
            if lc == b'i' {
                St::Infi
            } else {
                return None;
            }
        }
        St::Infi => {
            if lc == b'n' {
                St::Infin
            } else {
                return None;
            }
        }
        St::Infin => {
            if lc == b'i' {
                St::Infini
            } else {
                return None;
            }
        }
        St::Infini => {
            if lc == b't' {
                St::Infinit
            } else {
                return None;
            }
        }
        St::Infinit => {
            if lc == b'y' {
                St::Infinity
            } else {
                return None;
            }
        }
        St::N => {
            if lc == b'a' {
                St::Na
            } else {
                return None;
            }
        }
        St::Na => {
            if lc == b'n' {
                St::Nan
            } else {
                return None;
            }
        }
        St::Nan => {
            if c == b'(' {
                St::NanOpen
            } else {
                return None;
            }
        }
        St::NanOpen => {
            if c == b')' {
                St::NanClose
            } else if c.is_ascii_alphanumeric() || c == b'_' {
                St::NanOpen
            } else {
                return None;
            }
        }
        St::Infinity | St::NanClose => return None,
    })
}

struct ByteReader<R: Read> {
    inner: R,
}

impl<R: Read> ByteReader<R> {
    fn next_byte(&mut self) -> Option<u8> {
        let mut b = [0u8; 1];
        loop {
            match self.inner.read(&mut b) {
                Ok(0) => return None,
                Ok(_) => return Some(b[0]),
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            }
        }
    }
}

/// Emulates `scanf("%f", &x)`: returns `Some(value)` on a successful
/// conversion, `None` on matching failure or EOF (in which case the C code
/// leaves `x` untouched).
fn scan_float<R: Read>(r: &mut ByteReader<R>) -> Option<f32> {
    // %f skips leading white space (isspace).
    let mut c = loop {
        let c = r.next_byte()?;
        if !matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
            break c;
        }
    };

    let mut buf: Vec<u8> = Vec::new();
    let mut state = St::Start;
    let mut valid_len = 0usize;

    loop {
        match step(state, c) {
            Some(next) => {
                buf.push(c);
                state = next;
                if accepting(state) {
                    valid_len = buf.len();
                }
            }
            None => break,
        }
        match r.next_byte() {
            Some(n) => c = n,
            None => break,
        }
    }

    // glibc's scanf can only push back a single character. Once it has seen
    // "inf" followed by an 'i' it commits to matching the rest of "infinity";
    // if that fails the whole conversion fails (rather than falling back to
    // the "inf" prefix the way strtof would).
    if matches!(state, St::Infi | St::Infin | St::Infini | St::Infinit) {
        return None;
    }

    // glibc's scanf accumulates the subject sequence into a buffer and then
    // hands it to strtof -- but it first rejects the case where the buffer is
    // nothing but an (optionally signed) "0x"/"0X" hex prefix, reporting a
    // matching failure instead of converting. That is observable: strtof("-0x")
    // would yield -0.0, yet the C program prints 00000000 (the untouched
    // initial +0.f) for input "-0x". A following '.' is enough to make glibc
    // accept again ("-0x." prints 00000080), so the check is exactly "the DFA
    // never left the hex-prefix state".
    if state == St::HexPrefix {
        return None;
    }

    if valid_len == 0 {
        return None;
    }
    Some(convert(&buf[..valid_len]))
}

fn scale_pow2(mut x: f64, mut n: i64) -> f64 {
    // 2^512 and 2^-512 as exact f64 values.
    let up = f64::from_bits(((1023i64 + 512) as u64) << 52);
    let down = f64::from_bits(((1023i64 - 512) as u64) << 52);
    while n > 512 {
        x *= up;
        n -= 512;
    }
    while n < -512 {
        x *= down;
        n += 512;
    }
    // For -512 <= n <= 512, 2^n is an exactly representable normal f64.
    x * f64::from_bits(((1023i64 + n) as u64) << 52)
}

fn convert(s: &[u8]) -> f32 {
    let mut i = 0usize;
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let body = &s[i..];
    let lower: Vec<u8> = body.iter().map(|c| c.to_ascii_lowercase()).collect();

    let value: f32 = if lower.starts_with(b"inf") {
        f32::INFINITY
    } else if lower.starts_with(b"nan") {
        f32::NAN
    } else if lower.starts_with(b"0x") {
        hex_to_f32(&lower[2..])
    } else {
        dec_to_f32(&lower)
    };

    if neg {
        -value
    } else {
        value
    }
}

fn dec_to_f32(s: &[u8]) -> f32 {
    // Normalize the shapes Rust's parser does not accept ("1.", "1.e5").
    let (mant, exp) = match s.iter().position(|&c| c == b'e') {
        Some(p) => (&s[..p], Some(&s[p..])),
        None => (s, None),
    };
    let mut out = String::new();
    let mant_str = std::str::from_utf8(mant).unwrap_or("0");
    let trimmed = mant_str.trim_end_matches('.');
    out.push_str(if trimmed.is_empty() { "0" } else { trimmed });
    if let Some(e) = exp {
        out.push_str(std::str::from_utf8(e).unwrap_or(""));
    }
    out.parse::<f32>().unwrap_or(0.0)
}

fn hex_to_f32(s: &[u8]) -> f32 {
    let mut m: u128 = 0;
    let mut exp2: i64 = 0;
    let mut sticky = false;
    let mut seen_dot = false;
    let mut i = 0usize;

    while i < s.len() {
        let c = s[i];
        if c == b'.' {
            seen_dot = true;
            i += 1;
            continue;
        }
        if c == b'p' {
            break;
        }
        let d = match (c as char).to_digit(16) {
            Some(d) => d as u128,
            None => break,
        };
        if m <= (u128::MAX >> 4) {
            m = (m << 4) | d;
            if seen_dot {
                exp2 -= 4;
            }
        } else {
            if d != 0 {
                sticky = true;
            }
            if !seen_dot {
                exp2 += 4;
            }
        }
        i += 1;
    }

    let mut pexp: i64 = 0;
    if i < s.len() && s[i] == b'p' {
        i += 1;
        let mut eneg = false;
        if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
            eneg = s[i] == b'-';
            i += 1;
        }
        let mut v: i64 = 0;
        while i < s.len() && s[i].is_ascii_digit() {
            v = v.saturating_mul(10).saturating_add((s[i] - b'0') as i64);
            if v > 100_000 {
                v = 100_000;
            }
            i += 1;
        }
        pexp = if eneg { -v } else { v };
    }

    if m == 0 {
        return 0.0;
    }

    // Round to odd at 53 significant bits so the later f64 -> f32 rounding is
    // correctly rounded (no double rounding).
    let bits = 128 - m.leading_zeros() as i64;
    if bits > 53 {
        let shift = (bits - 53) as u32;
        let dropped = m & ((1u128 << shift) - 1);
        m >>= shift;
        exp2 += shift as i64;
        if dropped != 0 || sticky {
            m |= 1;
        }
    } else if sticky {
        m |= 1;
    }

    let v = scale_pow2(m as f64, exp2.saturating_add(pexp));
    v as f32
}

fn main() {
    let stdin = std::io::stdin();
    let mut reader = ByteReader {
        inner: stdin.lock(),
    };

    let mut x: f32 = 0.0;
    if let Some(v) = scan_float(&mut reader) {
        x = v;
    }

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    driver(&mut out, x);
    let _ = out.flush();
}
