//! Emulation of C stdio stream buffering semantics.
//!
//! When a C program's stdout is a pipe or a file, glibc fully buffers it and the
//! buffer is only flushed when it fills up or at `exit()`. stderr, by contrast, is
//! unbuffered and is written immediately. Reproducing that here keeps the byte
//! stream identical to the C program even when a caller merges the two streams
//! (`2>&1`), where all stderr output precedes the stdout output.

use std::cell::RefCell;
use std::io::{self, Write};

thread_local! {
    /// Fully buffered stdout, flushed explicitly at the end of `main`.
    static STDOUT_BUF: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(1 << 16));
}

/// Append bytes to the buffered stdout stream.
pub fn out_write(bytes: &[u8]) {
    STDOUT_BUF.with(|b| b.borrow_mut().extend_from_slice(bytes));
}

/// Flush the buffered stdout stream, mirroring the implicit flush at C `exit()`.
pub fn out_flush() {
    STDOUT_BUF.with(|b| {
        let mut buf = b.borrow_mut();
        let stdout = io::stdout();
        let mut lock = stdout.lock();
        // Errors on flush are ignored, exactly as the C code ignores printf's
        // return value.
        let _ = lock.write_all(&buf);
        let _ = lock.flush();
        buf.clear();
    });
}

/// Write bytes straight to stderr, unbuffered like C's stderr.
pub fn err_write(bytes: &[u8]) {
    let stderr = io::stderr();
    let mut lock = stderr.lock();
    let _ = lock.write_all(bytes);
    let _ = lock.flush();
}

/// `printf`-equivalent writing to the fully buffered stdout emulation.
#[macro_export]
macro_rules! c_printf {
    ($($arg:tt)*) => {
        $crate::cout::out_write(format!($($arg)*).as_bytes())
    };
}

/// `fprintf(stderr, ...)`-equivalent writing straight through to stderr.
#[macro_export]
macro_rules! c_eprintf {
    ($($arg:tt)*) => {
        $crate::cout::err_write(format!($($arg)*).as_bytes())
    };
}
