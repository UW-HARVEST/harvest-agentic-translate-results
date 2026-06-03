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

// interactive echo; ignores arguments, copies stdin to stdout
use std::io::{self, BufRead, Write};

fn main() {
    // Mirror C's fgets(text, 128, stdin) behavior: read up to 127 bytes or
    // until a newline, whichever comes first, and echo each chunk to stdout.
    const BUF_SIZE: usize = 128;
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdin_lock = stdin.lock();
    let mut stdout_lock = stdout.lock();

    let mut buf = [0u8; BUF_SIZE];
    loop {
        let mut len = 0usize;
        // Read one byte at a time, stopping at newline or when buffer holds
        // BUF_SIZE - 1 bytes (leaving room for the implicit terminator that
        // fgets would have written).
        while len < BUF_SIZE - 1 {
            let available = match stdin_lock.fill_buf() {
                Ok(b) => b,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return,
            };
            if available.is_empty() {
                break;
            }
            let byte = available[0];
            buf[len] = byte;
            len += 1;
            stdin_lock.consume(1);
            if byte == b'\n' {
                break;
            }
        }

        if len == 0 {
            // EOF and nothing read: matches fgets returning NULL.
            break;
        }

        if stdout_lock.write_all(&buf[..len]).is_err() {
            return;
        }
        let _ = stdout_lock.flush();
    }
}
