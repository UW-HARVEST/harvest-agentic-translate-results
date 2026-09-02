//! Faithful re-implementations of the C standard library behaviours that the
//! original program relies on: `fgets`, `sscanf("%d", ...)`, `strcspn`, and
//! C-string (NUL terminated) semantics for `%s` conversions.

use std::io::{BufRead, BufReader, IsTerminal, Stdin, Write};

/// Returns the bytes of `buf` that make up the C string it holds, i.e.
/// everything before the first NUL byte.  `printf("%s")`, `strcmp`, `strcspn`
/// and friends all stop at the terminator, so any byte after an embedded NUL is
/// invisible to the original program.
pub fn cstr(buf: &[u8]) -> &[u8] {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    &buf[..end]
}

/// `input[strcspn(input, "\n")] = 0;`
///
/// Truncates the C string at the first newline.  If the buffer contains no
/// newline (a line longer than the buffer, or a final line without a trailing
/// newline at EOF) the string is left untouched, exactly as in C.
pub fn strip_newline(buf: &[u8]) -> &[u8] {
    let s = cstr(buf);
    let end = s.iter().position(|&c| c == b'\n').unwrap_or(s.len());
    &s[..end]
}

/// C `isspace` for the "C" locale: space, \t, \n, \v, \f, \r.
fn c_isspace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// `sscanf(s, "%d", &out)`.
///
/// Returns `Some(value)` when the conversion succeeds (assignment count 1) and
/// `None` on a matching failure or on an empty input (assignment count 0 or
/// EOF).  Out-of-range values reproduce glibc's behaviour: the accumulated
/// value saturates at `long` range and is then truncated on assignment to
/// `int`.
pub fn sscanf_int(buf: &[u8]) -> Option<i32> {
    let s = cstr(buf);
    let mut i = 0usize;

    // %d skips leading whitespace.
    while i < s.len() && c_isspace(s[i]) {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    if i == digits_start {
        // No digits consumed: matching failure (or EOF on empty input).
        return None;
    }

    let mut acc: i64 = 0;
    let mut overflow = false;
    for &c in &s[digits_start..i] {
        let d = i64::from(c - b'0');
        match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
            Some(v) => acc = v,
            None => {
                overflow = true;
                break;
            }
        }
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

    Some(value as i32)
}

/// A byte oriented stdin wrapper providing C `fgets` semantics.
pub struct CStdin {
    reader: BufReader<Stdin>,
}

impl CStdin {
    pub fn new() -> Self {
        CStdin {
            reader: BufReader::new(std::io::stdin()),
        }
    }

    fn read_byte(&mut self) -> Option<u8> {
        loop {
            let available = match self.reader.fill_buf() {
                Ok(b) => b,
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            };
            if available.is_empty() {
                return None;
            }
            let b = available[0];
            self.reader.consume(1);
            return Some(b);
        }
    }

    /// `fgets(buf, size, stdin)`.
    ///
    /// Reads at most `size - 1` bytes, stopping after a newline (which is kept)
    /// or at EOF.  Returns `None` (NULL) only when nothing at all could be read.
    /// A line longer than `size - 1` bytes is split across successive calls,
    /// just like the C original.
    pub fn fgets(&mut self, size: usize) -> Option<Vec<u8>> {
        let max = size - 1;
        let mut out: Vec<u8> = Vec::new();
        while out.len() < max {
            match self.read_byte() {
                Some(b) => {
                    out.push(b);
                    if b == b'\n' {
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
}

/// stdout with glibc's buffering discipline: block buffered (4096 bytes, the
/// usual pipe block size) when redirected, line buffered when attached to a
/// terminal.
pub struct Out {
    buf: Vec<u8>,
    cap: usize,
    line_buffered: bool,
}

impl Out {
    pub fn new() -> Self {
        Out {
            buf: Vec::with_capacity(4096),
            cap: 4096,
            line_buffered: std::io::stdout().is_terminal(),
        }
    }

    pub fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.buf.push(b);
            if self.buf.len() >= self.cap || (self.line_buffered && b == b'\n') {
                self.flush();
            }
        }
    }

    pub fn flush(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        let _ = lock.write_all(&self.buf);
        let _ = lock.flush();
        self.buf.clear();
    }
}

impl Drop for Out {
    fn drop(&mut self) {
        self.flush();
    }
}

/// `fprintf(stderr, ...)`: stderr is unbuffered in C.
pub fn err(bytes: &[u8]) {
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    let _ = lock.write_all(bytes);
    let _ = lock.flush();
}
