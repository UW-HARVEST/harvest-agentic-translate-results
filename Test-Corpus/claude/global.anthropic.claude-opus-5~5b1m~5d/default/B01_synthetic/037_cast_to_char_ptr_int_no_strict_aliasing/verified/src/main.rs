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

use std::io::{self, Read, Write};

/// Minimal stdin byte reader with one byte of pushback, mimicking the way
/// `scanf` consumes characters from the stream (it may read across newlines).
struct Stdin {
    inner: io::Stdin,
    peeked: Option<u8>,
    eof: bool,
}

impl Stdin {
    fn new() -> Self {
        Stdin {
            inner: io::stdin(),
            peeked: None,
            eof: false,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        loop {
            match self.inner.read(&mut buf) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(_) => return Some(buf[0]),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    fn unget(&mut self, b: u8) {
        self.peeked = Some(b);
    }
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Emulates `scanf("%d", &x)`: returns Some(value) on a successful conversion,
/// or None on matching failure / EOF (in which case the caller leaves the
/// destination untouched, exactly like C).
fn scanf_i32(input: &mut Stdin) -> Option<i32> {
    // Skip leading whitespace.
    let mut b = loop {
        match input.next_byte() {
            Some(c) if is_c_space(c) => continue,
            Some(c) => break c,
            None => return None,
        }
    };

    let mut negative = false;
    if b == b'+' || b == b'-' {
        negative = b == b'-';
        match input.next_byte() {
            Some(c) => b = c,
            None => return None,
        }
    }

    if !b.is_ascii_digit() {
        input.unget(b);
        return None;
    }

    // Accumulate as a C `long` (64-bit), saturating on overflow the same way
    // glibc's strtol-based conversion does, then truncate to `int`.
    let mut acc: i64 = 0;
    let mut overflow = false;
    let mut cur = Some(b);
    while let Some(c) = cur {
        if !c.is_ascii_digit() {
            input.unget(c);
            break;
        }
        let d = i64::from(c - b'0');
        if !overflow {
            match acc.checked_mul(10).and_then(|v| {
                if negative {
                    v.checked_sub(d)
                } else {
                    v.checked_add(d)
                }
            }) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        cur = input.next_byte();
    }

    if overflow {
        acc = if negative { i64::MIN } else { i64::MAX };
    }

    Some(acc as i32)
}

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut line = String::new();
    for &byte in p {
        line.push_str(&format!("{:02x}", byte));
    }
    line.push('\n');
    let _ = out.write_all(line.as_bytes());
}

fn driver(x: i32) {
    let raw = x.to_ne_bytes();
    print_hex(&raw);
}

/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, but a C
/// program inherits the default disposition. Without this, writing to a pipe
/// whose reader is gone makes the C program die from `SIGPIPE` while the Rust
/// one quietly exits 0 — an observable difference in termination status.
#[cfg(unix)]
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

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

fn main() {
    restore_default_sigpipe();

    let mut x: i32 = 0;
    let mut input = Stdin::new();
    if let Some(v) = scanf_i32(&mut input) {
        x = v;
    }
    driver(x);
    let _ = io::stdout().flush();
}
