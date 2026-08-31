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
//
// Rust translation of src/main.c.

use std::io::{Read, Write};

// ---------------------------------------------------------------------------
// Byte-oriented stdin reader with unlimited pushback.
//
// C reads stdin through `scanf`, which consumes only as many characters as the
// conversion needs (plus lookahead that it pushes back).  Reading one byte at a
// time with a pushback stack reproduces that consumption behaviour and, more
// importantly, never blocks waiting for input the C program would not have
// waited for.
// ---------------------------------------------------------------------------
struct Input {
    stdin: std::io::Stdin,
    pushback: Vec<u8>,
    eof: bool,
}

impl Input {
    fn new() -> Self {
        Input {
            stdin: std::io::stdin(),
            pushback: Vec::new(),
            eof: false,
        }
    }

    fn next(&mut self) -> Option<u8> {
        if let Some(c) = self.pushback.pop() {
            return Some(c);
        }
        if self.eof {
            return None;
        }
        let mut b = [0u8; 1];
        match self.stdin.read(&mut b) {
            Ok(1) => Some(b[0]),
            _ => {
                self.eof = true;
                None
            }
        }
    }

    fn unget(&mut self, c: u8) {
        self.pushback.push(c);
    }

    /// Push a run of characters back, preserving their original order.
    fn unget_all(&mut self, s: &[u8]) {
        for &c in s.iter().rev() {
            self.pushback.push(c);
        }
    }
}

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Float assembly helpers.  Everything is carried around as raw IEEE-754
// binary32 bit patterns so that NaN payloads and signed zeros survive intact.
// ---------------------------------------------------------------------------
const SIGN: u32 = 0x8000_0000;
const INF_BITS: u32 = 0x7f80_0000;
const QNAN_BITS: u32 = 0x7fc0_0000;

/// Round `sig * 2^exp2` (with `sticky` marking discarded non-zero low bits) to
/// the nearest binary32, ties to even.  Returns the magnitude's bit pattern.
fn round_to_f32_bits(sig: u128, exp2: i64, sticky: bool) -> u32 {
    if sig == 0 {
        return 0;
    }

    let nbits = 128 - sig.leading_zeros() as i64; // bit length of sig
    let e = exp2 + nbits - 1; // unbiased exponent of the leading bit

    if e > 127 {
        return INF_BITS;
    }
    if e < -160 {
        // Far below half of the smallest subnormal.
        return 0;
    }

    // Exponent of the least significant bit we are allowed to keep.
    let target_exp = if e - 23 > -149 { e - 23 } else { -149 };
    let shift = target_exp - exp2;

    let mut q: u128;
    let mut round_up = false;

    if shift <= 0 {
        // Widening only; `sticky` is unreachable here (it is only set when the
        // significand already holds far more bits than binary32 can keep).
        q = sig << ((-shift) as u32);
    } else if shift >= 128 {
        // Everything shifts out and the remainder is below one half ulp.
        return 0;
    } else {
        let s = shift as u32;
        q = sig >> s;
        let rem = sig & ((1u128 << s) - 1);
        let half = 1u128 << (s - 1);
        round_up = rem > half || (rem == half && (sticky || (q & 1) == 1));
    }

    if round_up {
        q += 1;
    }

    let mut te = target_exp;
    if q >= 1u128 << 24 {
        q >>= 1;
        te += 1;
    }

    if te == -149 && q < 1u128 << 23 {
        // Subnormal (or zero).
        return q as u32;
    }

    let biased = te + 23 + 127;
    if biased >= 255 {
        return INF_BITS;
    }
    ((biased as u32) << 23) | ((q - (1u128 << 23)) as u32)
}

/// Consume `word` (ASCII, case-insensitive) if it matches; otherwise consume
/// nothing.
fn try_word(inp: &mut Input, word: &[u8]) -> bool {
    let mut taken: Vec<u8> = Vec::with_capacity(word.len());
    for &w in word {
        match inp.next() {
            Some(c) if (c | 0x20) == w => taken.push(c),
            Some(c) => {
                inp.unget(c);
                inp.unget_all(&taken);
                return false;
            }
            None => {
                inp.unget_all(&taken);
                return false;
            }
        }
    }
    true
}

fn scan_nan(inp: &mut Input) -> Option<u32> {
    if !try_word(inp, b"nan") {
        return None;
    }
    // glibc's scanf accepts an optional n-char-sequence but, unlike strtof,
    // discards it: the result is always the default quiet NaN.
    match inp.next() {
        Some(b'(') => loop {
            match inp.next() {
                Some(c) if c.is_ascii_alphanumeric() || c == b'_' => continue,
                Some(b')') => break,
                Some(c) => {
                    inp.unget(c);
                    break;
                }
                None => break,
            }
        },
        Some(c) => inp.unget(c),
        None => {}
    }
    Some(QNAN_BITS)
}

/// Signed decimal exponent, clamped to a range that cannot alter the result.
/// `marker` is the lowercase exponent letter; both cases are accepted.
fn scan_exponent(inp: &mut Input, marker: u8) -> i64 {
    let mut taken: Vec<u8> = Vec::new();
    let c = match inp.next() {
        Some(c) => c,
        None => return 0,
    };
    if (c | 0x20) != marker {
        inp.unget(c);
        return 0;
    }
    taken.push(c);

    let mut neg = false;
    match inp.next() {
        Some(b'+') => taken.push(b'+'),
        Some(b'-') => {
            neg = true;
            taken.push(b'-');
        }
        Some(c) => inp.unget(c),
        None => {}
    }

    let mut have = false;
    let mut acc: i64 = 0;
    loop {
        match inp.next() {
            Some(c) if c.is_ascii_digit() => {
                have = true;
                taken.push(c);
                if acc < 1_000_000 {
                    acc = acc * 10 + (c - b'0') as i64;
                }
            }
            Some(c) => {
                inp.unget(c);
                break;
            }
            None => break,
        }
    }

    if !have {
        // No exponent digits: the marker (and sign) are not part of the number.
        inp.unget_all(&taken);
        return 0;
    }
    if neg {
        -acc
    } else {
        acc
    }
}

/// Hexadecimal form: `0x` has already been consumed.
fn scan_hex(inp: &mut Input) -> Option<u32> {
    let mut digits: Vec<u8> = Vec::new();
    let mut frac_digits: i64 = 0;

    loop {
        match inp.next() {
            Some(c) => match hex_val(c) {
                Some(d) => digits.push(d),
                None => {
                    inp.unget(c);
                    break;
                }
            },
            None => break,
        }
    }

    let mut saw_point = false;
    match inp.next() {
        Some(b'.') => {
            saw_point = true;
            loop {
                match inp.next() {
                    Some(c) => match hex_val(c) {
                        Some(d) => {
                            digits.push(d);
                            frac_digits += 1;
                        }
                        None => {
                            inp.unget(c);
                            break;
                        }
                    },
                    None => break,
                }
            }
        }
        Some(c) => inp.unget(c),
        None => {}
    }

    if digits.is_empty() {
        // glibc's scanf fails the conversion when nothing but "0x" was
        // accumulated, but a decimal point is enough to keep it alive; the
        // underlying strtof then stops after the leading "0", giving zero.
        if saw_point {
            let _ = scan_exponent(inp, b'p');
            return Some(0);
        }
        return None;
    }

    let pexp = scan_exponent(inp, b'p');

    // Value == digits (as an integer) * 2^(pexp - 4*frac_digits).
    let mut sig: u128 = 0;
    let mut kept = 0usize;
    let mut dropped = 0i64;
    let mut sticky = false;
    let mut leading = true;
    for &d in &digits {
        if leading {
            if d == 0 {
                continue;
            }
            leading = false;
        }
        if kept < 28 {
            sig = (sig << 4) | d as u128;
            kept += 1;
        } else {
            dropped += 1;
            if d != 0 {
                sticky = true;
            }
        }
    }

    let exp2 = pexp - 4 * frac_digits + 4 * dropped;
    Some(round_to_f32_bits(sig, exp2, sticky))
}

/// Decimal form.
fn scan_decimal(inp: &mut Input, first: Option<u8>) -> Option<u32> {
    let mut mant = String::new();
    let mut ndigits = 0usize;

    if let Some(c) = first {
        mant.push(c as char);
        ndigits += 1;
    }

    loop {
        match inp.next() {
            Some(c) if c.is_ascii_digit() => {
                mant.push(c as char);
                ndigits += 1;
            }
            Some(c) => {
                inp.unget(c);
                break;
            }
            None => break,
        }
    }

    match inp.next() {
        Some(b'.') => {
            mant.push('.');
            loop {
                match inp.next() {
                    Some(c) if c.is_ascii_digit() => {
                        mant.push(c as char);
                        ndigits += 1;
                    }
                    Some(c) => {
                        inp.unget(c);
                        break;
                    }
                    None => break,
                }
            }
        }
        Some(c) => inp.unget(c),
        None => {}
    }

    if ndigits == 0 {
        return None; // matching failure
    }

    let exp = scan_exponent(inp, b'e');
    if exp != 0 {
        mant.push('e');
        mant.push_str(&exp.to_string());
    }

    // Rust's decimal parser is correctly rounded, matching strtof.
    match mant.parse::<f32>() {
        Ok(v) => Some(v.to_bits()),
        Err(_) => None,
    }
}

/// One `scanf("%f", ...)` conversion.  Returns the magnitude/sign bit pattern
/// on success, `None` on input or matching failure (in which case the C code
/// leaves its variable untouched).
fn scan_float_bits(inp: &mut Input) -> Option<u32> {
    // Leading whitespace.
    loop {
        match inp.next() {
            None => return None,
            Some(c) if is_space(c) => continue,
            Some(c) => {
                inp.unget(c);
                break;
            }
        }
    }

    let mut sign: u32 = 0;
    match inp.next() {
        None => return None,
        Some(b'+') => {}
        Some(b'-') => sign = SIGN,
        Some(c) => inp.unget(c),
    }

    let c = match inp.next() {
        Some(c) => c,
        None => return None,
    };

    if (c | 0x20) == b'i' {
        inp.unget(c);
        if !try_word(inp, b"inf") {
            return None;
        }
        // Either "inf" stands alone or the full "infinity" must follow: glibc's
        // scanf cannot push back a partially matched suffix, so "infi..." that
        // does not complete the word is a matching failure.
        match inp.next() {
            Some(n) if (n | 0x20) == b'i' => {
                if !try_word(inp, b"nity") {
                    return None;
                }
            }
            Some(n) => inp.unget(n),
            None => {}
        }
        return Some(sign | INF_BITS);
    }

    if (c | 0x20) == b'n' {
        inp.unget(c);
        return scan_nan(inp).map(|b| sign | b);
    }

    if c == b'0' {
        match inp.next() {
            Some(x) if (x | 0x20) == b'x' => {
                return scan_hex(inp).map(|b| sign | b);
            }
            Some(other) => inp.unget(other),
            None => {}
        }
        return scan_decimal(inp, Some(b'0')).map(|b| sign | b);
    }

    inp.unget(c);
    scan_decimal(inp, None).map(|b| sign | b)
}

// ---------------------------------------------------------------------------
// Translation of the C program.
// ---------------------------------------------------------------------------
fn print_hex(out: &mut impl Write, p: &[u8]) {
    for &b in p {
        let _ = write!(out, "{:02x}", b);
    }
    let _ = writeln!(out);
}

fn driver(out: &mut impl Write, x: f32) {
    print_hex(out, &x.to_ne_bytes());
}

fn main() {
    let mut x: f32 = 0.0;

    let mut inp = Input::new();
    if let Some(bits) = scan_float_bits(&mut inp) {
        x = f32::from_bits(bits);
    }

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    driver(&mut out, x);
    let _ = out.flush();
}
