// cstdio.rs
//
// Emulates glibc's `stdout` buffering so that the byte stream *and* its
// interleaving with `stderr` match the C program.
//
// glibc picks the mode on first use: line-buffered when stdout is a terminal,
// fully buffered otherwise (flushing only when the buffer fills or at exit).
// Rust's `println!` is always line-buffered, which reorders output relative to
// the unbuffered `stderr` when both are redirected to the same file.

use std::cell::RefCell;
use std::io::{IsTerminal, Write};
use std::sync::OnceLock;

/// glibc's `BUFSIZ` on Linux.
const BUFSIZ: usize = 4096;

thread_local! {
    static BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn line_buffered() -> bool {
    static TTY: OnceLock<bool> = OnceLock::new();
    *TTY.get_or_init(|| std::io::stdout().is_terminal())
}

fn write_through(bytes: &[u8]) {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(bytes);
    let _ = lock.flush();
}

/// Equivalent of `fwrite(bytes, 1, n, stdout)`.
pub fn out(bytes: &[u8]) {
    let to_flush = BUF.with(|b| {
        let mut b = b.borrow_mut();
        b.extend_from_slice(bytes);

        let should_flush =
            (line_buffered() && bytes.contains(&b'\n')) || b.len() >= BUFSIZ;

        if should_flush {
            Some(std::mem::take(&mut *b))
        } else {
            None
        }
    });

    if let Some(data) = to_flush {
        write_through(&data);
    }
}

/// `fflush(stdout)`, also run implicitly at `exit`.
pub fn flush() {
    let data = BUF.with(|b| std::mem::take(&mut *b.borrow_mut()));
    if !data.is_empty() {
        write_through(&data);
    }
}

/// `printf(...)` with a trailing newline, i.e. the `printf("...\n")` pattern.
macro_rules! c_println {
    () => { $crate::cstdio::out(b"\n") };
    ($($arg:tt)*) => {{
        let mut s = format!($($arg)*);
        s.push('\n');
        $crate::cstdio::out(s.as_bytes())
    }};
}

/// `fprintf(stderr, ...)` with a trailing newline. `stderr` is unbuffered in C,
/// and Rust's `eprintln!` writes straight through, so no buffering is needed.
macro_rules! c_eprintln {
    ($($arg:tt)*) => {{
        // Ordering note: C's stderr write happens while stdout data may still
        // be sitting in stdout's buffer; we deliberately do NOT flush stdout
        // here, matching glibc.
        eprintln!($($arg)*)
    }};
}
