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

#[cfg(unix)]
mod sigpipe {
    // `sighandler_t` is pointer-sized on every Unix target; SIG_DFL is 0.
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }

    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;

    /// The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main`
    /// runs, so a write to a closed pipe returns `EPIPE` instead of killing the
    /// process. A C program keeps the default disposition and is terminated by
    /// the signal, exiting with status 141 (`128 + 13`) as far as a shell is
    /// concerned. Restore the C behaviour so the exit status matches.
    pub fn restore_default() {
        unsafe {
            signal(SIGPIPE, SIG_DFL);
        }
    }
}

#[cfg(not(unix))]
mod sigpipe {
    pub fn restore_default() {}
}

/// Re-establish the signal dispositions a C program starts life with.
pub fn init_c_runtime() {
    sigpipe::restore_default();
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
