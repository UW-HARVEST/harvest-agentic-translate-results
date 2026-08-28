// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Faithful Rust translation of `c_src/src/main.c`.
//!
//! ```c
//! int main() {
//!     char text[128];
//!     while (fgets(text, 128, stdin)) {
//!         fputs(text, stdout);
//!     }
//!     return 0;
//! }
//! ```
//!
//! interactive echo; ignores arguments, copies stdin to stdout

use std::io::{self, BufRead, BufReader, ErrorKind, IsTerminal, Write};
use std::process::ExitCode;

/// Size of the C `char text[128]` buffer.
const TEXT_LEN: usize = 128;

/// The Rust runtime masks `SIGPIPE` (`SIG_IGN`) before `main` runs, but a C
/// program starts with the default disposition. Without this reset, writing to
/// a closed pipe makes this program see `EPIPE` and exit 0, while the C program
/// is killed by signal 13. Restore `SIG_DFL` so the process dies the same way.
#[cfg(unix)]
fn reset_sigpipe() {
    const SIGPIPE: i32 = 13;

    extern "C" {
        fn signal(signum: i32, handler: Option<extern "C" fn(i32)>) -> usize;
    }

    // `None` is the null function pointer, i.e. C's `SIG_DFL`.
    unsafe {
        signal(SIGPIPE, None);
    }
}

#[cfg(not(unix))]
fn reset_sigpipe() {}

/// Emulation of C's `fgets(buf, size, stdin)`.
///
/// Reads at most `size - 1` bytes, stopping early after a newline (which is
/// kept) or at end-of-file. The C version NUL-terminates the buffer; here the
/// bytes actually read are returned instead, and the terminator is modelled by
/// the caller (`fputs` stops at the first NUL byte).
///
/// Returns `None` only when no bytes at all could be consumed, matching glibc's
/// `_IO_fgets`, which yields `NULL` just when the line count is zero and the
/// stream is in an error or end-of-file state. A read error that happens *after*
/// some bytes were consumed therefore still returns those bytes; the following
/// call is the one that reports `NULL`.
fn fgets<R: BufRead>(reader: &mut R, size: usize) -> Option<Vec<u8>> {
    // `fgets` with size <= 0 reads nothing; with size == 1 it stores just the
    // NUL terminator, so no input bytes are consumed but a non-NULL pointer is
    // returned. The C program always passes 128, so this is only for safety.
    if size == 0 {
        return None;
    }
    let max = size - 1;

    let mut out: Vec<u8> = Vec::new();
    while out.len() < max {
        let refilled: Option<&[u8]> = loop {
            match reader.fill_buf() {
                Ok(buf) => break Some(buf),
                // Retry on EINTR, as stdio does.
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                // A read error ends this call; whatever was already consumed is
                // still returned, and only an empty buffer becomes NULL below.
                Err(_) => break None,
            }
        };
        let available = match refilled {
            Some(buf) => buf,
            None => break,
        };

        if available.is_empty() {
            // End of file.
            break;
        }

        let take = std::cmp::min(max - out.len(), available.len());
        let chunk = &available[..take];

        match chunk.iter().position(|&b| b == b'\n') {
            Some(nl) => {
                out.extend_from_slice(&chunk[..=nl]);
                reader.consume(nl + 1);
                break;
            }
            None => {
                out.extend_from_slice(chunk);
                reader.consume(take);
            }
        }
    }

    if out.is_empty() {
        // EOF and nothing read: `fgets` returns NULL.
        None
    } else {
        Some(out)
    }
}

/// Block size C stdio picks for a regular file or pipe (`st_blksize`). Only the
/// granularity of the underlying `write` calls depends on this; the byte stream
/// produced is identical either way.
const BUFSIZ: usize = 4096;

/// The process' stdout descriptor, wrapped so it is never closed on drop — just
/// like C's `stdout`, which outlives `main`.
struct Fd1 {
    #[cfg(unix)]
    file: std::mem::ManuallyDrop<std::fs::File>,
    #[cfg(not(unix))]
    file: io::Stdout,
}

impl Fd1 {
    fn new() -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::io::FromRawFd;
            // SAFETY: descriptor 1 is open for the lifetime of the process, and
            // `ManuallyDrop` keeps this wrapper from closing it.
            Fd1 {
                file: std::mem::ManuallyDrop::new(unsafe {
                    std::fs::File::from_raw_fd(1)
                }),
            }
        }
        #[cfg(not(unix))]
        {
            Fd1 { file: io::stdout() }
        }
    }
}

impl Write for Fd1 {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        #[cfg(unix)]
        {
            let mut file: &std::fs::File = &self.file;
            file.write(bytes)
        }
        #[cfg(not(unix))]
        {
            self.file.write(bytes)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        #[cfg(unix)]
        {
            let mut file: &std::fs::File = &self.file;
            file.flush()
        }
        #[cfg(not(unix))]
        {
            self.file.flush()
        }
    }
}

/// C `stdout`: line buffered when it refers to a terminal, fully buffered
/// otherwise. Rust's own `Stdout` is *always* line buffered, so the descriptor
/// is driven directly to keep the flush points where C puts them.
struct CStdout {
    sink: Fd1,
    buf: Vec<u8>,
    line_buffered: bool,
}

impl CStdout {
    fn new() -> Self {
        CStdout {
            sink: Fd1::new(),
            buf: Vec::with_capacity(BUFSIZ),
            line_buffered: io::stdout().is_terminal(),
        }
    }

    /// Emulation of C's `fputs(text, stdout)` on a NUL-terminated buffer: writes
    /// the bytes up to (but not including) the first NUL byte.
    fn fputs(&mut self, text: &[u8]) -> io::Result<()> {
        let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
        self.buf.extend_from_slice(&text[..end]);

        if self.line_buffered && self.buf.contains(&b'\n') {
            return self.flush();
        }

        // C stdio fills the buffer to capacity and writes it out whole, keeping
        // whatever did not fit. Draining in exact `BUFSIZ` blocks reproduces the
        // same `write` boundaries, so a reader sees the same bytes at the same
        // points even before the program exits.
        while self.buf.len() >= BUFSIZ {
            let result = self.sink.write_all(&self.buf[..BUFSIZ]);
            self.buf.drain(..BUFSIZ);
            result?;
        }
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.buf.is_empty() {
            return Ok(());
        }
        let result = self.sink.write_all(&self.buf);
        self.buf.clear();
        result
    }
}

fn main() -> ExitCode {
    reset_sigpipe();

    let stdin = io::stdin();
    let mut reader = BufReader::with_capacity(BUFSIZ, stdin.lock());
    let mut stdout = CStdout::new();

    while let Some(text) = fgets(&mut reader, TEXT_LEN) {
        if stdout.fputs(&text).is_err() {
            // Mirror stdio: a write error is not reported by this program.
            break;
        }
    }

    let _ = stdout.flush();
    ExitCode::from(0)
}
