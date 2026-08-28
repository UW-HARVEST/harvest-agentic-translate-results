//! Emulation of C stdio buffering semantics.
//!
//! In C, `stdout` is fully buffered when it is not a terminal (the common case
//! when the program's output is captured/piped) and line buffered when it is a
//! terminal.  `stderr` is always unbuffered.  Reproducing this exactly matters
//! because the C program writes to both streams, and the relative ordering of
//! the two streams in a merged capture depends on that buffering.

use std::cell::RefCell;
use std::io::{IsTerminal, Write};

pub struct StdoutBuf {
    buf: Vec<u8>,
    line_buffered: bool,
}

impl StdoutBuf {
    fn new() -> Self {
        StdoutBuf {
            buf: Vec::new(),
            line_buffered: std::io::stdout().is_terminal(),
        }
    }

    fn push(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
        if self.line_buffered {
            self.flush_complete_lines();
        }
    }

    fn flush_complete_lines(&mut self) {
        if let Some(pos) = self.buf.iter().rposition(|&c| c == b'\n') {
            let stdout = std::io::stdout();
            let mut lock = stdout.lock();
            let _ = lock.write_all(&self.buf[..=pos]);
            let _ = lock.flush();
            self.buf.drain(..=pos);
        }
    }

    fn flush_all(&mut self) {
        if !self.buf.is_empty() {
            let stdout = std::io::stdout();
            let mut lock = stdout.lock();
            let _ = lock.write_all(&self.buf);
            let _ = lock.flush();
            self.buf.clear();
        }
    }
}

thread_local! {
    static OUT: RefCell<StdoutBuf> = RefCell::new(StdoutBuf::new());
}

/// `printf`-like write of already formatted text to stdout.
pub fn out_str(s: &str) {
    OUT.with(|o| o.borrow_mut().push(s.as_bytes()));
}

/// Write raw bytes to stdout (used for `%s` of a C `char` buffer).
pub fn out_bytes(b: &[u8]) {
    OUT.with(|o| o.borrow_mut().push(b));
}

/// Flush stdout, as the C runtime does at normal process exit.
pub fn flush() {
    OUT.with(|o| o.borrow_mut().flush_all());
}

/// `printf(...)` to stdout, honoring C's buffering rules.
#[macro_export]
macro_rules! c_printf {
    ($($arg:tt)*) => {{
        $crate::cio::out_str(&format!($($arg)*));
    }};
}

/// `fprintf(stderr, ...)`: stderr is unbuffered in C, so write immediately.
#[macro_export]
macro_rules! c_eprintf {
    ($($arg:tt)*) => {{
        use std::io::Write;
        let stderr = std::io::stderr();
        let mut lock = stderr.lock();
        let _ = lock.write_all(format!($($arg)*).as_bytes());
        let _ = lock.flush();
    }};
}

/// `assert()` from <assert.h>: on failure glibc prints a diagnostic to stderr
/// and calls abort(), which does *not* flush the buffered stdout stream.
#[macro_export]
macro_rules! c_assert {
    ($cond:expr, $func:expr) => {{
        if !$cond {
            $crate::c_eprintf!(
                "driver: main.c:{}: {}: Assertion `{}' failed.\n",
                line!(),
                $func,
                stringify!($cond)
            );
            std::process::abort();
        }
    }};
}
