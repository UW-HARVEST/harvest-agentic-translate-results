// Rust translation of c_src/src/main.c
//
// Original C:
//     void driver(const char *s1, const char *s2) {
//         printf("%zu\n", strcspn(s1, s2));
//     }
//
//     int main() {
//         char s1[100] = "", s2[100] = "";
//         fgets(s1, sizeof(s1), stdin);
//         fgets(s2, sizeof(s1), stdin);
//
//         s1[strlen(s1)-1] = '\0';
//         s2[strlen(s2)-1] = '\0';
//
//         driver(s1, s2);
//         return 0;
//     }
//
// Behavior is reproduced exactly, including the original's quirks:
//   * fgets() stops at a newline and never reads past it, so at most one
//     line (99 bytes) is consumed per call; an over-long line leaves its
//     remainder for the next fgets() call.
//   * The unconditional `s[strlen(s)-1] = '\0'` blindly deletes the last
//     byte of the buffer even when that byte is not a newline (truncated
//     long line), and underflows the buffer when the string is empty
//     (EOF before any byte was read). The underflowing store targets a
//     byte outside the string and is not observable in the program's
//     output, so it is emulated as a no-op.
//   * NUL bytes present in the input terminate the string early for
//     strlen()/strcspn() purposes, exactly as in C.

use std::io::{self, Read, Write};

const BUF_LEN: usize = 100; // sizeof(s1) == sizeof(s2) == 100

/// Emulation of C's `fgets(buf, size, stdin)`.
///
/// Reads at most `size - 1` bytes, stopping after a newline (which is kept
/// in the buffer), and NUL-terminates what was read. On immediate EOF the
/// buffer is left untouched, mirroring fgets() returning NULL.
fn fgets<R: Read>(buf: &mut [u8; BUF_LEN], size: usize, input: &mut R) -> bool {
    if size == 0 {
        return false;
    }

    let mut n = 0usize;
    let mut byte = [0u8; 1];

    while n + 1 < size {
        match input.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buf[n] = byte[0];
                n += 1;
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }

    if n == 0 {
        // fgets() returns NULL and does not modify the buffer.
        return false;
    }

    buf[n] = 0;
    true
}

/// Emulation of C's `strlen`: offset of the first NUL byte.
fn strlen(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

/// Emulation of C's `strcspn(s1, s2)`: length of the initial segment of
/// `s1` containing no byte from `s2`. Both are NUL-terminated buffers.
fn strcspn(s1: &[u8], s2: &[u8]) -> usize {
    let reject = &s2[..strlen(s2)];
    let s1 = &s1[..strlen(s1)];

    for (i, c) in s1.iter().enumerate() {
        if reject.contains(c) {
            return i;
        }
    }
    s1.len()
}

fn driver(s1: &[u8], s2: &[u8]) {
    // printf("%zu\n", strcspn(s1, s2));
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", strcspn(s1, s2));
    let _ = out.flush();
}

/// Emulation of `s[strlen(s)-1] = '\0'`.
///
/// When the string is empty this is an out-of-bounds store at `s[-1]` in
/// the original C; that byte lies outside the string and does not affect
/// the program's output, so nothing is written here.
fn chop_last_byte(buf: &mut [u8; BUF_LEN]) {
    let len = strlen(buf);
    if len > 0 {
        buf[len - 1] = 0;
    }
}

fn main() {
    // char s1[100] = "", s2[100] = "";  -> zero filled
    let mut s1 = [0u8; BUF_LEN];
    let mut s2 = [0u8; BUF_LEN];

    let stdin = io::stdin();
    let mut input = io::BufReader::new(stdin.lock());

    // Return values are ignored by the original program.
    let _ = fgets(&mut s1, BUF_LEN, &mut input);
    let _ = fgets(&mut s2, BUF_LEN, &mut input);

    chop_last_byte(&mut s1);
    chop_last_byte(&mut s2);

    driver(&s1, &s2);
}
