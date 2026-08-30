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

use std::cell::Cell;
use std::io::{Read, Write};

#[derive(Clone, Copy)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

// `static house_t the_house = {...};` -- mutable process-wide state.
thread_local! {
    static THE_HOUSE: Cell<House> = const {
        Cell::new(House {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        })
    };
}

fn with_the_house<R>(f: impl FnOnce(&mut House) -> R) -> R {
    THE_HOUSE.with(|cell| {
        let mut house = cell.get();
        let result = f(&mut house);
        cell.set(house);
        result
    })
}

fn add_floor(house: &mut House) {
    // C: house->floors++  (wraps in practice on overflow)
    house.floors = house.floors.wrapping_add(1);
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    // C: house->bedrooms += extra_bedrooms
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

fn add_floor_to_the_house() {
    with_the_house(add_floor);
}

fn print_the_house() {
    let house = THE_HOUSE.with(|cell| cell.get());
    // NOTE: `printf` in C reports failure through its return value, which the
    // original ignores -- a failing write is silent and does NOT abort the
    // program. The `print!`/`println!` macros panic on write errors, so we go
    // through `write!` and drop the `Result` to reproduce C's behavior.
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = write!(
        out,
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
        house.floors, house.bedrooms, house.bathrooms
    );
}

fn run(extra_bedrooms: i32) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    with_the_house(|h| h.bathrooms += 1.0);
    print_the_house();
    with_the_house(|h| add_bedrooms(h, extra_bedrooms));
    print_the_house();
}

/// Minimal reader over stdin with one byte of pushback, mimicking the way
/// `scanf` consumes exactly what it needs (and ungets the terminating byte).
struct Stdin {
    inner: std::io::Stdin,
    pushback: Option<u8>,
}

impl Stdin {
    fn new() -> Self {
        Stdin {
            inner: std::io::stdin(),
            pushback: None,
        }
    }

    fn getc(&mut self) -> Option<u8> {
        if let Some(b) = self.pushback.take() {
            return Some(b);
        }
        let mut buf = [0u8; 1];
        loop {
            match self.inner.read(&mut buf) {
                Ok(0) => return None,
                Ok(_) => return Some(buf[0]),
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            }
        }
    }

    fn ungetc(&mut self, b: u8) {
        self.pushback = Some(b);
    }
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// Emulates `scanf("%d", &x)`: returns Some(value) on a successful conversion,
/// None on matching failure or EOF (in which case the caller leaves `x` alone).
fn scanf_i32(input: &mut Stdin) -> Option<i32> {
    // Skip leading whitespace (the %d conversion does this implicitly).
    let mut c = loop {
        match input.getc() {
            Some(b) if is_c_space(b) => continue,
            Some(b) => break b,
            None => return None, // EOF before any input
        }
    };

    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        match input.getc() {
            Some(b) => c = b,
            None => return None, // sign then EOF: matching failure
        }
    }

    if !c.is_ascii_digit() {
        input.ungetc(c);
        return None; // matching failure
    }

    // Accumulate like strtol: saturate at long range, then truncate to int,
    // which is what glibc's %d conversion ends up doing.
    let mut acc: i128 = 0;
    loop {
        let digit = (c - b'0') as i128;
        if acc < (i64::MAX as i128) + 1 {
            acc = acc * 10 + digit;
        }
        match input.getc() {
            Some(b) if b.is_ascii_digit() => c = b,
            Some(b) => {
                input.ungetc(b);
                break;
            }
            None => break,
        }
    }

    let signed: i128 = if negative { -acc } else { acc };
    let clamped: i64 = if signed > i64::MAX as i128 {
        i64::MAX
    } else if signed < i64::MIN as i128 {
        i64::MIN
    } else {
        signed as i64
    };
    Some(clamped as i32)
}

/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs; a C program
/// started from a shell inherits the default disposition and is therefore
/// *killed* by `SIGPIPE` when it writes to a closed pipe. Restore the default
/// so the exit status matches the C program's in that case.
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn main() {
    #[cfg(unix)]
    restore_default_sigpipe();

    let mut x: i32 = 0;
    let mut input = Stdin::new();
    if let Some(v) = scanf_i32(&mut input) {
        x = v;
    }
    run(x);
    run(x);
    let _ = std::io::stdout().flush();
}
