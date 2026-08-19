// Rust translation of c_src/src/main.c (shared implementation).
//
// This module is compiled into BOTH the `driver` binary (src/main.rs) and the
// `driver` cdylib (src/lib.rs) so that the executable and the exported C ABI
// surface can never drift apart.
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

use std::io::{BufRead, Write};

#[derive(Clone, Copy)]
pub struct House {
    pub floors: i32,
    pub bedrooms: i32,
    pub bathrooms: f64,
}

/// Mirrors the C file-scope `static house_t the_house`.
static mut THE_HOUSE: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

fn the_house() -> &'static mut House {
    // Single-threaded program; equivalent to C's access of the global object.
    unsafe { &mut *std::ptr::addr_of_mut!(THE_HOUSE) }
}

fn add_floor(house: &mut House) {
    // C: house->floors++
    house.floors = house.floors.wrapping_add(1);
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    // C: house->bedrooms += extra_bedrooms
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

fn add_floor_to_the_house() {
    add_floor(the_house());
}

fn print_the_house(out: &mut dyn Write) {
    let h = *the_house();
    // printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...)
    let _ = write!(
        out,
        "The house has {} floors, {} bedrooms, and {} bathrooms\n",
        h.floors,
        h.bedrooms,
        format_f64_1(h.bathrooms)
    );
}

/// Formats a double the way C's `%.1f` does.
pub fn format_f64_1(v: f64) -> String {
    if v.is_nan() {
        // C prints "nan" / "-nan"
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
    // Rust's {:.1} matches glibc's round-half-to-even on the exact binary value.
    format!("{:.1}", v)
}

/// C: `void run(int extra_bedrooms)`
pub fn run(extra_bedrooms: i32) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    run_to(&mut out, extra_bedrooms);
    let _ = out.flush();
}

/// The body of `run`, writing to an arbitrary sink (used by the tests and by
/// `run` itself).
pub fn run_to(out: &mut dyn Write, extra_bedrooms: i32) {
    print_the_house(out);
    add_floor_to_the_house();
    print_the_house(out);
    the_house().bathrooms += 1.0;
    print_the_house(out);
    add_bedrooms(the_house(), extra_bedrooms);
    print_the_house(out);
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

fn push_digit(acc: &mut i64, overflow: &mut bool, byte: u8) {
    let d = i64::from(byte - b'0');
    if !*overflow {
        match acc.checked_mul(10).and_then(|a| a.checked_add(d)) {
            Some(next) => *acc = next,
            None => *overflow = true,
        }
    }
}

fn finish_i32(acc: i64, overflow: bool, negative: bool) -> i32 {
    // glibc parses with strtol and stores the (possibly saturated) long
    // truncated to int.
    let value: i64 = if overflow {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        acc.wrapping_neg()
    } else {
        acc
    };
    value as i32
}

/// Emulates `scanf("%d", &x)` against a buffered reader, consuming only as many
/// bytes as the conversion needs (like C's `stdio`, which never blocks waiting
/// for more input than the conversion requires).
///
/// Returns the converted value (`None` on matching/input failure) together with
/// the number of bytes the conversion *logically* consumed. The byte that
/// terminates the conversion is pushed back by `scanf` (`ungetc`), so it is not
/// counted — glibc restores that logical position on the underlying descriptor
/// when the stream is seekable, see `sync_stdin_position`.
pub fn scanf_i32_reader(input: &mut dyn BufRead) -> (Option<i32>, u64) {
    let mut consumed: u64 = 0;

    // Skip whitespace.
    let mut cur = loop {
        match next_byte(input) {
            Some(b) if is_c_space(b) => {
                consumed += 1;
                continue;
            }
            other => break other,
        }
    };

    let mut negative = false;
    if cur == Some(b'+') || cur == Some(b'-') {
        negative = cur == Some(b'-');
        consumed += 1;
        cur = next_byte(input);
    }

    let mut acc: i64 = 0;
    let mut overflow = false;
    let mut digits = 0usize;
    while let Some(b) = cur {
        if !b.is_ascii_digit() {
            break;
        }
        push_digit(&mut acc, &mut overflow, b);
        digits += 1;
        consumed += 1;
        cur = next_byte(input);
    }

    if digits == 0 {
        // Matching failure (or EOF): nothing is stored.
        return (None, consumed);
    }

    (Some(finish_i32(acc, overflow, negative)), consumed)
}

fn next_byte(input: &mut dyn BufRead) -> Option<u8> {
    loop {
        let (byte, empty) = match input.fill_buf() {
            Ok(buf) => {
                if buf.is_empty() {
                    (None, true)
                } else {
                    (Some(buf[0]), false)
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => (None, true),
        };
        if empty {
            return None;
        }
        input.consume(1);
        return byte;
    }
}

/// C: `int main()`
pub fn c_main() -> i32 {
    restore_default_sigpipe();

    let stdin = std::io::stdin();
    // Like glibc's `stdio`, this reads whatever a single `read` returns instead
    // of draining the stream, so a program that keeps its stdin open (or never
    // closes it) terminates exactly when the C version does. A `read_to_end`
    // here would not.
    let mut reader = stdin.lock();

    let start = stdin_offset();
    let mut x: i32 = 0;
    let (parsed, consumed) = scanf_i32_reader(&mut reader);
    if let Some(v) = parsed {
        x = v;
    }
    drop(reader);
    sync_stdin_position(start, consumed);

    run(x);
    run(x);
    let _ = std::io::stdout().flush();
    0
}

#[cfg(unix)]
extern "C" {
    fn lseek(fd: i32, offset: i64, whence: i32) -> i64;
}

/// `lseek(0, 0, SEEK_CUR)`; `None` when stdin is not seekable (pipe, tty) or
/// not open.
fn stdin_offset() -> Option<i64> {
    #[cfg(unix)]
    {
        const SEEK_CUR: i32 = 1;
        let pos = unsafe { lseek(0, 0, SEEK_CUR) };
        if pos < 0 {
            None
        } else {
            Some(pos)
        }
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// When a C program exits, `stdio` gives back the bytes it read ahead into its
/// buffer by seeking a seekable stream to the position the program logically
/// consumed. Reproduce that so a caller like `{ driver; cat; } < file` observes
/// exactly the same remaining input.
fn sync_stdin_position(start: Option<i64>, consumed: u64) {
    #[cfg(unix)]
    if let Some(start) = start {
        const SEEK_SET: i32 = 0;
        let target = start.saturating_add(consumed as i64);
        unsafe {
            lseek(0, target, SEEK_SET);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (start, consumed);
    }
}

/// The Rust runtime sets `SIGPIPE` to `SIG_IGN`; a C program leaves it at the
/// default disposition, so restore it to keep the observable exit status
/// identical when stdout is a closed pipe.
fn restore_default_sigpipe() {
    #[cfg(unix)]
    unsafe {
        extern "C" {
            fn signal(signum: i32, handler: usize) -> usize;
        }
        const SIGPIPE: i32 = 13;
        const SIG_DFL: usize = 0;
        signal(SIGPIPE, SIG_DFL);
    }
}
