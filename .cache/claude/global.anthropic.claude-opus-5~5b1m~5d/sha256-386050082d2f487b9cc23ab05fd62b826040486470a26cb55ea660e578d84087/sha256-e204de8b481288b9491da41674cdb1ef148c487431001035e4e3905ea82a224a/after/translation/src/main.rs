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

use std::cell::Cell;
use std::io::{Read, Write};

// ---------------------------------------------------------------------------
// Emulation of the C program's uninitialized-pointer read (CWE-457/824).
//
// `bad()` in the C source declares `int *data;` and immediately dereferences it
// without initializing it.  That is undefined behavior, but the behavior of the
// reference build (the CMake default, i.e. unoptimized) is deterministic: the
// stack slot that `data` occupies in `bad()`'s frame still holds the leftover
// pointer that `scanf("%d", &x)` placed there in `main()`'s call, namely the
// address of `main`'s `x`.  Dereferencing therefore prints the current value of
// `x` -- which on this code path is always 0, because `bad()` is only reached
// when `x` is false.
//
// To reproduce the reference build's bytes exactly (and without any unsafe
// code), the stale stack slot is modeled explicitly: `main` publishes the
// storage that `&x` referred to, and `bad()` reads through it.
// ---------------------------------------------------------------------------
thread_local! {
    /// The integer object that the stale, "uninitialized" pointer in `bad()`
    /// happens to point at in the reference build: `main`'s local `x`.
    static STALE_POINTEE: Cell<i32> = const { Cell::new(0) };
}

/// `void printIntPtrLine(const int *intNumber) { printf("%d\n", *intNumber); }`
fn print_int_ptr_line(int_number: &i32) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    // printf("%d\n", ...)
    let _ = write!(out, "{}\n", *int_number);
    let _ = out.flush();
}

/// `void bad() { int *data; printIntPtrLine(data); }`
fn bad() {
    // `data` is never assigned; see the note above for what it actually points
    // at in the reference build.
    let data: i32 = STALE_POINTEE.with(|slot| slot.get());
    print_int_ptr_line(&data);
}

/// `void good() { int data = 5; int *data_addr = &data; printIntPtrLine(data_addr); }`
fn good() {
    let data: i32 = 5;
    let data_addr: &i32 = &data;
    print_int_ptr_line(data_addr);
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Reads one byte from stdin, or `None` on EOF.
fn next_byte<R: Read>(r: &mut R) -> Option<u8> {
    let mut buf = [0u8; 1];
    loop {
        match r.read(&mut buf) {
            Ok(0) => return None,
            Ok(_) => return Some(buf[0]),
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        }
    }
}

/// Emulates `scanf("%d", &x)`:
///  * skips leading whitespace (crossing newlines),
///  * accepts an optional `+`/`-` sign,
///  * requires at least one decimal digit, otherwise the conversion fails and
///    the destination is left untouched,
///  * accumulates into a saturating `long` (64-bit) and then stores it into an
///    `int`, i.e. truncates to 32 bits -- matching glibc's behavior for values
///    outside the range of `int`.
///
/// Returns `Some(value)` on a successful conversion, `None` on input or
/// matching failure.
fn scanf_int<R: Read>(r: &mut R) -> Option<i32> {
    // Skip whitespace.
    let mut c = loop {
        match next_byte(r) {
            None => return None, // input failure (EOF)
            Some(b) if is_c_space(b) => continue,
            Some(b) => break b,
        }
    };

    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        match next_byte(r) {
            None => return None, // matching failure: sign then EOF
            Some(b) => c = b,
        }
    }

    if !c.is_ascii_digit() {
        return None; // matching failure
    }

    let mut acc: i64 = 0;
    loop {
        let digit = (c - b'0') as i64;
        if negative {
            acc = acc
                .checked_mul(10)
                .and_then(|v| v.checked_sub(digit))
                .unwrap_or(i64::MIN);
        } else {
            acc = acc
                .checked_mul(10)
                .and_then(|v| v.checked_add(digit))
                .unwrap_or(i64::MAX);
        }
        match next_byte(r) {
            Some(b) if b.is_ascii_digit() => c = b,
            // Any other byte terminates the conversion (it would be pushed
            // back onto the stream; nothing else in this program reads stdin).
            _ => break,
        }
    }

    Some(acc as i32)
}

fn main() {
    let mut x: i32 = 0;
    {
        let stdin = std::io::stdin();
        let mut lock = stdin.lock();
        if let Some(v) = scanf_int(&mut lock) {
            x = v;
        }
    }

    // The storage `&x` referred to, as observed later through the stale pointer.
    STALE_POINTEE.with(|slot| slot.set(x));

    if x != 0 {
        good();
    } else {
        bad();
    }
}
