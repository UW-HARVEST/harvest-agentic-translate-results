//! Emulation of the small subset of C's <stdio.h> / <stdlib.h> behaviour that
//! the original program relies on:
//!
//!   * `fgets()`   - stops at (and keeps) a newline, never reads past it
//!   * `getchar()` - single byte reads from the very same stream position
//!   * `scanf("%d")` / `fscanf("%d")` / `sscanf("%d")` - whitespace skipping,
//!     `long`-width accumulation with saturation, truncation to `int`, and the
//!     push back of the first non matching character
//!   * a fully buffered `stdout` (which is what glibc uses when stdout is not a
//!     terminal), so that the byte stream produced on stdout is identical and
//!     unflushed data is lost when the process is killed - exactly like the C
//!     original.

use std::io::{self, Read, Write};

pub const EOF: i32 = -1;

/// `isspace()` in the "C" locale.
pub fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// A source of characters that supports pushing one character back, mirroring
/// the `FILE*` behaviour used by the C scanf family.
pub trait CharSrc {
    fn next_char(&mut self) -> i32;
    fn unget_char(&mut self, c: u8);
}

/// The program's `stdin` (a `FILE*` in C).  Bytes are pulled from the real
/// stdin in blocks, so a byte is never consumed before it is needed.
pub struct CStdin {
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
    pushback: Option<u8>,
}

impl CStdin {
    pub fn new() -> CStdin {
        CStdin {
            buf: Vec::new(),
            pos: 0,
            eof: false,
            pushback: None,
        }
    }

    /// Make sure at least one byte is available; `false` means end of file.
    fn fill(&mut self) -> bool {
        if self.pos < self.buf.len() {
            return true;
        }
        if self.eof {
            // The EOF indicator of a C stream is sticky until clearerr().
            return false;
        }
        let mut tmp = [0u8; 4096];
        loop {
            match io::stdin().read(&mut tmp) {
                Ok(0) => {
                    self.eof = true;
                    return false;
                }
                Ok(n) => {
                    self.buf.clear();
                    self.buf.extend_from_slice(&tmp[..n]);
                    self.pos = 0;
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

    /// `getchar()`
    pub fn getchar(&mut self) -> i32 {
        if let Some(c) = self.pushback.take() {
            return c as i32;
        }
        if !self.fill() {
            return EOF;
        }
        let c = self.buf[self.pos];
        self.pos += 1;
        c as i32
    }

    /// `fgets(buf, size, stdin)`.  Returns the bytes that were stored (the
    /// terminating NUL is not part of the returned data) or `None` when NULL
    /// would have been returned.
    pub fn fgets(&mut self, size: usize) -> Option<Vec<u8>> {
        if size == 0 {
            return None;
        }
        let max = size - 1;
        let mut out: Vec<u8> = Vec::new();
        while out.len() < max {
            let c = self.getchar();
            if c == EOF {
                break;
            }
            out.push(c as u8);
            if c == b'\n' as i32 {
                break;
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    /// `scanf("%d", &x)`: `Some(v)` when the conversion succeeded (return value
    /// 1), `None` for a matching failure or EOF (return value 0 or -1).
    pub fn scanf_int(&mut self) -> Option<i32> {
        scan_int(self)
    }

    /// `while (getchar() != '\n');`
    ///
    /// At end of file glibc's `getchar()` keeps returning EOF, so the original
    /// C program loops forever here.  That behaviour is reproduced (without
    /// burning a CPU core): no further output is produced and the pending
    /// stdout buffer is never flushed.
    pub fn discard_line(&mut self) {
        loop {
            let c = self.getchar();
            if c == b'\n' as i32 {
                return;
            }
            if c == EOF {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(3600));
                }
            }
        }
    }
}

impl CharSrc for CStdin {
    fn next_char(&mut self) -> i32 {
        self.getchar()
    }
    fn unget_char(&mut self, c: u8) {
        self.pushback = Some(c);
    }
}

/// A `FILE*` opened on an in memory byte buffer (used for the scene files) and
/// also the backing store for `sscanf()`.
pub struct ByteSrc<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ByteSrc<'a> {
    pub fn new(data: &'a [u8]) -> ByteSrc<'a> {
        ByteSrc { data, pos: 0 }
    }

    /// `fgets(buf, size, file)`
    pub fn fgets(&mut self, size: usize) -> Option<Vec<u8>> {
        if size == 0 {
            return None;
        }
        let max = size - 1;
        let mut out: Vec<u8> = Vec::new();
        while out.len() < max && self.pos < self.data.len() {
            let c = self.data[self.pos];
            self.pos += 1;
            out.push(c);
            if c == b'\n' {
                break;
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    /// `fscanf(file, "%d\n", &x)` - the trailing whitespace directive consumes
    /// any run of whitespace that follows the converted number.
    pub fn fscanf_int_nl(&mut self) -> Option<i32> {
        let v = scan_int(self);
        // Whitespace directive: skip whitespace, push back the first non-space.
        loop {
            let c = self.next_char();
            if c == EOF {
                break;
            }
            if !is_space(c as u8) {
                self.unget_char(c as u8);
                break;
            }
        }
        v
    }
}

impl<'a> CharSrc for ByteSrc<'a> {
    fn next_char(&mut self) -> i32 {
        if self.pos < self.data.len() {
            let c = self.data[self.pos];
            self.pos += 1;
            c as i32
        } else {
            self.pos = self.data.len() + 1; // remember that EOF was hit
            EOF
        }
    }
    fn unget_char(&mut self, _c: u8) {
        if self.pos > 0 && self.pos <= self.data.len() {
            self.pos -= 1;
        }
    }
}

/// `sscanf(s, "%d", &x)`
pub fn sscanf_int(s: &[u8]) -> Option<i32> {
    let mut src = ByteSrc::new(s);
    scan_int(&mut src)
}

/// The `%d` conversion of the C scanf family: an optionally signed decimal
/// number accumulated with `long` width (saturating on overflow, like strtol)
/// and finally truncated to `int`.
fn scan_int<S: CharSrc + ?Sized>(src: &mut S) -> Option<i32> {
    let mut c = src.next_char();
    while c != EOF && is_space(c as u8) {
        c = src.next_char();
    }
    if c == EOF {
        return None; // input failure -> scanf returns EOF
    }

    let mut neg = false;
    if c == b'+' as i32 || c == b'-' as i32 {
        neg = c == b'-' as i32;
        c = src.next_char();
    }

    let mut digits = 0usize;
    let mut acc: i128 = 0;
    let mut over = false;
    while c != EOF && (c as u8).is_ascii_digit() {
        digits += 1;
        if !over {
            acc = acc * 10 + ((c as u8) - b'0') as i128;
            if acc > (i64::MAX as i128) + 1 {
                over = true;
            }
        }
        c = src.next_char();
    }

    // The character that stopped the conversion is pushed back onto the stream.
    if c != EOF {
        src.unget_char(c as u8);
    }

    if digits == 0 {
        return None; // matching failure -> scanf returns 0
    }

    let v: i64 = if over {
        if neg {
            i64::MIN
        } else {
            i64::MAX
        }
    } else {
        let signed: i128 = if neg { -acc } else { acc };
        if signed > i64::MAX as i128 {
            i64::MAX
        } else if signed < i64::MIN as i128 {
            i64::MIN
        } else {
            signed as i64
        }
    };
    Some(v as i32)
}

/// The fully buffered `stdout` of the C program.
///
/// glibc gives a non-tty stdout a buffer of one block (4096 bytes) and flushes
/// it only when it is completely full, splitting a write across the boundary if
/// necessary.  Reproducing that exactly means the bytes that reach fd 1 at any
/// point in time - and therefore also the output that survives when the process
/// is killed before its buffer is flushed - are the same as in C.
pub struct COut {
    buf: Vec<u8>,
    cap: usize,
}

impl COut {
    pub fn new() -> COut {
        COut {
            buf: Vec::with_capacity(4096),
            cap: 4096,
        }
    }

    /// `printf`-style raw byte output (errors are ignored, just like the C code
    /// ignores printf's return value).
    pub fn put(&mut self, bytes: &[u8]) {
        let mut rest = bytes;
        while !rest.is_empty() {
            let space = self.cap - self.buf.len();
            let n = if space < rest.len() { space } else { rest.len() };
            self.buf.extend_from_slice(&rest[..n]);
            rest = &rest[n..];
            if self.buf.len() >= self.cap {
                self.flush();
            }
        }
    }

    pub fn puts(&mut self, s: &str) {
        self.put(s.as_bytes());
    }

    pub fn flush(&mut self) {
        if !self.buf.is_empty() {
            write_fd1(&self.buf);
            self.buf.clear();
        }
    }
}

/// Write straight to file descriptor 1, bypassing Rust's own (line buffered)
/// stdout so that the buffering behaviour above is the only one in effect.
#[cfg(unix)]
fn write_fd1(data: &[u8]) {
    use std::os::unix::io::FromRawFd;
    let file = unsafe { std::fs::File::from_raw_fd(1) };
    let mut file = std::mem::ManuallyDrop::new(file); // must not close fd 1
    let _ = file.write_all(data);
}

#[cfg(not(unix))]
fn write_fd1(data: &[u8]) {
    let out = io::stdout();
    let mut lock = out.lock();
    let _ = lock.write_all(data);
    let _ = lock.flush();
}

/// `fprintf(stderr, ...)` - stderr is unbuffered in C.
pub fn err(s: &[u8]) {
    let mut e = io::stderr();
    let _ = e.write_all(s);
    let _ = e.flush();
}

/// Truncate at the first NUL byte: the bytes that `%s` / `strlen` would see.
pub fn c_str(bytes: &[u8]) -> &[u8] {
    match bytes.iter().position(|&b| b == 0) {
        Some(i) => &bytes[..i],
        None => bytes,
    }
}

/// `buf[strcspn(buf, "\n")] = 0;` applied to a freshly `fgets`ed buffer.
pub fn strip_newline(bytes: &[u8]) -> Vec<u8> {
    let s = c_str(bytes);
    match s.iter().position(|&b| b == b'\n') {
        Some(i) => s[..i].to_vec(),
        None => s.to_vec(),
    }
}
