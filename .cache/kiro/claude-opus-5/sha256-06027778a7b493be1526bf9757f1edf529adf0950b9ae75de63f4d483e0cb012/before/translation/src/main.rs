// Rust translation of c_src/src/main.c
//
// Behavior is intentionally identical to the original C program, including its
// bugs: a negative value read from stdin leads to `strncpy(dest, source, data)`
// with a huge `size_t` length plus a `dest[data]` underwrite, which crashes the
// process with SIGSEGV before the (fully buffered) stdout is flushed.

use std::io::{Read, Write};

extern "C" {
    fn isatty(fd: i32) -> i32;
    fn raise(sig: i32) -> i32;
    fn signal(sig: i32, handler: usize) -> usize;
}

const SIGSEGV: i32 = 11;
const SIG_DFL: usize = 0;

/// Emulates C's stdio buffering for stdout: fully buffered when stdout is not a
/// terminal, line buffered when it is. This matters because the original
/// program can die by signal with pending buffered output, which is then lost.
struct CStdout {
    buf: Vec<u8>,
    line_buffered: bool,
}

impl CStdout {
    fn new() -> Self {
        let line_buffered = unsafe { isatty(1) } == 1;
        CStdout {
            buf: Vec::new(),
            line_buffered,
        }
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
        if self.line_buffered && bytes.contains(&b'\n') {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        let out = std::io::stdout();
        let mut lock = out.lock();
        let _ = lock.write_all(&self.buf);
        let _ = lock.flush();
        self.buf.clear();
    }
}

/// `printf("%s\n", line)` over a NUL-terminated byte buffer.
fn print_line(out: &mut CStdout, line: &[u8]) {
    let len = cstr_len(line);
    out.write_bytes(&line[..len]);
    out.write_bytes(b"\n");
}

fn cstr_len(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

/// `fgets(buf, size, stdin)`: reads at most `size - 1` bytes, stops after a
/// newline (which is kept), NUL-terminates, and returns false (NULL) only when
/// no bytes at all could be read.
fn fgets(buf: &mut [u8], size: usize) -> bool {
    let max = size.saturating_sub(1);
    let stdin = std::io::stdin();
    let mut lock = stdin.lock();
    let mut count = 0usize;
    let mut byte = [0u8; 1];

    while count < max {
        let n = loop {
            match lock.read(&mut byte) {
                Ok(n) => break n,
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break 0,
            }
        };
        if n == 0 {
            break; // EOF (or error)
        }
        buf[count] = byte[0];
        count += 1;
        if byte[0] == b'\n' {
            break;
        }
    }

    if count == 0 {
        return false;
    }
    buf[count] = 0;
    true
}

/// glibc's `atoi`: `(int) strtol(nptr, NULL, 10)`, i.e. long-sized parsing with
/// saturation, then truncation to `int`.
fn atoi(buf: &[u8]) -> i32 {
    let s = &buf[..cstr_len(buf)];
    let mut i = 0usize;

    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let mut acc: i64 = 0;
    let mut saturated = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = (s[i] - b'0') as i64;
        if !saturated {
            match acc.checked_mul(10).and_then(|v| {
                if negative {
                    v.checked_sub(digit)
                } else {
                    v.checked_add(digit)
                }
            }) {
                Some(v) => acc = v,
                None => {
                    saturated = true;
                    acc = if negative { i64::MIN } else { i64::MAX };
                }
            }
        }
        i += 1;
    }

    acc as i32
}

/// `strncpy(dest, src, n)`: copies up to `n` bytes from `src`, stopping at
/// `src`'s NUL, then zero-pads `dest` up to `n` bytes.
fn strncpy(dest: &mut [u8], src: &[u8], n: usize) {
    let src_len = cstr_len(src);
    let copy = src_len.min(n);
    dest[..copy].copy_from_slice(&src[..copy]);
    for slot in dest[copy..n].iter_mut() {
        *slot = 0;
    }
}

/// The original program's undefined behavior for a negative `data`: glibc's
/// `strncpy` receives `(size_t) data` (a huge length) and walks off the stack,
/// killing the process with SIGSEGV. Buffered stdout is never flushed.
fn die_like_c_overflow() -> ! {
    unsafe {
        signal(SIGSEGV, SIG_DFL);
        raise(SIGSEGV);
    }
    // Unreachable in practice; keeps the function divergent.
    std::process::abort();
}

fn main() {
    let mut out = CStdout::new();

    let mut data: i32 = -1;
    {
        let mut input_buffer = [0u8; 14];
        if fgets(&mut input_buffer, 14) {
            /* Convert to int */
            data = atoi(&input_buffer);
        } else {
            print_line(&mut out, b"fgets() failed.\0");
        }
    }
    {
        let mut source = [0u8; 100];
        let mut dest = [0u8; 100];
        for slot in source[..100 - 1].iter_mut() {
            *slot = b'A';
        }
        source[100 - 1] = 0;
        if data < 100 {
            if data < 0 {
                die_like_c_overflow();
            }
            let n = data as usize;
            strncpy(&mut dest, &source, n);
            dest[n] = 0;
        }
        print_line(&mut out, &dest);
    }

    out.flush();
}
