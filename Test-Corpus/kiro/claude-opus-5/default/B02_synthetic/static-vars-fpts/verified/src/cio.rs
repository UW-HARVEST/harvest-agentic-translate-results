//! C stdio emulation: fully-buffered stdout, unbuffered stderr, and `fgets`.
//!
//! The original program never calls `fflush`, so when stdout is a pipe glibc
//! buffers everything and only writes it out at exit. `Out` mirrors that with a
//! BufWriter whose capacity matches glibc's usual pipe buffer size.

use std::io::{self, BufReader, Read, Write};

/// Restore the default disposition for `SIGPIPE`.
///
/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, so a write
/// to a closed pipe returns `EPIPE` and the process goes on to exit 0. A C
/// program keeps the default disposition and is killed by the signal instead,
/// exiting with status 141 (128 + SIGPIPE). Resetting it here makes the exit
/// status match the C program when the reader closes the pipe early.
pub fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

/// Buffered stdout writer (mirrors C's fully-buffered `stdout`).
pub struct Out {
    w: io::BufWriter<io::Stdout>,
}

impl Out {
    pub fn new() -> Self {
        Out {
            w: io::BufWriter::with_capacity(4096, io::stdout()),
        }
    }

    /// Write raw bytes (equivalent to `fputs`/`printf` of already formatted text).
    pub fn bytes(&mut self, b: &[u8]) {
        let _ = self.w.write_all(b);
    }

    pub fn str(&mut self, s: &str) {
        self.bytes(s.as_bytes());
    }

    pub fn flush(&mut self) {
        let _ = self.w.flush();
    }
}

/// Write to stderr immediately (C's `stderr` is unbuffered).
pub fn err_bytes(b: &[u8]) {
    let mut e = io::stderr();
    let _ = e.write_all(b);
    let _ = e.flush();
}

pub fn err_str(s: &str) {
    err_bytes(s.as_bytes());
}

/// Line reader replicating `fgets` semantics.
pub struct In {
    r: BufReader<io::Stdin>,
}

impl In {
    pub fn new() -> Self {
        In {
            r: BufReader::new(io::stdin()),
        }
    }

    /// `fgets(buf, n, stdin)`: read at most `n - 1` bytes, stopping after a
    /// newline (which is kept). Returns `None` only when EOF is hit before any
    /// byte is read, matching fgets' NULL return.
    pub fn fgets(&mut self, n: usize) -> Option<Vec<u8>> {
        let mut v: Vec<u8> = Vec::new();
        while v.len() + 1 < n {
            let mut b = [0u8; 1];
            match self.r.read(&mut b) {
                Ok(0) => break,
                Ok(_) => {
                    v.push(b[0]);
                    if b[0] == b'\n' {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    }
}

/// Reinterpret a raw buffer as a C string: everything up to the first NUL.
pub fn cstr(buf: &[u8]) -> &[u8] {
    match buf.iter().position(|&b| b == 0) {
        Some(i) => &buf[..i],
        None => buf,
    }
}

/// `sscanf(s, "%d", &out)`: returns None when the conversion fails (the C code
/// treats any return value other than 1 as invalid input).
///
/// glibc converts via `strtol`, saturating at LONG_MIN/LONG_MAX on overflow,
/// then truncates the `long` to `int`. That truncation is reproduced here.
pub fn sscanf_d(s: &[u8]) -> Option<i32> {
    let mut i = 0usize;
    while i < s.len() && is_space(s[i]) {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    let mut acc: i128 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        if !overflow {
            acc = acc * 10 + i128::from(s[i] - b'0');
            if acc > i128::from(i64::MAX) {
                overflow = true;
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits converted: sscanf returns 0 or EOF, never 1.
        return None;
    }

    let value: i64 = if overflow {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -(acc as i64)
    } else {
        acc as i64
    };

    Some(value as i32)
}

// --- ctype.h helpers -------------------------------------------------------
//
// The C code passes plain `char` values to isspace/isalpha/... On x86 `char` is
// signed, so bytes >= 0x80 arrive as negative values; in the "C" locale glibc
// classifies none of those as alpha/digit/space. Restricting these helpers to
// ASCII therefore matches the original behavior.

pub fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

pub fn is_alpha(c: u8) -> bool {
    c.is_ascii_alphabetic()
}

pub fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

pub fn is_alnum(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

/// `strstr(haystack, needle) != NULL`
pub fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}
