//! Small C-stdio compatibility layer.
//!
//! The goal of this module is to reproduce, byte for byte, the observable
//! behaviour of the C standard library calls used by the original program:
//!
//!  * `printf` to a *fully buffered* `stdout` (the usual case when stdout is a
//!    pipe or a file) or a *line buffered* `stdout` (when stdout is a tty),
//!  * `fgets`, `getchar`, `fscanf("%d")` / `scanf("%d")` and `sscanf("%d")`.

use std::io::{self, IsTerminal, Read, Write};

/// glibc's default buffer size for a fully buffered stream.
const BUFSIZ: usize = 4096;

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

/// A `FILE*`-like buffered writer for stdout.
///
/// glibc fills its buffer completely before issuing a `write(2)`, so the exact
/// same byte stream (and the same chunking) is produced here.  Nothing is
/// flushed before reading from stdin, exactly like a fully buffered C stream.
pub struct Out {
    buf: Vec<u8>,
    line_buffered: bool,
}

impl Out {
    pub fn new() -> Out {
        let line_buffered = io::stdout().is_terminal();
        Out {
            buf: Vec::with_capacity(BUFSIZ),
            line_buffered,
        }
    }

    /// `fwrite` of raw bytes (used for strings that came from user input and
    /// therefore are not guaranteed to be valid UTF-8).
    pub fn b(&mut self, data: &[u8]) {
        let has_newline = self.line_buffered && data.contains(&b'\n');
        let mut rest = data;
        while !rest.is_empty() {
            let space = BUFSIZ - self.buf.len();
            let n = if space < rest.len() { space } else { rest.len() };
            self.buf.extend_from_slice(&rest[..n]);
            rest = &rest[n..];
            if self.buf.len() == BUFSIZ {
                self.flush();
            }
        }
        if has_newline {
            self.flush();
        }
    }

    /// `fputs`/`printf` of an already formatted UTF-8 string.
    pub fn s(&mut self, text: &str) {
        self.b(text.as_bytes());
    }

    pub fn flush(&mut self) {
        if !self.buf.is_empty() {
            let stdout = io::stdout();
            let mut lock = stdout.lock();
            let _ = lock.write_all(&self.buf);
            let _ = lock.flush();
            self.buf.clear();
        }
    }
}

/// `printf`-style helper: `p!(out, "fmt", args...)`.
#[macro_export]
macro_rules! p {
    ($out:expr, $($arg:tt)*) => {
        $out.s(&format!($($arg)*))
    };
}

/// `fprintf(stderr, ...)`: stderr is unbuffered in C.
pub fn err_bytes(parts: &[&[u8]]) {
    let stderr = io::stderr();
    let mut lock = stderr.lock();
    for part in parts {
        let _ = lock.write_all(part);
    }
    let _ = lock.flush();
}

pub fn err_str(text: &str) {
    err_bytes(&[text.as_bytes()]);
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

/// A `FILE*`-like buffered reader (used for both stdin and files opened by the
/// program).  The EOF indicator is sticky, just like a C stream: once EOF has
/// been seen, `getchar` keeps returning EOF without touching the fd again.
pub struct CReader<R: Read> {
    src: R,
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
}

impl<R: Read> CReader<R> {
    pub fn new(src: R) -> CReader<R> {
        CReader {
            src,
            buf: Vec::new(),
            pos: 0,
            eof: false,
        }
    }

    fn fill(&mut self) -> bool {
        if self.pos < self.buf.len() {
            return true;
        }
        if self.eof {
            return false;
        }
        self.buf.clear();
        self.pos = 0;
        let mut chunk = [0u8; BUFSIZ];
        loop {
            match self.src.read(&mut chunk) {
                Ok(0) => {
                    self.eof = true;
                    return false;
                }
                Ok(n) => {
                    self.buf.extend_from_slice(&chunk[..n]);
                    return true;
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return false;
                }
            }
        }
    }

    /// `getc`: `None` means EOF.
    pub fn getchar(&mut self) -> Option<u8> {
        if self.fill() {
            let c = self.buf[self.pos];
            self.pos += 1;
            Some(c)
        } else {
            None
        }
    }

    /// Peek at the next byte without consuming it (`getc` + `ungetc`).
    pub fn peek(&mut self) -> Option<u8> {
        if self.fill() {
            Some(self.buf[self.pos])
        } else {
            None
        }
    }

    /// `fgets(buf, size, stream)`.
    ///
    /// Reads at most `size - 1` bytes, stopping after a newline (which is kept
    /// in the buffer).  Returns `None` when nothing at all could be read,
    /// mirroring the `NULL` return of `fgets`.
    pub fn fgets(&mut self, size: usize) -> Option<Vec<u8>> {
        if size == 0 {
            return None;
        }
        let mut out: Vec<u8> = Vec::new();
        while out.len() + 1 < size {
            match self.getchar() {
                Some(c) => {
                    out.push(c);
                    if c == b'\n' {
                        break;
                    }
                }
                None => break,
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    /// `fscanf(stream, "%d", &v)` / `scanf("%d", &v)`.
    ///
    /// `None` covers both a matching failure (return value 0) and an input
    /// failure at EOF (return value EOF); the original program treats both the
    /// same way (`!= 1`).
    pub fn scan_int(&mut self) -> Option<i32> {
        // Leading whitespace is skipped by the %d conversion.
        loop {
            match self.peek() {
                Some(c) if is_space(c) => {
                    self.getchar();
                }
                Some(_) => break,
                None => return None, // input failure (EOF)
            }
        }

        let mut negative = false;
        match self.peek() {
            Some(b'+') => {
                self.getchar();
            }
            Some(b'-') => {
                negative = true;
                self.getchar();
            }
            _ => {}
        }

        let mut magnitude: u64 = 0;
        let mut digits = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.getchar();
                digits = true;
                magnitude = magnitude
                    .saturating_mul(10)
                    .saturating_add((c - b'0') as u64);
            } else {
                break;
            }
        }
        if !digits {
            // Matching failure: the offending character stays in the stream
            // (glibc pushes it back with ungetc).
            return None;
        }
        Some(clamp_to_int(magnitude, negative))
    }

    /// The literal whitespace of a format such as `"%d\n"`: matches zero or
    /// more whitespace characters.
    pub fn skip_format_space(&mut self) {
        while let Some(c) = self.peek() {
            if is_space(c) {
                self.getchar();
            } else {
                break;
            }
        }
    }
}

pub type In = CReader<io::Stdin>;

pub fn stdin_reader() -> In {
    CReader::new(io::stdin())
}

/// `while (getchar() != '\n');`
///
/// NOTE: this faithfully reproduces the behaviour of the original C code.  Once
/// stdin is at EOF, `getchar()` keeps returning `EOF` (never `'\n'`), so the C
/// loop never terminates.  The translation therefore also never returns, and
/// (like the C program) leaves the pending stdout buffer unflushed.
pub fn discard_line(input: &mut In) {
    loop {
        match input.getchar() {
            Some(b'\n') => return,
            Some(_) => {}
            None => spin_forever(),
        }
    }
}

fn spin_forever() -> ! {
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}

/// `sscanf(str, "%d", &v)` over a NUL-terminated C string.
pub fn sscanf_int(s: &[u8]) -> Option<i32> {
    let mut i = 0usize;
    // The C string ends at the first NUL byte.
    let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    let s = &s[..end];

    while i < s.len() && is_space(s[i]) {
        i += 1;
    }
    if i >= s.len() {
        return None;
    }
    let mut negative = false;
    if s[i] == b'+' {
        i += 1;
    } else if s[i] == b'-' {
        negative = true;
        i += 1;
    }
    let mut magnitude: u64 = 0;
    let mut digits = false;
    while i < s.len() && s[i].is_ascii_digit() {
        digits = true;
        magnitude = magnitude
            .saturating_mul(10)
            .saturating_add((s[i] - b'0') as u64);
        i += 1;
    }
    if !digits {
        return None;
    }
    Some(clamp_to_int(magnitude, negative))
}

/// glibc clamps an out-of-range `%d` conversion to `LONG_MIN`/`LONG_MAX` and
/// then stores the (truncated) `int`.
fn clamp_to_int(magnitude: u64, negative: bool) -> i32 {
    let value: i64 = if negative {
        if magnitude > (i64::MAX as u64) + 1 {
            i64::MIN
        } else if magnitude == (i64::MAX as u64) + 1 {
            i64::MIN
        } else {
            -(magnitude as i64)
        }
    } else if magnitude > i64::MAX as u64 {
        i64::MAX
    } else {
        magnitude as i64
    };
    value as i32
}

pub fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

// ---------------------------------------------------------------------------
// C string helpers
// ---------------------------------------------------------------------------

/// The bytes of a C string held in a `char[]` buffer: everything up to the
/// first NUL byte.
pub fn c_str(buf: &[u8]) -> &[u8] {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    &buf[..end]
}

/// `buf[strcspn(buf, "\n")] = 0;` applied to the C string in `buf`.
pub fn strip_newline(buf: &[u8]) -> Vec<u8> {
    let s = c_str(buf);
    let end = s.iter().position(|&c| c == b'\n').unwrap_or(s.len());
    s[..end].to_vec()
}

/// `strncpy(dst, src, n - 1); dst[n - 1] = '\0';` for a `char[n]` destination.
pub fn truncate_to_buffer(src: &[u8], buffer_size: usize) -> Vec<u8> {
    let s = c_str(src);
    let max = buffer_size - 1;
    if s.len() > max {
        s[..max].to_vec()
    } else {
        s.to_vec()
    }
}
