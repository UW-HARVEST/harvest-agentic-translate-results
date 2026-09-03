//! Minimal `<stdio.h>`-shaped output helpers.
//!
//! The C code reaches `printf`/`fprintf`, which write raw bytes and ignore
//! errors. These wrappers do the same: they take byte slices (so a non-UTF-8
//! `argv[0]` still reproduces exactly under `%s`) and swallow I/O errors instead
//! of panicking, which is what an unchecked `printf` return value amounts to.

use std::io::Write;

/// `fwrite(bytes, 1, n, stdout)` -- errors ignored, as `printf`'s return value is.
pub fn print_bytes(bytes: &[u8]) {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(bytes);
}

/// `printf("%s", s)`.
pub fn print_str(s: &str) {
    print_bytes(s.as_bytes());
}

/// `fwrite(bytes, 1, n, stderr)`.
pub fn eprint_bytes(bytes: &[u8]) {
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    let _ = lock.write_all(bytes);
}

/// Flushes `stdout`, mirroring the implicit flush C performs at `exit`.
pub fn flush_stdout() {
    let _ = std::io::stdout().flush();
}
