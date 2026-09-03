// C stdio emulation layer.
//
// Reproduces the observable behaviour of glibc's stdio as used by the original
// C program:
//   * `stdout` is fully buffered with a 4096 byte buffer (the behaviour when
//     stdout is a pipe or a regular file), flushed at exit.
//   * `stderr` is unbuffered.
//   * `fgets()` stops at a newline (which it keeps) or after n-1 bytes.
//   * `scanf("%d")` skips leading whitespace, accepts an optional sign, and
//     pushes back the first non-matching character.
//   * `getchar()` returns EOF forever once the stream hit end of file.

use std::io::{BufReader, Read, Write};

pub const BUFSIZ: usize = 4096;

// ---------------------------------------------------------------------------
// stdout
// ---------------------------------------------------------------------------

pub struct Out {
    buf: Vec<u8>,
}

impl Out {
    pub fn new() -> Out {
        Out {
            buf: Vec::with_capacity(BUFSIZ * 2),
        }
    }

    pub fn put(&mut self, s: &str) {
        self.put_bytes(s.as_bytes());
    }

    pub fn put_bytes(&mut self, b: &[u8]) {
        self.buf.extend_from_slice(b);
        // glibc drains the stream in whole-buffer sized chunks.
        while self.buf.len() >= BUFSIZ {
            Self::raw_write(&self.buf[..BUFSIZ]);
            self.buf.drain(..BUFSIZ);
        }
    }

    pub fn flush(&mut self) {
        if !self.buf.is_empty() {
            Self::raw_write(&self.buf);
            self.buf.clear();
        }
    }

    fn raw_write(bytes: &[u8]) {
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        let _ = lock.write_all(bytes);
        let _ = lock.flush();
    }
}

// ---------------------------------------------------------------------------
// stderr (unbuffered)
// ---------------------------------------------------------------------------

pub fn err_put(s: &str) {
    err_put_bytes(s.as_bytes());
}

pub fn err_put_bytes(b: &[u8]) {
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    let _ = lock.write_all(b);
    let _ = lock.flush();
}

/// glibc's printf("%p"): "0x" followed by lowercase hex, or "(nil)" for NULL.
pub fn fmt_ptr(p: usize) -> String {
    if p == 0 {
        "(nil)".to_string()
    } else {
        format!("0x{:x}", p)
    }
}

// ---------------------------------------------------------------------------
// stdin
// ---------------------------------------------------------------------------

pub enum Scan {
    /// A value was converted (scanf returned 1).
    Val(i32),
    /// Matching failure or input failure (scanf returned 0 or EOF).
    Fail,
}

pub struct In<R: Read> {
    reader: R,
    pushback: Option<u8>,
    eof: bool,
}

fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

impl In<BufReader<std::io::Stdin>> {
    pub fn stdin() -> In<BufReader<std::io::Stdin>> {
        In {
            reader: BufReader::new(std::io::stdin()),
            pushback: None,
            eof: false,
        }
    }
}

impl In<std::io::Cursor<Vec<u8>>> {
    /// A stream over an already-read file body.
    pub fn from_bytes(bytes: Vec<u8>) -> In<std::io::Cursor<Vec<u8>>> {
        In {
            reader: std::io::Cursor::new(bytes),
            pushback: None,
            eof: false,
        }
    }
}

impl<R: Read> In<R> {
    /// getchar(): None models EOF. The EOF flag is sticky, as in C.
    pub fn getc(&mut self) -> Option<u8> {
        if let Some(c) = self.pushback.take() {
            return Some(c);
        }
        if self.eof {
            return None;
        }
        let mut b = [0u8; 1];
        match self.reader.read(&mut b) {
            Ok(0) => {
                self.eof = true;
                None
            }
            Ok(_) => Some(b[0]),
            Err(_) => {
                self.eof = true;
                None
            }
        }
    }

    /// ungetc(): a single character of pushback, like the C library guarantees.
    fn ungetc(&mut self, c: u8) {
        self.pushback = Some(c);
    }

    /// `while (getchar() != '\n');`
    ///
    /// Note that this loops forever at end of file, exactly as the C code does:
    /// getchar() keeps returning EOF, which never compares equal to '\n'.
    pub fn eat_until_newline(&mut self) {
        loop {
            match self.getc() {
                Some(b'\n') => return,
                Some(_) => continue,
                None => continue,
            }
        }
    }

    /// fgets(s, n, stdin). Returns None for the NULL return (EOF with nothing
    /// read). The returned buffer keeps the trailing newline when one was read.
    pub fn fgets(&mut self, n: usize) -> Option<Vec<u8>> {
        if n == 0 {
            return None;
        }
        let mut out: Vec<u8> = Vec::new();
        while out.len() + 1 < n {
            match self.getc() {
                None => {
                    if out.is_empty() {
                        return None;
                    }
                    break;
                }
                Some(b'\n') => {
                    out.push(b'\n');
                    break;
                }
                Some(c) => out.push(c),
            }
        }
        Some(out)
    }

    /// scanf("%d", &x)
    pub fn scan_int(&mut self) -> Scan {
        // Skip leading whitespace (newlines included).
        loop {
            match self.getc() {
                None => return Scan::Fail,
                Some(c) if is_c_space(c) => continue,
                Some(c) => {
                    self.ungetc(c);
                    break;
                }
            }
        }

        let mut neg = false;
        match self.getc() {
            None => return Scan::Fail,
            Some(b'-') => neg = true,
            Some(b'+') => {}
            Some(c) => self.ungetc(c),
        }

        let mut any = false;
        let mut acc: i64 = 0;
        let mut saturated = false;
        loop {
            match self.getc() {
                None => break,
                Some(c) if c.is_ascii_digit() => {
                    any = true;
                    let d = (c - b'0') as i64;
                    if !saturated {
                        match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                            Some(v) => acc = v,
                            None => saturated = true,
                        }
                    }
                }
                Some(c) => {
                    self.ungetc(c);
                    break;
                }
            }
        }

        if !any {
            return Scan::Fail;
        }
        Scan::Val(finish_int(neg, acc, saturated))
    }
}

/// glibc converts via the strtol family (which saturates at LONG_MAX /
/// LONG_MIN) and then stores into an `int`, truncating.
fn finish_int(neg: bool, acc: i64, saturated: bool) -> i32 {
    let v: i64 = if saturated {
        if neg {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if neg {
        acc.wrapping_neg()
    } else {
        acc
    };
    v as i32
}

/// sscanf(buf, "%d", &x) over an in-memory buffer.
pub fn sscanf_int(bytes: &[u8]) -> Option<i32> {
    // The buffer is a C string: it ends at the first NUL byte.
    let s: &[u8] = match bytes.iter().position(|&c| c == 0) {
        Some(i) => &bytes[..i],
        None => bytes,
    };

    let mut i = 0usize;
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }
    if i >= s.len() {
        return None;
    }
    let mut neg = false;
    if s[i] == b'-' {
        neg = true;
        i += 1;
    } else if s[i] == b'+' {
        i += 1;
    }
    let mut any = false;
    let mut acc: i64 = 0;
    let mut saturated = false;
    while i < s.len() && s[i].is_ascii_digit() {
        any = true;
        let d = (s[i] - b'0') as i64;
        if !saturated {
            match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => acc = v,
                None => saturated = true,
            }
        }
        i += 1;
    }
    if !any {
        return None;
    }
    Some(finish_int(neg, acc, saturated))
}

// ---------------------------------------------------------------------------
// C string helpers
// ---------------------------------------------------------------------------

/// Emulates `buf[strcspn(buf, "\n")] = 0` applied to the C string held in
/// `buf`: the result is the bytes up to the first newline or NUL.
pub fn trim_at_newline(buf: &[u8]) -> Vec<u8> {
    let end = buf
        .iter()
        .position(|&c| c == b'\n' || c == 0)
        .unwrap_or(buf.len());
    buf[..end].to_vec()
}

/// The bytes printf("%s") would emit for a C string held in `bytes`.
pub fn c_str_bytes(bytes: &[u8]) -> &[u8] {
    let end = bytes.iter().position(|&c| c == 0).unwrap_or(bytes.len());
    &bytes[..end]
}
