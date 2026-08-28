//! C stdio behaviour needed for byte-identical output:
//!
//! * `stdout` is fully buffered when it is not a terminal and line buffered when
//!   it is, with a buffer sized from `fstat`'s `st_blksize`; `stderr` is
//!   unbuffered. Keeping that split preserves the interleaving of the two
//!   streams and the exact points at which stdout reaches the file descriptor.
//! * `fgets` stops after a newline and never reads past the buffer size, so a
//!   line longer than the buffer is returned across several calls.
//! * `sscanf("%d")` skips leading whitespace, accepts an optional sign, needs at
//!   least one digit, and (in glibc on 64-bit Linux) parses into a `long` that
//!   saturates before being truncated to `int`.

use std::io::{IsTerminal, Read, Write};

/// glibc's fallback when `st_blksize` is unavailable, and the value Linux
/// reports for pipes and for the usual on-disk filesystems.
const DEFAULT_BUFSIZ: usize = 4096;

pub struct Console {
    buf: Vec<u8>,
    bufsiz: usize,
    line_buffered: bool,
}

impl Console {
    pub fn new() -> Console {
        Console {
            buf: Vec::with_capacity(DEFAULT_BUFSIZ * 2),
            bufsiz: stdout_blksize(),
            line_buffered: std::io::stdout().is_terminal(),
        }
    }

    /// `printf` / `fputs(.., stdout)`.
    pub fn out(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);

        // Line buffered: everything up to and including the last newline goes
        // out now.
        if self.line_buffered {
            if let Some(nl) = self.buf.iter().rposition(|&b| b == b'\n') {
                let rest = self.buf.split_off(nl + 1);
                let chunk = std::mem::replace(&mut self.buf, rest);
                write_stdout(&chunk);
            }
        }

        // Full buffer: glibc fills it exactly, writes it, then continues.
        while self.buf.len() >= self.bufsiz {
            let rest = self.buf.split_off(self.bufsiz);
            let chunk = std::mem::replace(&mut self.buf, rest);
            write_stdout(&chunk);
        }
    }

    /// `fprintf(stderr, ..)`; unbuffered.
    pub fn err(&mut self, bytes: &[u8]) {
        let mut e = std::io::stderr().lock();
        let _ = e.write_all(bytes);
        let _ = e.flush();
    }

    /// Implicit flush of `stdout` at process exit.
    pub fn flush(&mut self) {
        if !self.buf.is_empty() {
            let chunk = std::mem::take(&mut self.buf);
            write_stdout(&chunk);
        }
    }
}

/// `st_blksize` of file descriptor 1, which is the buffer size glibc picks for
/// `stdout`. Probed without `unsafe`; pipes and sockets have no path under
/// `/proc/self/fd`, and for those the fallback is the size Linux reports anyway.
fn stdout_blksize() -> usize {
    use std::os::unix::fs::MetadataExt;
    match std::fs::metadata("/proc/self/fd/1") {
        Ok(m) if m.blksize() > 0 => m.blksize() as usize,
        _ => DEFAULT_BUFSIZ,
    }
}

/// Push bytes all the way to fd 1. `std::io::stdout()` is line buffered, so it
/// would otherwise hold back any tail after the last newline and shift where
/// unbuffered stderr output lands in an interleaved stream.
fn write_stdout(bytes: &[u8]) {
    let mut o = std::io::stdout().lock();
    let _ = o.write_all(bytes);
    let _ = o.flush();
}

/// glibc's `malloc_printerr`: write the diagnostic to stderr and `abort()`.
/// Anything still sitting in the stdout buffer is lost, exactly as in C.
pub fn glibc_abort(c: &mut Console, msg: &[u8]) -> ! {
    c.err(msg);
    std::process::abort()
}

pub struct Input {
    reader: std::io::BufReader<std::io::Stdin>,
}

impl Input {
    pub fn new() -> Input {
        Input {
            reader: std::io::BufReader::new(std::io::stdin()),
        }
    }

    /// `fgets(buf, size, stdin)`: at most `size - 1` bytes, stopping after a
    /// newline (which is kept). Returns `None` on immediate EOF, like a NULL
    /// return.
    pub fn fgets(&mut self, size: usize) -> Option<Vec<u8>> {
        let mut out: Vec<u8> = Vec::new();
        let mut byte = [0u8; 1];
        while out.len() + 1 < size {
            match self.reader.read(&mut byte) {
                Ok(0) => break,
                Ok(_) => {
                    out.push(byte[0]);
                    if byte[0] == b'\n' {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }
}

/// `sscanf(s, "%d", &out)`: `Some(value)` when one item was assigned.
pub fn sscanf_int(s: &[u8]) -> Option<i32> {
    let mut i = 0usize;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r') {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let start = i;
    let mut acc: i64 = 0;
    let mut saturated = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = (s[i] - b'0') as i64;
        if !saturated {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => saturated = true,
            }
        }
        i += 1;
    }

    if i == start {
        // Matching failure: no digits consumed.
        return None;
    }

    // strtol saturates at LONG_MAX / LONG_MIN, then scanf stores the long into
    // an int, truncating the high bits.
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
