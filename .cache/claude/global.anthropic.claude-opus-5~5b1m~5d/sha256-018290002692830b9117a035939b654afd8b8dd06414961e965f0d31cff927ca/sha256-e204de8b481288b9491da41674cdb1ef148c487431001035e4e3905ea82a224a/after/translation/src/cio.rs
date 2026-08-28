//! Emulation of C stdio buffering semantics.
//!
//! In C, `stdout` is fully buffered when it is not a terminal (the common case
//! when the program's output is captured/piped) and line buffered when it is a
//! terminal.  `stderr` is always unbuffered.  Reproducing this exactly matters
//! because the C program writes to both streams, and the relative ordering of
//! the two streams in a merged capture depends on that buffering.

use std::cell::RefCell;
use std::io::{IsTerminal, Write};

/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, so a failed
/// write to a closed pipe surfaces as an `EPIPE` error instead of killing the
/// process.  A C program keeps the default disposition, so writing to a closed
/// pipe terminates it with signal 13 (wait status 141).  Restore `SIG_DFL` so
/// the exit status matches the C program's for every stream-error input.
pub fn restore_default_sigpipe() {
    // `signal()` comes from libc, which is already linked on every *-linux-gnu
    // (and *-apple-darwin) target, so no extra dependency is needed.
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

/// glibc's `BUFSIZ`.
const BUFSIZ: u64 = 8192;

/// Reproduce glibc's `_IO_file_doallocate` buffer sizing: start from `BUFSIZ`
/// and shrink to the descriptor's `st_blksize` when that is smaller (4096 for a
/// pipe, and for a regular file on ext4/xfs). If `fstat` fails -- for instance
/// because fd 1 was closed outright -- glibc keeps `BUFSIZ`, so we do too.
fn stdout_buffer_size() -> usize {
    use std::os::unix::fs::MetadataExt;
    use std::os::unix::io::{FromRawFd, IntoRawFd};

    // Borrow fd 1 without taking ownership of it: `into_raw_fd` gives it back
    // without running File's destructor, so stdout is never closed.
    let f = unsafe { std::fs::File::from_raw_fd(1) };
    let blksize = f.metadata().ok().map(|m| m.blksize());
    let _ = f.into_raw_fd();

    match blksize {
        Some(b) if b > 0 && b < BUFSIZ => b as usize,
        _ => BUFSIZ as usize,
    }
}

pub struct StdoutBuf {
    buf: Vec<u8>,
    line_buffered: bool,
    /// Capacity of the emulated stdio buffer.
    bufsize: usize,
}

impl StdoutBuf {
    fn new() -> Self {
        StdoutBuf {
            buf: Vec::new(),
            line_buffered: std::io::stdout().is_terminal(),
            bufsize: stdout_buffer_size(),
        }
    }

    /// Mirror of glibc's `_IO_new_file_xsputn`.
    ///
    /// The buffer is only flushed when the incoming bytes do not fit in the
    /// space that is left -- notably *not* when a write happens to fill it
    /// exactly, which is why the flush can lag one `printf` behind. That timing
    /// is observable whenever stdout errors out partway through a run.
    fn push(&mut self, bytes: &[u8]) {
        if self.line_buffered {
            self.buf.extend_from_slice(bytes);
            self.flush_complete_lines();
            // A line-buffered stream still flushes if the buffer fills up.
            while self.buf.len() > self.bufsize {
                self.write_out(self.bufsize);
            }
            return;
        }

        let space = self.bufsize - self.buf.len();
        if bytes.len() <= space {
            // Fits: buffer it and return without flushing.
            self.buf.extend_from_slice(bytes);
            return;
        }

        // Fill the buffer to the brim, then flush all of it.
        self.buf.extend_from_slice(&bytes[..space]);
        self.write_out(self.bufsize);

        // glibc writes the remainder in whole blocks and buffers the tail.
        let mut rest = &bytes[space..];
        while rest.len() >= self.bufsize {
            self.buf.extend_from_slice(&rest[..self.bufsize]);
            self.write_out(self.bufsize);
            rest = &rest[self.bufsize..];
        }
        self.buf.extend_from_slice(rest);
    }

    /// Write out (and drop) the first `n` buffered bytes.
    fn write_out(&mut self, n: usize) {
        let n = n.min(self.buf.len());
        if n == 0 {
            return;
        }
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        let _ = lock.write_all(&self.buf[..n]);
        let _ = lock.flush();
        self.buf.drain(..n);
    }

    fn flush_complete_lines(&mut self) {
        if let Some(pos) = self.buf.iter().rposition(|&c| c == b'\n') {
            self.write_out(pos + 1);
        }
    }

    fn flush_all(&mut self) {
        let n = self.buf.len();
        self.write_out(n);
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
