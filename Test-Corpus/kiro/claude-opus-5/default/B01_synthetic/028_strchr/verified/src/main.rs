// Rust translation of c_src/src/main.c
//
// Original C:
//   char in[1000] = "";              // 1000 zeroed bytes
//   fread(in, 1, sizeof(in), stdin); // read up to 1000 raw bytes, no NUL added
//   driver(in);                      // treated as a NUL-terminated C string
//
// Behaviour preserved exactly, including the quirks:
//   * raw byte reads (no line semantics) that cross newlines,
//   * the buffer is capped at 1000 bytes; anything beyond is ignored,
//   * an embedded NUL byte in the input terminates the "string" early,
//     so characters after it are not counted,
//   * `strchr` scanning is a plain byte comparison (no UTF-8 awareness).

use std::io::{self, Read, Write};

const BUF_SIZE: usize = 1000;

/// Equivalent of the C `foo`: counts occurrences of `c` in the NUL-terminated
/// string `in_`, using repeated `strchr` calls.
///
/// `in_` is the byte slice up to (excluding) the terminating NUL, mirroring
/// what `strchr` would traverse.
fn foo(in_: &[u8], c: u8) -> i32 {
    let mut res: i32 = 0;
    let mut s: usize = 0;
    // `for (const char *s = in; s = strchr(s, c); s++) res++;`
    while s <= in_.len() {
        match strchr(in_, s, c) {
            Some(found) => {
                res = res.wrapping_add(1);
                // loop increment: s++ past the found character
                s = found + 1;
            }
            None => break,
        }
    }
    res
}

/// `strchr(in_ + from, c)` returning the index of the match, if any.
/// Note: the C string's terminating NUL is part of the searched region, so
/// searching for b'\0' would match it; the callers here only pass 'A'/'x'.
fn strchr(in_: &[u8], from: usize, c: u8) -> Option<usize> {
    if from > in_.len() {
        return None;
    }
    if c == 0 {
        // strchr matches the terminating NUL itself.
        return Some(in_.len());
    }
    in_[from..].iter().position(|&b| b == c).map(|i| from + i)
}

fn driver(in_: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "A: {}\n", foo(in_, b'A'));
    let _ = write!(out, "x: {}\n", foo(in_, b'x'));
    let _ = out.flush();
}

fn main() {
    // char in[1000] = "";  -> all 1000 bytes zeroed
    let mut buf = [0u8; BUF_SIZE];

    // fread(in, 1, sizeof(in), stdin): fill up to BUF_SIZE bytes, stopping at
    // EOF (or error, whose return value the C code ignores).
    let mut filled = 0usize;
    let mut stdin = io::stdin();
    while filled < BUF_SIZE {
        match stdin.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }

    // The C code then treats the buffer as a C string: everything up to the
    // first NUL byte. Because the array was zero-initialised, any unread tail
    // provides the terminator.
    let end = buf.iter().position(|&b| b == 0).unwrap_or(BUF_SIZE);
    driver(&buf[..end]);
}
