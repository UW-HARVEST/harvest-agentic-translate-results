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

//! Emulation of the small slice of the C runtime that `main.c` depends on:
//! `fgets`, `atof` (i.e. `strtod`), buffered `printf` output, the default
//! `SIGPIPE` disposition, and C's float-to-`int` conversion as it behaves on
//! x86-64.

use std::io::{self, BufRead, Write};
use std::sync::{Mutex, OnceLock};

extern "C" {
    fn isatty(fd: i32) -> i32;
    fn signal(signum: i32, handler: usize) -> usize;
}

const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;

/// glibc's default `stdout` buffer size when the stream is not a terminal.
const BUFSIZ: usize = 8192;

/// Restores the default `SIGPIPE` disposition.
///
/// Rust's runtime sets `SIGPIPE` to `SIG_IGN` before `main`, so a failed write
/// to a closed pipe surfaces as `EPIPE` and makes `println!` panic (exit 134
/// plus a panic message on stderr). A C program inherits the default, so it is
/// killed by the signal (exit status 141, nothing on stderr). Reset it so the
/// two binaries agree.
pub fn reset_sigpipe() {
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

/// The emulated `stdout` buffer.
static STDOUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// Whether `stdout` is line buffered, i.e. whether it refers to a terminal.
/// glibc decides this once, when the stream is first used.
fn line_buffered() -> bool {
    static LINE_BUFFERED: OnceLock<bool> = OnceLock::new();
    *LINE_BUFFERED.get_or_init(|| unsafe { isatty(1) == 1 })
}

/// Writes the buffer out, discarding write errors exactly as the C program
/// ignores `printf`'s return value. (With the default `SIGPIPE` restored, a
/// broken pipe kills the process here rather than returning an error.)
fn write_out(buf: &mut Vec<u8>) {
    if buf.is_empty() {
        return;
    }
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let _ = stdout.write_all(buf);
    let _ = stdout.flush();
    buf.clear();
}

/// `printf("%s\n", ...)`-style output into the emulated `stdout` stream.
pub fn c_print_line(bytes: &[u8]) {
    let mut buf = STDOUT.lock().unwrap_or_else(|e| e.into_inner());
    buf.extend_from_slice(bytes);
    buf.push(b'\n');
    if line_buffered() || buf.len() >= BUFSIZ {
        write_out(&mut buf);
    }
}

/// Flushes `stdout`, as C does when `main` returns / `exit` is called.
pub fn flush_stdout() {
    let mut buf = STDOUT.lock().unwrap_or_else(|e| e.into_inner());
    write_out(&mut buf);
}

/// `fgets(buf, size, stdin)`.
///
/// Reads at most `size - 1` bytes from stdin, stopping early after a newline
/// (which is retained). Returns `None` exactly when C `fgets` returns `NULL`,
/// i.e. when end-of-file (or an error) is hit before any byte is stored.
/// Bytes not consumed stay in stdin's buffer for the next call, so a line
/// longer than `size - 1` is picked up piecewise by subsequent calls -- unlike
/// `scanf`, this never reads across a newline.
pub fn fgets(size: usize) -> Option<Vec<u8>> {
    if size == 0 {
        return None;
    }
    let capacity = size - 1;
    let mut out: Vec<u8> = Vec::new();
    let stdin = io::stdin();
    // `Stdin`'s buffer lives in a global mutex, so it persists across the
    // separate `lock()` calls made by separate `fgets` invocations.
    let mut input = stdin.lock();

    while out.len() < capacity {
        let available = match input.fill_buf() {
            Ok(buf) => buf,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        };
        if available.is_empty() {
            break; // end of file
        }
        let take = (capacity - out.len()).min(available.len());
        let chunk = &available[..take];
        match chunk.iter().position(|&b| b == b'\n') {
            Some(pos) => {
                out.extend_from_slice(&chunk[..=pos]);
                let consumed = pos + 1;
                input.consume(consumed);
                return Some(out);
            }
            None => {
                out.extend_from_slice(chunk);
                input.consume(take);
            }
        }
    }

    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// `atof(s)` == `strtod(s, NULL)`, ignoring `errno`.
///
/// `bytes` is the raw `fgets` buffer; as a C string it ends at the first NUL.
pub fn atof(bytes: &[u8]) -> f64 {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    strtod(&bytes[..end])
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Scans the longest valid numeric prefix the same way glibc's `strtod` does
/// and returns its value; yields `0.0` when no conversion can be performed.
fn strtod(s: &[u8]) -> f64 {
    let mut i = 0usize;
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    let negative = match s.get(i) {
        Some(b'+') => {
            i += 1;
            false
        }
        Some(b'-') => {
            i += 1;
            true
        }
        _ => false,
    };

    let rest = &s[i..];
    let magnitude = if starts_with_ignore_ascii_case(rest, b"inf") {
        f64::INFINITY
    } else if starts_with_ignore_ascii_case(rest, b"nan") {
        f64::NAN
    } else if rest.len() >= 2 && rest[0] == b'0' && (rest[1] | 0x20) == b'x' {
        // Hexadecimal form; falls back to the plain "0" if no hex digits follow.
        match parse_hex(&rest[2..]) {
            Some(value) => value,
            None => 0.0,
        }
    } else {
        parse_decimal(rest)
    };

    if negative {
        -magnitude
    } else {
        magnitude
    }
}

fn starts_with_ignore_ascii_case(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.len() >= needle.len()
        && haystack[..needle.len()].eq_ignore_ascii_case(needle)
}

/// Parses `[digits][.[digits]][(e|E)[sign]digits]`, returning `0.0` when the
/// prefix holds no digits at all (the "no conversion" case).
fn parse_decimal(s: &[u8]) -> f64 {
    let mut i = 0usize;
    let int_start = i;
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    let int_digits = i - int_start;

    let mut frac_digits = 0usize;
    let mut significand_end = i;
    if i < s.len() && s[i] == b'.' {
        let mut j = i + 1;
        while j < s.len() && s[j].is_ascii_digit() {
            j += 1;
        }
        frac_digits = j - (i + 1);
        if int_digits > 0 || frac_digits > 0 {
            i = j;
            significand_end = j;
        }
    }
    if int_digits == 0 && frac_digits == 0 {
        return 0.0; // no conversion
    }

    // An exponent only counts if it is complete: e/E, optional sign, >=1 digit.
    let mut number_end = significand_end;
    if i < s.len() && (s[i] | 0x20) == b'e' {
        let mut j = i + 1;
        if j < s.len() && (s[j] == b'+' || s[j] == b'-') {
            j += 1;
        }
        let digits_start = j;
        while j < s.len() && s[j].is_ascii_digit() {
            j += 1;
        }
        if j > digits_start {
            number_end = j;
        }
    }

    // Safe: the slice contains only ASCII from the grammar above.
    let text = std::str::from_utf8(&s[..number_end]).unwrap_or("0");
    text.parse::<f64>().unwrap_or(0.0)
}

/// Parses the part after `0x`: `[hexdigits][.[hexdigits]][(p|P)[sign]digits]`.
/// Returns `None` when there are no hex digits, so the caller can fall back to
/// reading just the leading `0`, exactly as `strtod` does.
fn parse_hex(s: &[u8]) -> Option<f64> {
    // Significand accumulated as an integer, with `dropped_exp` counting bits
    // shifted off the right and `sticky` recording whether any of them was set.
    let mut mantissa: u128 = 0;
    let mut dropped_exp: i32 = 0;
    let mut sticky = false;
    let mut digit_count = 0usize;
    let mut frac_digits: i32 = 0;
    let mut i = 0usize;
    let mut seen_point = false;

    while i < s.len() {
        let b = s[i];
        if b == b'.' {
            if seen_point {
                break;
            }
            seen_point = true;
            i += 1;
            continue;
        }
        let digit = match (b as char).to_digit(16) {
            Some(d) => d as u128,
            None => break,
        };
        digit_count += 1;
        if seen_point {
            frac_digits += 1;
        }
        // 2^124 headroom keeps `mantissa * 16 + digit` from overflowing.
        if mantissa < (1u128 << 124) {
            mantissa = mantissa * 16 + digit;
        } else {
            dropped_exp += 4;
            if digit != 0 {
                sticky = true;
            }
        }
        i += 1;
    }
    if digit_count == 0 {
        return None;
    }

    let mut exponent: i64 = dropped_exp as i64 - 4 * frac_digits as i64;

    if i < s.len() && (s[i] | 0x20) == b'p' {
        let mut j = i + 1;
        let mut negative = false;
        if j < s.len() && (s[j] == b'+' || s[j] == b'-') {
            negative = s[j] == b'-';
            j += 1;
        }
        let digits_start = j;
        let mut value: i64 = 0;
        while j < s.len() && s[j].is_ascii_digit() {
            // Clamp instead of overflowing; the result saturates anyway.
            value = (value * 10 + (s[j] - b'0') as i64).min(1 << 40);
            j += 1;
        }
        if j > digits_start {
            exponent += if negative { -value } else { value };
        }
    }

    Some(scale_to_f64(mantissa, sticky, exponent))
}

/// Rounds `(mantissa + fraction) * 2^exponent` to the nearest `f64`
/// (ties to even), where `sticky` says the unrepresented `fraction` is nonzero.
fn scale_to_f64(mantissa: u128, sticky: bool, exponent: i64) -> f64 {
    if mantissa == 0 {
        return 0.0;
    }
    let bits = 128 - mantissa.leading_zeros() as i64;
    let unbiased = exponent + bits - 1;

    // Number of low bits to discard to land on the f64 significand grid.
    let mut drop = bits - 53;
    if unbiased < -1022 {
        drop = -1074 - exponent; // subnormal: absolute grid at 2^-1074
    }
    if drop <= 0 {
        // Exactly representable; any sticky remainder is below half an ulp.
        return ldexp(mantissa as f64, exponent);
    }
    if drop >= 128 {
        // Everything rounds away to (signed) zero for a subnormal underflow.
        return 0.0;
    }

    let drop = drop as u32;
    let quotient = mantissa >> drop;
    let half = 1u128 << (drop - 1);
    let discarded = mantissa & ((1u128 << drop) - 1);
    let guard = (discarded & half) != 0;
    let below = (discarded & (half - 1)) != 0 || sticky;

    let rounded = if guard && (below || (quotient & 1) == 1) {
        quotient + 1
    } else {
        quotient
    };
    ldexp(rounded as f64, exponent + drop as i64)
}

/// `ldexp(x, n)`: multiplies by 2^n, stepping to avoid intermediate overflow.
fn ldexp(x: f64, n: i64) -> f64 {
    let mut value = x;
    let mut n = n;
    while n > 1023 {
        value *= f64::from_bits(0x7FE0_0000_0000_0000); // 2^1023
        if !value.is_finite() || value == 0.0 {
            return value;
        }
        n -= 1023;
    }
    while n < -1022 {
        value *= f64::from_bits(0x0010_0000_0000_0000); // 2^-1022
        if value == 0.0 {
            return value;
        }
        n += 1022;
    }
    value * f64::from_bits(((n + 1023) as u64) << 52)
}

/// C's `(int)` conversion from a floating-point value.
///
/// Values that are NaN or out of range are undefined behaviour in C; this
/// reproduces what x86-64 actually does (`cvttsd2si` yields `INT_MIN`), which
/// is what the reference build of `main.c` prints.
pub fn f64_to_int(value: f64) -> i32 {
    if value.is_nan() {
        return i32::MIN;
    }
    let truncated = value.trunc();
    if truncated >= 2147483648.0 || truncated < -2147483648.0 {
        return i32::MIN;
    }
    truncated as i32
}
