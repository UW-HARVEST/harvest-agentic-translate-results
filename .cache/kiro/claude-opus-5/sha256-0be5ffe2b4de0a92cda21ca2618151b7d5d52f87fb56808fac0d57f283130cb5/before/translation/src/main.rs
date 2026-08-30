// Rust translation of c_src/src/main.c
//
// Original C:
//     char s1[100] = "", s2[100] = "";
//     fgets(s1, sizeof(s1), stdin);
//     fgets(s2, sizeof(s1), stdin);
//     s1[strlen(s1)-1] = '\0';
//     s2[strlen(s2)-1] = '\0';
//     printf("%zu\n", strcspn(s1, s2));
//
// Semantics preserved verbatim (no bug fixes):
//   * Both buffers are 100 bytes, zero filled, so a failed fgets leaves "".
//   * fgets keeps the trailing '\n' and stops after at most 99 bytes; NUL bytes
//     from the input are stored but terminate the C string for strlen/strcspn.
//   * The unconditional `s[strlen(s)-1] = '\0'` truncates the last byte of the
//     C string, which is the newline for a normal line, but the last data byte
//     when the line was truncated at 99 bytes or ended at EOF without '\n'.
//   * When strlen(s) == 0 the C code writes s[-1], which is out of bounds. The
//     byte preceding either buffer is always 0 in this program (fgets can only
//     ever place a NUL, never data, at index 99 of either array), so the write
//     is unobservable and is simply skipped here.

use std::io::{self, Read, Write};

const BUF_LEN: usize = 100;

/// Emulates `fgets(buf, BUF_LEN, stdin)`: stores at most BUF_LEN - 1 bytes,
/// stops after a '\n' (which is kept), and NUL terminates. Returns false when
/// EOF was hit before any byte was read, in which case `buf` is left untouched.
fn fgets<R: Read>(buf: &mut [u8; BUF_LEN], input: &mut R) -> bool {
    let mut n = 0usize;
    let mut byte = [0u8; 1];
    while n < BUF_LEN - 1 {
        match input.read(&mut byte) {
            Ok(0) => break,
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
        return false;
    }
    buf[n] = 0;
    true
}

/// `strlen`: bytes before the first NUL.
fn strlen(buf: &[u8; BUF_LEN]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(BUF_LEN)
}

/// `s[strlen(s) - 1] = '\0'`. See the note above about the len == 0 case.
fn chop_last(buf: &mut [u8; BUF_LEN]) {
    let len = strlen(buf);
    if len > 0 {
        buf[len - 1] = 0;
    }
}

/// `strcspn(s1, s2)`: length of the initial run of s1 made of bytes not in s2.
fn strcspn(s1: &[u8; BUF_LEN], s2: &[u8; BUF_LEN]) -> usize {
    let reject = &s2[..strlen(s2)];
    let hay = &s1[..strlen(s1)];
    for (i, b) in hay.iter().enumerate() {
        if reject.contains(b) {
            return i;
        }
    }
    hay.len()
}

fn driver(s1: &[u8; BUF_LEN], s2: &[u8; BUF_LEN]) {
    let out = io::stdout();
    let mut out = out.lock();
    let _ = write!(out, "{}\n", strcspn(s1, s2));
    let _ = out.flush();
}

fn main() {
    let mut s1 = [0u8; BUF_LEN];
    let mut s2 = [0u8; BUF_LEN];

    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    fgets(&mut s1, &mut stdin);
    fgets(&mut s2, &mut stdin);

    chop_last(&mut s1);
    chop_last(&mut s2);

    driver(&s1, &s2);
}
