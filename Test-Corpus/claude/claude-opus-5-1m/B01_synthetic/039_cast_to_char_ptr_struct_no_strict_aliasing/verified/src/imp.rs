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

// Faithful Rust translation of `c_src/src/main.c`.
//
// The C program reads a single integer with `scanf("%d", &x)` and then dumps the
// raw little-endian byte image of a `house_t` struct as lowercase hex.
//
// This module is shared by two crate targets:
//   * `src/lib.rs`   -> `cdylib`, exporting the same C ABI symbols as the C
//                       shared library build of `c_src/src/main.c`
//                       (`driver` and `main`).
//   * `src/main.rs`  -> the executable (`#![no_main]`, so the `#[no_mangle]`
//                       `main` below *is* the program entry point, exactly like
//                       the C `int main()`).

// Under `cfg(test)` (e.g. `cargo build --all-targets`, which forces a libtest
// harness for the lib/bin targets) libtest provides the process entry point, so
// the exported `main` at the bottom of this file is compiled out and a few
// helpers are then unused.
#![cfg_attr(test, allow(dead_code))]

use std::io::{Read, Write};
use std::os::raw::c_int;

// Mirror of the C `house_t`:
//
//     typedef struct {
//         int floors;       // offset 0, 4 bytes
//         int bedrooms;     // offset 4, 4 bytes
//         double bathrooms; // offset 8, 8 bytes
//     } house_t;            // sizeof == 16, alignof == 8 (System V x86-64)
#[repr(C)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

// `sizeof(house_t)` on the LP64 x86-64 ABI targeted by the original program.
const HOUSE_SIZE: usize = std::mem::size_of::<House>();

impl House {
    // Produce the exact `memcpy` image of the struct (System V x86-64, little
    // endian). The C code zero-initializes the struct with `= {0}` before
    // assigning every field, so any padding bytes are deterministically zero.
    // (Here there is in fact no padding: 4 + 4 + 8 == 16.)
    fn to_raw_bytes(&self) -> [u8; HOUSE_SIZE] {
        let mut raw = [0u8; HOUSE_SIZE];
        raw[0..4].copy_from_slice(&self.floors.to_le_bytes());
        raw[4..8].copy_from_slice(&self.bedrooms.to_le_bytes());
        raw[8..16].copy_from_slice(&self.bathrooms.to_le_bytes());
        raw
    }
}

// Equivalent of the C `static void print_hex(unsigned char *p, int len)`:
// `printf("%02x", p[i])` per byte, then `printf("\n")`.
//
// `len` is an `int` in C and the loop is `for (i = 0; i < len; i++)`, so a
// non-positive length prints nothing but the trailing newline. The single
// caller always passes `sizeof(raw) == 16`.
fn print_hex(out: &mut dyn Write, p: &[u8], len: i32) {
    let mut line = String::with_capacity(p.len() * 2 + 1);
    let count = if len < 0 { 0 } else { len as usize };
    for &b in p.iter().take(count) {
        // "%02x" -> lowercase hex, zero padded to two digits.
        line.push(char::from_digit(u32::from(b >> 4), 16).unwrap());
        line.push(char::from_digit(u32::from(b & 0x0f), 16).unwrap());
    }
    line.push('\n');
    let _ = out.write_all(line.as_bytes());
}

// Equivalent of the C `void driver(int floors)`.
fn driver_impl(floors: i32) {
    let house = House {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };
    // `char raw[sizeof(house)]; memcpy(raw, &house, sizeof(house));`
    let raw = house.to_raw_bytes();

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    print_hex(&mut out, &raw, HOUSE_SIZE as i32);
    // C's `stdout` is flushed by `exit()`; flush eagerly so that the byte
    // stream is identical no matter how this entry point is invoked (the
    // shared-library export may be called many times in one process).
    let _ = out.flush();
}

// True for the characters `isspace()` accepts in the C locale; `scanf`
// conversions skip these before a `%d` directive.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

// Reads stdin one byte at a time, mirroring `getc()` on the C `stdin` stream:
// `scanf` only consumes as many characters as the conversion needs (plus the
// single lookahead character that terminates the digit run), so the program
// must not block waiting for EOF.
fn next_byte(input: &mut dyn Read) -> Option<u8> {
    let mut b = [0u8; 1];
    loop {
        match input.read(&mut b) {
            Ok(0) => return None, // EOF
            Ok(_) => return Some(b[0]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return None, // read error behaves like EOF for scanf
        }
    }
}

// Emulates a single `scanf("%d", &x)` directive.
//
// Returns `Some(value)` when the conversion succeeds and `None` on matching
// failure or input failure/EOF (in which case the C variable keeps its previous
// value, i.e. the `int x = 0;` initializer).
//
// Overflow reproduces glibc's behaviour: `__vfscanf_internal` hands the digit
// run to `strtol`, which saturates at `LONG_MAX` / `LONG_MIN` and sets ERANGE;
// scanf ignores ERANGE and stores the saturated `long` into an `int`, which
// truncates to the low 32 bits. That is why the C program prints `ffffffff`
// for `99999999999999999999`.
fn scanf_i32(input: &mut dyn Read) -> Option<i32> {
    // Skip leading whitespace (this may cross newlines, matching scanf).
    let mut c = loop {
        match next_byte(input) {
            None => return None, // input failure (EOF)
            Some(b) if is_c_space(b) => continue,
            Some(b) => break b,
        }
    };

    // Optional sign.
    let mut negative = false;
    match c {
        b'-' => {
            negative = true;
            c = match next_byte(input) {
                Some(b) => b,
                None => return None, // sign then EOF: matching failure
            };
        }
        b'+' => {
            c = match next_byte(input) {
                Some(b) => b,
                None => return None,
            };
        }
        _ => {}
    }

    // At least one decimal digit is required.
    if !c.is_ascii_digit() {
        return None; // matching failure
    }

    let mut acc: i64 = 0;
    let mut overflow = false;
    loop {
        let digit = i64::from(c - b'0');
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        match next_byte(input) {
            Some(b) if b.is_ascii_digit() => c = b,
            // The terminating character (or EOF) ends the conversion. The C
            // program exits immediately afterwards, so pushing it back onto the
            // stream would not be observable.
            _ => break,
        }
    }

    let as_long: i64 = if overflow {
        // glibc: strtol saturates and sets ERANGE.
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

    // Storing a `long` into an `int` truncates to the low 32 bits.
    Some(as_long as i32)
}

// Equivalent of the C
//
//     int main() {
//         int x = 0;
//         scanf("%d", &x);
//         driver(x);
//         return 0;
//     }
fn main_impl() -> c_int {
    let mut x: i32 = 0;
    {
        let stdin = std::io::stdin();
        let mut input = stdin.lock();
        if let Some(v) = scanf_i32(&mut input) {
            x = v;
        }
    }
    driver_impl(x);
    0
}

// ---------------------------------------------------------------------------
// C ABI exports (the symbols the C shared-library build of main.c provides).
// ---------------------------------------------------------------------------

/// `void driver(int floors)`
#[no_mangle]
pub extern "C" fn driver(floors: c_int) {
    driver_impl(floors);
}

/// `int main()`
///
/// Exported for symbol parity with the C shared library, and used directly as
/// the process entry point by the `#![no_main]` executable target.
///
/// Omitted in `cfg(test)` builds only, where libtest defines its own `main`.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> c_int {
    main_impl()
}
