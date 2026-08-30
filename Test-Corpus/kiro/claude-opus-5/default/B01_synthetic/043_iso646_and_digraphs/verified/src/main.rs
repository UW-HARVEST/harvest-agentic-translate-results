// Rust translation of c_src/src/main.c
//
// The original C source uses digraphs and <iso646.h> alternative spellings:
//     %:include <stdio.h>   ->  #include <stdio.h>
//     <% ... %>             ->  { ... }
//     x bitor compl y       ->  x | ~y
//
// so `driver` computes `x | ~y` on `int` (32-bit) values and prints it
// followed by a newline (printf("%d", result) then puts("")).
//
// Behavior preserved from the C, including its quirks:
//   * The return values of the two scanf() calls are ignored, so a matching
//     failure or EOF leaves the corresponding variable at its initial 0.
//   * scanf("%d") skips arbitrary leading whitespace (including newlines) and
//     therefore reads across line boundaries.
//   * Out-of-range integers follow glibc's behaviour for "%d": the value is
//     converted with strtol (saturating at long's limits) and then assigned to
//     an int, truncating the low 32 bits.

use std::io::{self, Read, Write};

/// A stdin reader that consumes only the bytes scanf would consume, with a
/// single-byte pushback slot to emulate ungetc().
struct Scanner {
    input: io::Stdin,
    buf: [u8; 1],
    pushback: Option<u8>,
    eof: bool,
}

impl Scanner {
    fn new() -> Self {
        Scanner {
            input: io::stdin(),
            buf: [0u8; 1],
            pushback: None,
            eof: false,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.pushback.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        loop {
            match self.input.read(&mut self.buf) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(_) => return Some(self.buf[0]),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    fn unread(&mut self, b: u8) {
        self.pushback = Some(b);
    }

    /// Emulates `scanf("%d", &out)`. Returns Some(value) on a successful
    /// conversion, None on a matching failure or input failure (in which case
    /// the caller leaves its variable untouched, exactly like C).
    fn scan_int(&mut self) -> Option<i32> {
        // Skip leading whitespace, as the %d directive does.
        let mut c = loop {
            match self.next_byte() {
                None => return None, // input failure (EOF)
                Some(b) => {
                    if is_c_space(b) {
                        continue;
                    }
                    break b;
                }
            }
        };

        let mut negative = false;
        if c == b'+' || c == b'-' {
            negative = c == b'-';
            match self.next_byte() {
                None => return None,
                Some(b) => c = b,
            }
        }

        if !c.is_ascii_digit() {
            // Matching failure: push the offending character back.
            self.unread(c);
            return None;
        }

        // Accumulate like strtol: saturate at long (i64) bounds.
        let mut acc: i64 = 0;
        let mut overflow = false;
        loop {
            let digit = (c - b'0') as i64;
            if !overflow {
                match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                    Some(v) => acc = v,
                    None => overflow = true,
                }
            }
            match self.next_byte() {
                None => break,
                Some(b) => {
                    if b.is_ascii_digit() {
                        c = b;
                    } else {
                        self.unread(b);
                        break;
                    }
                }
            }
        }

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

        // Assignment of a long to an int truncates to the low 32 bits.
        Some(value as u64 as u32 as i32)
    }
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

fn driver(x: i32, y: i32, out: &mut impl Write) {
    let result = x | !y;
    // printf("%d", result);
    //
    // C ignores the return value of printf/puts, so a write error must not be
    // turned into a panic here: on a full device the C program still exits 0
    // with an empty stderr.
    let _ = write!(out, "{}", result);
    // puts("");
    let _ = writeln!(out);
}

/// The Rust runtime sets SIGPIPE to SIG_IGN before `main`, which makes writes
/// to a closed pipe return EPIPE instead of terminating the process. The C
/// program keeps the default disposition and is therefore killed by SIGPIPE
/// (shell status 141). Restore the default so the exit status matches.
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
    let mut y: i32 = 0;

    let mut scanner = Scanner::new();
    if let Some(v) = scanner.scan_int() {
        x = v;
    }
    if let Some(v) = scanner.scan_int() {
        y = v;
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    driver(x, y, &mut out);
    // Like the implicit flush of C's stdout at exit, failures are ignored.
    let _ = out.flush();
}
