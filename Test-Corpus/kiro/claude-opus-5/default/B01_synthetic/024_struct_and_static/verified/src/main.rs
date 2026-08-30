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
// Rust translation of c_src/src/main.c. Behavior, including the original
// program's quirks, is reproduced as-is.

use std::cell::RefCell;
use std::io::{Read, Write};

/// Mirrors `house_t` from the C source.
#[derive(Clone, Copy)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

thread_local! {
    /// Mirrors the file-scope `static house_t the_house` initializer.
    static THE_HOUSE: RefCell<House> = const {
        RefCell::new(House {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        })
    };

    /// Mirrors C's stdio buffer for `stdout`. When stdout is not a terminal,
    /// glibc block-buffers it (4096 bytes by default); this program emits far
    /// less than that, so every `printf` lands in the buffer and a single
    /// write happens when the process exits. Buffering here rather than
    /// writing per line keeps the syscall pattern - and therefore the point at
    /// which a `SIGPIPE` can be raised - the same as the C program's.
    static STDOUT_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Restore the default disposition for `SIGPIPE`.
///
/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, which makes
/// writes to a closed pipe fail with `EPIPE` instead of killing the process. A
/// C program inherits the default disposition, so `printf` to a closed pipe
/// terminates it with signal 13. Without this the Rust program exits 0 where
/// the C program dies by signal.
#[cfg(unix)]
fn restore_default_sigpipe() {
    const SIGPIPE: core::ffi::c_int = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: core::ffi::c_int, handler: usize) -> usize;
    }
    // Safety: `signal` with `SIG_DFL` is async-signal-safe and this runs before
    // any other thread exists.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

fn add_floor(house: &mut House) {
    // C: house->floors++ (wraps in practice on overflow)
    house.floors = house.floors.wrapping_add(1);
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    // C: house->bedrooms += extra_bedrooms
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

fn add_floor_to_the_house() {
    THE_HOUSE.with(|h| add_floor(&mut h.borrow_mut()));
}

fn print_the_house() {
    let h = THE_HOUSE.with(|h| *h.borrow());
    // printf("The house has %d floors, %d bedrooms, and %.1f bathrooms\n", ...)
    STDOUT_BUF.with(|buf| {
        let _ = write!(
            buf.borrow_mut(),
            "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
            h.floors,
            h.bedrooms,
            h.bathrooms
        );
    });
}

/// Mirrors the implicit `fflush(stdout)` that `exit` performs.
fn flush_stdout() {
    let bytes = STDOUT_BUF.with(|buf| std::mem::take(&mut *buf.borrow_mut()));
    let out = std::io::stdout();
    let mut out = out.lock();
    // C ignores printf/fflush failures here; the exit status stays 0. If the
    // write triggers SIGPIPE the process dies before returning, as C's would.
    let _ = out.write_all(&bytes);
    let _ = out.flush();
}

fn run(extra_bedrooms: i32) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    THE_HOUSE.with(|h| h.borrow_mut().bathrooms += 1.0);
    print_the_house();
    THE_HOUSE.with(|h| add_bedrooms(&mut h.borrow_mut(), extra_bedrooms));
    print_the_house();
}

/// Emulates `scanf("%d", &x)`: skips leading whitespace (including newlines),
/// accepts an optional sign, then decimal digits. On matching failure the
/// destination is left untouched. Overflow follows glibc's strtol-based
/// behavior: saturate to long, then truncate to int.
fn scanf_i32() -> Option<i32> {
    let stdin = std::io::stdin();
    let mut stdin = stdin.lock();
    let mut byte = [0u8; 1];

    let mut next = || -> Option<u8> {
        match stdin.read(&mut byte) {
            Ok(1) => Some(byte[0]),
            _ => None,
        }
    };

    // Skip whitespace, as the C library's isspace() would.
    let mut c = loop {
        let c = next()?;
        if !matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
            break c;
        }
    };

    let negative = match c {
        b'-' => {
            c = next().unwrap_or(0);
            true
        }
        b'+' => {
            c = next().unwrap_or(0);
            false
        }
        _ => false,
    };

    if !c.is_ascii_digit() {
        // Matching failure: nothing is assigned.
        return None;
    }

    let mut acc: i64 = 0;
    let mut saturated = false;
    loop {
        if !saturated {
            let digit = i64::from(c - b'0');
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => saturated = true,
            }
        }
        match next() {
            Some(n) if n.is_ascii_digit() => c = n,
            // The trailing non-digit byte would be pushed back by scanf; the
            // original program never reads stdin again, so dropping it is
            // indistinguishable.
            _ => break,
        }
    }

    let value: i64 = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -acc
    } else {
        acc
    };

    Some(value as i32)
}

fn main() {
    restore_default_sigpipe();
    let mut x: i32 = 0;
    if let Some(v) = scanf_i32() {
        x = v;
    }
    run(x);
    run(x);
    flush_stdout();
}
