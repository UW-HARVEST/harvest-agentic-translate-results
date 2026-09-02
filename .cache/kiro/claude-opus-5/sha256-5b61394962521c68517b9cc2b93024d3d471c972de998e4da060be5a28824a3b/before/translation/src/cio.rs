//! C stdio emulation helpers: buffered stdout (like glibc's fully-buffered
//! stdout when redirected), unbuffered stderr, and `fgets` semantics on stdin.

use std::cell::RefCell;
use std::io::{BufRead, BufReader, Stdin, Write};

/// glibc's default stdio buffer size for a pipe/file is st_blksize (4096).
const STDOUT_BUF_SIZE: usize = 4096;

thread_local! {
    static OUT: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(STDOUT_BUF_SIZE));
}

fn drain(buf: &mut Vec<u8>) {
    if buf.is_empty() {
        return;
    }
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(buf);
    let _ = lock.flush();
    buf.clear();
}

/// Write raw bytes to the emulated stdout stream.
pub fn out_bytes(data: &[u8]) {
    OUT.with(|cell| {
        let mut buf = cell.borrow_mut();
        let mut rest = data;
        while !rest.is_empty() {
            let space = STDOUT_BUF_SIZE - buf.len();
            if space == 0 {
                drain(&mut buf);
                continue;
            }
            let n = if space < rest.len() { space } else { rest.len() };
            buf.extend_from_slice(&rest[..n]);
            rest = &rest[n..];
        }
    });
}

/// Flush the emulated stdout stream (called at process exit).
pub fn out_flush() {
    OUT.with(|cell| {
        let mut buf = cell.borrow_mut();
        drain(&mut buf);
    });
}

/// Write raw bytes to stderr (unbuffered, like C's stderr).
pub fn err_bytes(data: &[u8]) {
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    let _ = lock.write_all(data);
    let _ = lock.flush();
}

/// `printf`-alike for UTF-8 formatted text.
#[macro_export]
macro_rules! cprintf {
    ($($arg:tt)*) => {
        $crate::cio::out_bytes(format!($($arg)*).as_bytes())
    };
}

/// `fprintf(stderr, ...)`-alike for UTF-8 formatted text.
#[macro_export]
macro_rules! ceprintf {
    ($($arg:tt)*) => {
        $crate::cio::err_bytes(format!($($arg)*).as_bytes())
    };
}

/// Wrapper around stdin providing C `fgets` semantics.
pub struct CStdin {
    reader: BufReader<Stdin>,
}

impl CStdin {
    pub fn new() -> Self {
        CStdin {
            reader: BufReader::new(std::io::stdin()),
        }
    }

    /// `fgets(buf, size, stdin)`: reads at most `size - 1` bytes, stopping
    /// after a newline (which is retained). Returns `None` on EOF with no
    /// bytes read (i.e. when C's fgets returns NULL).
    pub fn fgets(&mut self, size: usize) -> Option<Vec<u8>> {
        if size <= 1 {
            return None;
        }
        let max = size - 1;
        let mut out: Vec<u8> = Vec::new();

        while out.len() < max {
            let (chunk, found_newline) = {
                let buf = match self.reader.fill_buf() {
                    Ok(b) => b,
                    Err(_) => break,
                };
                if buf.is_empty() {
                    break;
                }
                let remaining = max - out.len();
                let avail = if buf.len() < remaining {
                    buf.len()
                } else {
                    remaining
                };
                match buf[..avail].iter().position(|&b| b == b'\n') {
                    Some(pos) => (buf[..=pos].to_vec(), true),
                    None => (buf[..avail].to_vec(), false),
                }
            };
            self.reader.consume(chunk.len());
            out.extend_from_slice(&chunk);
            if found_newline {
                return Some(out);
            }
        }

        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }
}

/// The C-string view of a buffer: everything up to the first NUL byte.
pub fn cstr(bytes: &[u8]) -> &[u8] {
    match bytes.iter().position(|&b| b == 0) {
        Some(pos) => &bytes[..pos],
        None => bytes,
    }
}

/// `strcspn(s, "\n")` followed by `s[n] = 0`: truncate at the first newline.
pub fn truncate_at_newline(bytes: &[u8]) -> &[u8] {
    match bytes.iter().position(|&b| b == b'\n') {
        Some(pos) => &bytes[..pos],
        None => bytes,
    }
}

/// `strstr(haystack, needle) != NULL`
pub fn strstr(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// `sscanf(s, "%d", &out)`: returns the parsed value if the conversion
/// succeeded (i.e. sscanf would return 1), otherwise `None`.
pub fn sscanf_int(s: &[u8]) -> Option<i32> {
    let mut i = 0usize;

    // %d skips leading whitespace.
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    let mut value: i64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = (s[i] - b'0') as i64;
        if !overflow {
            match value.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => value = v,
                None => overflow = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // Matching failure (or input failure): sscanf returns 0 or EOF.
        return None;
    }

    // glibc converts via strtol semantics, saturating at LONG_MIN/LONG_MAX,
    // then stores the (truncated) value into the int object.
    let wide = if overflow {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -value
    } else {
        value
    };

    Some(wide as i32)
}
