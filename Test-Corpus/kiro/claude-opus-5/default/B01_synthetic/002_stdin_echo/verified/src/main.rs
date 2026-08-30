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

//! Interactive echo; ignores arguments, copies stdin to stdout.
//!
//! Faithful translation of the original C:
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
//! The C behavior that must be preserved byte-for-byte:
//!   * `fgets(text, 128, stdin)` reads at most 127 bytes, stopping early after a
//!     newline (the newline is kept in the buffer), and NUL-terminates.
//!   * `fgets` returns NULL only when EOF (or an error) occurs *before* any byte
//!     was read; a final line without a trailing newline is still returned.
//!   * `fputs(text, stdout)` writes bytes up to the terminating NUL, so a NUL
//!     byte embedded in the input truncates the remainder of that 127-byte
//!     chunk from the output.
//!   * A C program inherits the default `SIGPIPE` disposition, so writing to a
//!     closed stdout terminates it with signal 13. The Rust runtime installs
//!     `SIG_IGN` for `SIGPIPE` before `main`, so it must be restored.

use std::io::{self, BufRead, BufReader, Write};

/// Size of the C `char text[128]` buffer.
const BUF_SIZE: usize = 128;

/// glibc gives a piped `stdout` a fully buffered `FILE` whose buffer is the
/// pipe's block size (4096 on Linux). Matching it keeps the flush granularity,
/// and therefore how much output escapes before a fatal `SIGPIPE`, the same.
const STDOUT_BUF_SIZE: usize = 4096;

/// Restores the default `SIGPIPE` disposition that a C program would have.
///
/// Rust sets `SIGPIPE` to `SIG_IGN` during runtime startup, which turns a write
/// to a closed pipe into an ignored `EPIPE` error and an exit status of 0. The C
/// program instead dies from the signal (status 141 as reported by a shell).
#[cfg(unix)]
fn restore_default_sigpipe() {
    // Linux/glibc: SIGPIPE == 13, SIG_DFL == 0.
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;

    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }

    // SAFETY: `signal` is being called with a valid signal number and the
    // well-known `SIG_DFL` handler value; it has no memory-safety effects.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

/// Emulates `fgets(buf, BUF_SIZE, stdin)`.
///
/// Fills `buf` with the bytes that C would have placed before the terminating
/// NUL. Returns `false` to model a NULL return (EOF/error with no bytes read).
fn fgets<R: BufRead>(reader: &mut R, buf: &mut Vec<u8>) -> bool {
    buf.clear();
    // fgets stores at most size-1 bytes plus the NUL terminator.
    let limit = BUF_SIZE - 1;

    while buf.len() < limit {
        let available = match reader.fill_buf() {
            Ok(chunk) => chunk,
            // glibc retries after EINTR (fgets only reports failure when
            // `errno != EINTR`), so this is not an error path.
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            // A genuine read error sets the stream's error indicator, and
            // glibc's fgets then returns NULL *even if bytes were already
            // stored in the buffer* — the partial data is discarded. The
            // indicator is sticky, so the C loop ends here as well.
            Err(_) => return false,
        };

        if available.is_empty() {
            // End of input.
            break;
        }

        let room = limit - buf.len();
        let take = room.min(available.len());
        // Copy up to and including a newline, whichever comes first.
        let (consumed, saw_newline) = match available[..take].iter().position(|&b| b == b'\n') {
            Some(idx) => (idx + 1, true),
            None => (take, false),
        };

        buf.extend_from_slice(&available[..consumed]);
        reader.consume(consumed);

        if saw_newline {
            break;
        }
    }

    // NULL is returned only when nothing at all was read.
    !buf.is_empty()
}

/// Emulates `fputs(text, stdout)`: writes the C string, i.e. everything up to
/// the first NUL byte.
fn fputs<W: Write>(writer: &mut W, buf: &[u8]) -> io::Result<()> {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    writer.write_all(&buf[..end])
}

fn main() {
    restore_default_sigpipe();

    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());

    let stdout = io::stdout();
    let mut writer = io::BufWriter::with_capacity(STDOUT_BUF_SIZE, stdout.lock());

    let mut text: Vec<u8> = Vec::with_capacity(BUF_SIZE);

    while fgets(&mut reader, &mut text) {
        // C ignores the return value of fputs, so write failures do not stop
        // the loop.
        let _ = fputs(&mut writer, &text);
    }

    let _ = writer.flush();
}
