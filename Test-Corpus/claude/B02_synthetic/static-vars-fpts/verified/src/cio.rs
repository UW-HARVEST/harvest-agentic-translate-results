//! C standard-I/O emulation helpers: buffered stdout (matching glibc's
//! block/line buffering behaviour), unbuffered stderr, `fgets`, `sscanf("%d")`,
//! `strncat`, `strstr` and the `<ctype.h>` classification functions for the
//! "C" locale.

use std::io::{self, IsTerminal, Read, Write};

// ---------------------------------------------------------------------------
// ctype.h (C locale)
// ---------------------------------------------------------------------------

pub fn c_isspace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

pub fn c_isalpha(c: u8) -> bool {
    c.is_ascii_alphabetic()
}

pub fn c_isdigit(c: u8) -> bool {
    c.is_ascii_digit()
}

pub fn c_isalnum(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

// ---------------------------------------------------------------------------
// C string helpers
// ---------------------------------------------------------------------------

/// The portion of `s` that a C string function would see (up to the first NUL).
pub fn up_to_nul(s: &[u8]) -> &[u8] {
    match s.iter().position(|&b| b == 0) {
        Some(i) => &s[..i],
        None => s,
    }
}

/// `strncat(dst, src, n)` where `dst`/`src` are NUL-terminated C strings.
pub fn strncat(dst: &mut Vec<u8>, src: &[u8], n: usize) {
    let s = up_to_nul(src);
    let take = if s.len() < n { s.len() } else { n };
    dst.extend_from_slice(&s[..take]);
}

/// `strstr(haystack, needle) != NULL` (an empty needle always matches).
pub fn strstr(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|w| w == needle)
}

/// `s[strcspn(s, "\n")] = 0` -- truncate the C string at the first newline.
pub fn truncate_at_newline(s: &[u8]) -> Vec<u8> {
    let s = up_to_nul(s);
    match s.iter().position(|&b| b == b'\n') {
        Some(i) => s[..i].to_vec(),
        None => s.to_vec(),
    }
}

/// `sscanf(s, "%d", &out)`: returns `Some(value)` when one item was assigned.
///
/// Mirrors glibc: the value is accumulated as a `long`, saturating at
/// `LONG_MAX`/`LONG_MIN` on overflow, and then truncated to `int`.
pub fn sscanf_int(s: &[u8]) -> Option<i32> {
    let s = up_to_nul(s);
    let mut i = 0usize;

    while i < s.len() && c_isspace(s[i]) {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let start = i;
    let mut acc: i64 = 0;
    let mut overflow = false;
    while i < s.len() && c_isdigit(s[i]) {
        let d = i64::from(s[i] - b'0');
        match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
            Some(v) => acc = v,
            None => overflow = true,
        }
        i += 1;
    }

    if i == start {
        // Matching failure (0) or input failure (EOF); neither assigns.
        return None;
    }

    let value: i64 = if overflow {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -acc
    } else {
        acc
    };

    // Assignment to an `int` truncates the low 32 bits.
    Some(value as i32)
}

// ---------------------------------------------------------------------------
// stdout / stderr
// ---------------------------------------------------------------------------

/// glibc's `BUFSIZ`.
const BUFSIZ: usize = 8192;

/// glibc's `_IO_file_doallocate`: the stream buffer is `st_blksize` of the
/// underlying fd when that is in `1..BUFSIZ`, and `BUFSIZ` otherwise (a pipe and
/// an ordinary file both report 4096 on Linux, a terminal 1024).
fn stdio_block_size(fd: i32) -> usize {
    use std::os::fd::FromRawFd;
    use std::os::unix::fs::MetadataExt;

    // fd 1 stays open: the File must not be dropped.
    let file = std::mem::ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(fd) });
    match file.metadata() {
        Ok(m) => {
            let b = m.blksize() as usize;
            if b > 0 && b < BUFSIZ {
                b
            } else {
                BUFSIZ
            }
        }
        Err(_) => BUFSIZ,
    }
}

/// Emulates a C `FILE *` for stdout: line buffered on a terminal, otherwise
/// block buffered with a buffer of `st_blksize` bytes.
pub struct Out {
    buf: Vec<u8>,
    cap: usize,
    line_buffered: bool,
}

impl Out {
    pub fn new() -> Out {
        let line_buffered = io::stdout().is_terminal();
        let cap = stdio_block_size(1);
        Out {
            buf: Vec::with_capacity(cap),
            cap,
            line_buffered,
        }
    }

    fn emit(chunk: &[u8]) {
        let stdout = io::stdout();
        let mut lock = stdout.lock();
        let _ = lock.write_all(chunk);
        let _ = lock.flush();
    }

    pub fn put(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
        if self.line_buffered {
            if let Some(pos) = self.buf.iter().rposition(|&b| b == b'\n') {
                let chunk: Vec<u8> = self.buf.drain(..=pos).collect();
                Out::emit(&chunk);
            }
        } else {
            while self.buf.len() >= self.cap {
                let chunk: Vec<u8> = self.buf.drain(..self.cap).collect();
                Out::emit(&chunk);
            }
        }
    }

    pub fn puts(&mut self, s: &str) {
        self.put(s.as_bytes());
    }

    pub fn flush_all(&mut self) {
        if !self.buf.is_empty() {
            let chunk: Vec<u8> = self.buf.drain(..).collect();
            Out::emit(&chunk);
        }
    }

    /// glibc's `_IO_flush_all_linebuffered`, which `_IO_new_file_underflow`
    /// calls before it reads from a line-buffered (i.e. terminal) input stream:
    /// only line-buffered output streams are flushed.
    pub fn flush_if_line_buffered(&mut self) {
        if self.line_buffered {
            self.flush_all();
        }
    }
}

impl Write for Out {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.put(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_all();
        Ok(())
    }
}

/// `fprintf(stderr, ...)` -- stderr is unbuffered in C.
pub fn err(data: &[u8]) {
    let stderr = io::stderr();
    let mut lock = stderr.lock();
    let _ = lock.write_all(data);
    let _ = lock.flush();
}

// ---------------------------------------------------------------------------
// stdin
// ---------------------------------------------------------------------------

/// Emulates `fgets` on stdin, including its buffer-size truncation behaviour.
pub struct In {
    inner: io::Stdin,
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
    cap: usize,
    /// A terminal makes the C `stdin` stream line buffered, and reading from a
    /// line-buffered stream flushes every line-buffered output stream first.
    line_buffered: bool,
}

impl In {
    pub fn new() -> In {
        In {
            inner: io::stdin(),
            buf: Vec::new(),
            pos: 0,
            eof: false,
            cap: stdio_block_size(0),
            line_buffered: io::stdin().is_terminal(),
        }
    }

    fn fill(&mut self, out: &mut Out) -> bool {
        if self.pos < self.buf.len() {
            return true;
        }
        if self.eof {
            return false;
        }
        // _IO_new_file_underflow: flush the line-buffered output streams before
        // an actual read on a line-buffered input stream.
        if self.line_buffered {
            out.flush_if_line_buffered();
        }
        let mut chunk = vec![0u8; self.cap];
        loop {
            match self.inner.read(&mut chunk) {
                Ok(0) => {
                    self.eof = true;
                    return false;
                }
                Ok(n) => {
                    self.buf.clear();
                    self.buf.extend_from_slice(&chunk[..n]);
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

    fn next_byte(&mut self, out: &mut Out) -> Option<u8> {
        if !self.fill(out) {
            return None;
        }
        let b = self.buf[self.pos];
        self.pos += 1;
        Some(b)
    }

    /// `fgets(buf, size, stdin)`: reads at most `size - 1` bytes, stopping
    /// after a newline. Returns `None` on immediate EOF (C returns NULL).
    /// The returned vector is the C string contents (no NUL terminator).
    ///
    /// `out` is the emulated `stdout` stream, which C flushes when a read has to
    /// refill a line-buffered `stdin`.
    pub fn fgets(&mut self, size: usize, out: &mut Out) -> Option<Vec<u8>> {
        if size <= 1 {
            return None;
        }
        let mut line: Vec<u8> = Vec::new();
        while line.len() < size - 1 {
            match self.next_byte(out) {
                Some(b) => {
                    line.push(b);
                    if b == b'\n' {
                        break;
                    }
                }
                None => break,
            }
        }
        if line.is_empty() {
            None
        } else {
            Some(line)
        }
    }
}
