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

use std::io::{self, BufRead, BufReader, Write};

/// Size of the C `char text[128]` buffer.
const BUF_SIZE: usize = 128;

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
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            // A read error behaves like EOF here: fgets returns NULL unless
            // bytes were already stored in the buffer.
            Err(_) => break,
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
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());

    let stdout = io::stdout();
    let mut writer = io::BufWriter::new(stdout.lock());

    let mut text: Vec<u8> = Vec::with_capacity(BUF_SIZE);

    while fgets(&mut reader, &mut text) {
        // C ignores the return value of fputs, so write failures do not stop
        // the loop.
        let _ = fputs(&mut writer, &text);
    }

    let _ = writer.flush();
}
