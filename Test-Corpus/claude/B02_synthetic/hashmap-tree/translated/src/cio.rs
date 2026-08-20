//! Minimal C-like stdio emulation.
//!
//! C's `stdout` is fully buffered when it is not a terminal (the common case
//! for a redirected/piped run) and `stderr` is unbuffered.  To reproduce the
//! exact interleaving of the original program we buffer stdout in a single
//! large buffer that is flushed only at process exit, and write stderr
//! straight through.

use std::cell::RefCell;
use std::io::Write;

thread_local! {
    static STDOUT_BUF: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(1 << 16));
}

/// Append bytes to the emulated stdout buffer.
pub fn out_bytes(bytes: &[u8]) {
    STDOUT_BUF.with(|b| b.borrow_mut().extend_from_slice(bytes));
}

/// Append a string to the emulated stdout buffer.
pub fn out_str(s: &str) {
    out_bytes(s.as_bytes());
}

/// Flush the emulated stdout buffer to the real stdout.
pub fn out_flush() {
    STDOUT_BUF.with(|b| {
        let mut buf = b.borrow_mut();
        if !buf.is_empty() {
            let stdout = std::io::stdout();
            let mut lock = stdout.lock();
            let _ = lock.write_all(&buf);
            let _ = lock.flush();
            buf.clear();
        }
    });
}

/// Unbuffered write to stderr, like C's `fprintf(stderr, ...)`.
pub fn err_str(s: &str) {
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    let _ = lock.write_all(s.as_bytes());
    let _ = lock.flush();
}

#[macro_export]
macro_rules! cprintf {
    ($($arg:tt)*) => {
        $crate::cio::out_str(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! ceprintf {
    ($($arg:tt)*) => {
        $crate::cio::err_str(&format!($($arg)*))
    };
}
