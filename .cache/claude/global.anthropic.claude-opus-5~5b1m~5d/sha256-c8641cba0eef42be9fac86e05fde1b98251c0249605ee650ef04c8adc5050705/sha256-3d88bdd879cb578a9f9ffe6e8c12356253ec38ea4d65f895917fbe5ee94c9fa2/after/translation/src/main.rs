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

/// Emulation of C's `fgets(buf, size, stdin)`.
///
/// Reads at most `size - 1` bytes, stopping early after a newline (which is
/// kept) or at end-of-file. The C version NUL-terminates the buffer; here the
/// bytes actually read are returned instead, and the terminator is modelled by
/// the caller (`fputs` stops at the first NUL byte).
///
/// Returns `None` when no bytes could be read (EOF with an empty buffer) or on
/// a read error, exactly like `fgets` returning `NULL`.
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
        let available = loop {
            match reader.fill_buf() {
                Ok(buf) => break buf,
                // Retry on EINTR, as stdio does.
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                // A read error makes `fgets` return NULL.
                Err(_) => return None,
            }
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

/// Emulation of C's `fputs(text, stdout)` on a NUL-terminated buffer: writes
/// the bytes up to (but not including) the first NUL byte.
fn fputs<W: Write>(writer: &mut W, text: &[u8]) -> io::Result<()> {
    let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    writer.write_all(&text[..end])
}

fn main() -> ExitCode {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());

    let stdout = io::stdout();
    // C stdio is line buffered on a terminal and fully buffered otherwise.
    let line_buffered = stdout.is_terminal();
    let mut writer = io::BufWriter::with_capacity(64 * 1024, stdout.lock());

    while let Some(text) = fgets(&mut reader, TEXT_LEN) {
        if fputs(&mut writer, &text).is_err() {
            // Mirror stdio: a write error is not reported by this program.
            break;
        }
        if line_buffered && text.contains(&b'\n') && writer.flush().is_err() {
            break;
        }
    }

    let _ = writer.flush();
    ExitCode::from(0)
}
