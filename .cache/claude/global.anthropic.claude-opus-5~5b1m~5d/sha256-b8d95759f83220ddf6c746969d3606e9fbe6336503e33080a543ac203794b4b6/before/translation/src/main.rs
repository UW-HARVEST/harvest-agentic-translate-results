// Rust translation of c_src/src/main.c
//
// Original C:
//   int foo(const char *in, char c) — counts occurrences of `c` in the
//   NUL-terminated string `in` using repeated strchr().
//   void driver(const char *in) — prints the counts of 'A' and 'x'.
//   int main() — zero-initialized char in[1000], fread(in, 1, 1000, stdin).

use std::io::{Read, Write};

/// Equivalent of the C `foo`: number of occurrences of byte `c` in the
/// NUL-terminated byte string `s` (the slice passed in is already truncated
/// at the first NUL, mirroring what strchr() would traverse).
fn foo(s: &[u8], c: u8) -> i32 {
    let mut res: i32 = 0;
    for &b in s {
        if b == c {
            res += 1;
        }
    }
    res
}

fn driver(s: &[u8]) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "A: {}\n", foo(s, b'A'));
    let _ = write!(out, "x: {}\n", foo(s, b'x'));
    let _ = out.flush();
}

fn main() {
    // char in[1000] = ""; -> 1000 zero bytes
    let mut buf = [0u8; 1000];

    // fread(in, 1, sizeof(in), stdin): read up to 1000 bytes, looping until
    // the buffer is full or EOF (short reads are retried, as fread does).
    let mut stdin = std::io::stdin();
    let mut filled = 0usize;
    while filled < buf.len() {
        match stdin.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }

    // The C code then treats `in` as a NUL-terminated string.
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    driver(&buf[..end]);
}
