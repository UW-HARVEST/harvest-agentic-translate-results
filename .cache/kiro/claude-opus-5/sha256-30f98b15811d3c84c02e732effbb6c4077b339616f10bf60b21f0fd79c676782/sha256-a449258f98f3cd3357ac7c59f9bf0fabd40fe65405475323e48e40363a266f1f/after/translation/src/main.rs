// Rust translation of c_src/src/main.c
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

/// Stand-in for C's `stdout` FILE stream.
///
/// C's `stdout` is fully buffered when it is not a terminal (BUFSIZ = 8192 on
/// glibc) and this program emits at most 8 short lines, so nothing is ever
/// flushed before `exit`. Just as importantly, `printf` in the C code has its
/// return value discarded: a failing write (EPIPE, ENOSPC, closed fd, ...) is
/// silently ignored and `main` still returns 0. Rust's `print!` macro instead
/// *panics* on a write error, so it cannot be used here.
struct COut {
    buf: Vec<u8>,
}

impl COut {
    fn new() -> Self {
        COut { buf: Vec::new() }
    }

    /// Equivalent of `printf(...)` with the result discarded.
    fn put(&mut self, s: &str) {
        self.buf.extend_from_slice(s.as_bytes());
    }

    /// Equivalent of the implicit stream flush performed at `exit`. All errors
    /// are swallowed, matching C, where a failed flush at exit does not change
    /// the status already returned by `main`.
    fn flush_at_exit(&mut self) {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.write_all(&self.buf);
        let _ = handle.flush();
        self.buf.clear();
    }
}

/// Mirrors `house_t` from the C source.
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn add_floor(house: &mut House) {
    // C: house->floors++ (wraps in practice on overflow)
    house.floors = house.floors.wrapping_add(1);
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    // C: house->bedrooms += extra_bedrooms (wraps in practice on overflow)
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

fn print_house(out: &mut COut, house: &House) {
    // C: printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...)
    out.put(&format!(
        "The house has {} floors, {} bedrooms, and {} bathrooms\n",
        house.floors,
        house.bedrooms,
        format_f1(house.bathrooms)
    ));
}

/// Formats a double the way C's `%.1f` does.
fn format_f1(v: f64) -> String {
    if v.is_nan() {
        // glibc prints "nan" / "-nan" depending on the sign bit.
        return if v.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if v.is_infinite() {
        return if v < 0.0 {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    let s = format!("{:.1}", v);
    // Rust prints "-0.0" for negative zero, as does glibc's %.1f; both agree.
    s
}

fn run(out: &mut COut, the_house: &mut House, extra_bedrooms: i32) {
    print_house(out, the_house);
    add_floor(the_house);
    print_house(out, the_house);
    the_house.bathrooms += 1.0;
    print_house(out, the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_house(out, the_house);
}

/// Result of a `strtol(str, &endp, 10)` call on a 64-bit `long` platform.
struct StrtolResult {
    value: i64,
    /// Offset of `endp` relative to the start of the string. `0` means no
    /// conversion was performed (i.e. `endp == str`).
    end_offset: usize,
    /// Whether ERANGE would have been stored in `errno`.
    erange: bool,
}

fn c_isspace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Faithful emulation of `strtol(s, &endp, 10)` for a NUL-terminated C string
/// whose bytes (excluding the terminator) are given by `s`.
fn strtol_base10(s: &[u8]) -> StrtolResult {
    let mut i = 0usize;

    // Skip leading whitespace.
    while i < s.len() && c_isspace(s[i]) {
        i += 1;
    }

    // Optional sign.
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    let mut acc: u128 = 0;
    let mut overflow = false;
    // Magnitude limits for a 64-bit long.
    let limit: u128 = if negative {
        i64::MAX as u128 + 1
    } else {
        i64::MAX as u128
    };

    while i < s.len() && s[i].is_ascii_digit() {
        let d = u128::from(s[i] - b'0');
        if !overflow {
            acc = acc * 10 + d;
            if acc > limit {
                overflow = true;
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No conversion performed: strtol sets *endptr = str and returns 0.
        return StrtolResult {
            value: 0,
            end_offset: 0,
            erange: false,
        };
    }

    let value = if overflow {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        // acc <= 2^63, so this negation is exact.
        (acc as i128).wrapping_neg() as i64
    } else {
        acc as i64
    };

    StrtolResult {
        value,
        end_offset: i,
        erange: overflow,
    }
}

/// Mirrors `parse_val` from the C source.
fn parse_val(s: &[u8], val: &mut i32) -> bool {
    // errno = 0;
    let r = strtol_base10(s);
    // if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX)
    if r.end_offset != 0
        && !r.erange
        && r.value >= i64::from(i32::MIN)
        && r.value <= i64::from(i32::MAX)
    {
        *val = r.value as i32;
        true
    } else {
        false
    }
}

/// Mirrors `fgets(in, size, stdin)` where `in` is a `char[size]` pre-filled
/// with NUL bytes. Returns the resulting C string contents (bytes up to, but
/// not including, the terminating NUL).
fn fgets_stdin(size: usize) -> Vec<u8> {
    let max = size - 1; // room for the terminating NUL
    let mut out: Vec<u8> = Vec::new();
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];
    while out.len() < max {
        match handle.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                out.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    // The C buffer is NUL-terminated, so the effective string stops at the
    // first embedded NUL byte (if any). On fgets failure the buffer keeps its
    // initial "" value, which is the same as an empty result here.
    let end = out.iter().position(|&b| b == 0).unwrap_or(out.len());
    out.truncate(end);
    out
}

/// Restores the default disposition of `SIGPIPE`.
///
/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, which a C
/// program does not do. Without this, writing to a closed pipe makes the C
/// binary die from signal 13 while the Rust binary would quietly exit 0.
#[cfg(unix)]
fn reset_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    // Safety: `signal` with `SIG_DFL` is async-signal-safe and simply restores
    // the kernel default for this one signal.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn reset_sigpipe() {}

fn main() {
    reset_sigpipe();
    let mut out = COut::new();
    let input = fgets_stdin(100);
    let mut x: i32 = 0;
    if parse_val(&input, &mut x) {
        let mut the_house = House {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut out, &mut the_house, x);
        run(&mut out, &mut the_house, x);
    } else {
        out.put("An error occurred\n");
    }
    // C: `return 0;` from main, which flushes stdout on exit.
    out.flush_at_exit();
    std::process::exit(0);
}
