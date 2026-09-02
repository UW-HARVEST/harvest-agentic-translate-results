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

/// C's `int` limits, used by the range check in `parse_val`.
const INT_MIN: i64 = -2_147_483_648;
const INT_MAX: i64 = 2_147_483_647;

/// Mirrors `typedef struct { int floors; int bedrooms; double bathrooms; } house_t;`
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

/// static house_t the_house = {.floors = 2, .bedrooms = 5, .bathrooms = 2.5};
static mut THE_HOUSE: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

fn add_floor(house: &mut House) {
    // C: house->floors++ (wraps on the target compilers)
    house.floors = house.floors.wrapping_add(1);
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    // C: house->bedrooms += extra_bedrooms (wraps on the target compilers)
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

/// Safe accessor for the single global instance. The program is single
/// threaded, exactly like the C original.
fn with_house<R>(f: impl FnOnce(&mut House) -> R) -> R {
    // SAFETY: single-threaded program; no other borrow of THE_HOUSE is live.
    unsafe { f(&mut *std::ptr::addr_of_mut!(THE_HOUSE)) }
}

fn add_floor_to_the_house() {
    with_house(add_floor);
}

fn print_the_house(out: &mut impl Write) {
    let (floors, bedrooms, bathrooms) = with_house(|h| (h.floors, h.bedrooms, h.bathrooms));
    // printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...)
    let _ = write!(
        out,
        "The house has {} floors, {} bedrooms, and {} bathrooms\n",
        floors,
        bedrooms,
        format_f64_1(bathrooms)
    );
}

/// Formats a double the way glibc's `%.1f` does.
fn format_f64_1(v: f64) -> String {
    if v.is_nan() {
        // glibc prints "nan" / "-nan"
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
    // Rust prints "-0.0" for negative zero, as does glibc's %.1f.
    s
}

fn run(out: &mut impl Write, extra_bedrooms: i32) {
    print_the_house(out);
    add_floor_to_the_house();
    print_the_house(out);
    with_house(|h| h.bathrooms += 1.0);
    print_the_house(out);
    with_house(|h| add_bedrooms(h, extra_bedrooms));
    print_the_house(out);
}

/// Result of the `strtol` emulation: (value, index just past the consumed
/// characters, whether ERANGE was raised).
fn strtol_base10(s: &[u8]) -> (i64, usize, bool) {
    let mut i = 0usize;

    // strtol skips leading white space (C locale).
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }

    // Optional sign.
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    let limit: u64 = if negative {
        9_223_372_036_854_775_808 // -LONG_MIN
    } else {
        9_223_372_036_854_775_807 // LONG_MAX
    };
    let mut acc: u64 = 0;
    let mut erange = false;

    while i < s.len() && s[i].is_ascii_digit() {
        let d = u64::from(s[i] - b'0');
        if !erange {
            match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) if v <= limit => acc = v,
                _ => erange = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No conversion performed: endptr is set back to the start of the
        // input string and 0 is returned.
        return (0, 0, false);
    }

    let value = if erange {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -(acc as i128) as i64
    } else {
        acc as i64
    };

    (value, i, erange)
}

/// static bool parse_val(const char *str, int *val)
fn parse_val(str_bytes: &[u8], val: &mut i32) -> bool {
    // errno = 0;
    // char *endp = (char *)str;
    // long tmp = strtol(str, &endp, 10);
    let (tmp, end_off, erange) = strtol_base10(str_bytes);
    let errno_is_zero = !erange;

    if end_off != 0 && errno_is_zero && tmp >= INT_MIN && tmp <= INT_MAX {
        *val = tmp as i32;
        true
    } else {
        false
    }
}

/// fgets(in, sizeof(in), stdin) over a 100-byte buffer: reads at most 99
/// bytes, stops after a newline (which is kept), and NUL-terminates. On
/// immediate EOF the buffer is left untouched.
fn fgets(buf: &mut [u8; 100]) {
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let mut scratch = [0u8; 1];
    let mut n = 0usize;
    while n < buf.len() - 1 {
        match handle.read(&mut scratch) {
            Ok(0) => break,              // EOF
            Err(_) => break,             // read error: fgets returns NULL
            Ok(_) => {
                buf[n] = scratch[0];
                n += 1;
                if scratch[0] == b'\n' {
                    break;
                }
            }
        }
    }
    if n > 0 {
        buf[n] = 0;
    }
}

/// Bytes of the C string held in `buf` (up to the first NUL).
fn c_str(buf: &[u8]) -> &[u8] {
    match buf.iter().position(|&b| b == 0) {
        Some(p) => &buf[..p],
        None => buf,
    }
}

/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs; a C program
/// started from a shell inherits `SIG_DFL`. Without this, a write to a closed
/// pipe makes the C program die with SIGPIPE (status 141) while the Rust
/// program would merely see `EPIPE` and exit 0. Restore the C behaviour.
#[cfg(unix)]
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    // SAFETY: `signal(2)` with SIG_DFL is always valid; called once, before any
    // other thread exists.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

fn main() {
    restore_default_sigpipe();

    // char in[100] = "";
    let mut input = [0u8; 100];
    fgets(&mut input);

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    let mut x: i32 = 0;
    if parse_val(c_str(&input), &mut x) {
        run(&mut out, x);
        run(&mut out, x);
    } else {
        let _ = write!(out, "An error occurred\n");
    }

    let _ = out.flush();
    std::process::exit(0);
}
