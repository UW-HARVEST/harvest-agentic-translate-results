//! Small helpers that reproduce the C standard library I/O behaviour that the
//! original program relies on:
//!
//!   * `fgets()` semantics (stop after a newline, keep the newline, never read
//!     more than `size - 1` bytes, return NULL only when nothing could be read)
//!   * `sscanf(buf, "%d", &x)` semantics (skip whitespace, optional sign,
//!     glibc's saturate-to-`long`-then-truncate-to-`int` overflow behaviour)
//!   * `strcspn(s, "\n")` / NUL-terminated string handling on raw bytes
//!   * stdout being fully buffered (as it is when redirected to a file/pipe)
//!     while stderr is unbuffered.

use std::fs::File;
use std::io::{self, BufReader, Read, Stdin, Write};
use std::mem::ManuallyDrop;
use std::os::fd::FromRawFd;

/// Buffered reader over stdin used to emulate `fgets`.
pub struct CStdin {
    reader: BufReader<Stdin>,
}

impl CStdin {
    pub fn new() -> CStdin {
        CStdin {
            reader: BufReader::new(io::stdin()),
        }
    }

    /// `fgets(buf, size, stdin)`.
    ///
    /// Returns `None` when `fgets` would return `NULL` (end of file / error
    /// before any character was stored).  The returned bytes include the
    /// trailing `'\n'` when one was read, exactly like the C buffer contents
    /// (minus the terminating NUL which is implicit here).
    pub fn fgets(&mut self, size: usize) -> Option<Vec<u8>> {
        if size == 0 {
            return None;
        }
        let max = size - 1;
        let mut out: Vec<u8> = Vec::new();
        let mut byte = [0u8; 1];
        while out.len() < max {
            match self.reader.read(&mut byte) {
                Ok(0) => break,
                Ok(_) => {
                    out.push(byte[0]);
                    if byte[0] == b'\n' {
                        break;
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }
}

/// Size of glibc's stdout buffer: the block size of the stream, which is
/// 4096 for pipes and for the usual regular files.
const STDOUT_BUF_SIZE: usize = 4096;

/// Fully buffered stdout, mirroring glibc's behaviour for a non-tty stdout:
/// the data is accumulated until the buffer is *exactly* full, then the whole
/// buffer is handed to a single `write()`; anything still buffered is written
/// when the stream is flushed at exit.  Reproducing this gives the same
/// write boundaries as the C program, so even `2>&1` interleaving with the
/// unbuffered stderr comes out identical.
pub struct COut {
    /// Raw (unbuffered) handle for fd 1 - `std::io::Stdout` is itself line
    /// buffered, which would change the write boundaries.
    fd: ManuallyDrop<File>,
    buf: Vec<u8>,
}

impl COut {
    pub fn new() -> COut {
        // SAFETY: fd 1 is open for the whole lifetime of the process and the
        // `File` is wrapped in `ManuallyDrop` so it is never closed by us.
        let fd = unsafe { File::from_raw_fd(1) };
        COut {
            fd: ManuallyDrop::new(fd),
            buf: Vec::with_capacity(STDOUT_BUF_SIZE),
        }
    }

    /// `printf`-style raw byte output.  Write errors are ignored, just as the
    /// C code ignores the return value of `printf`.
    pub fn put(&mut self, bytes: &[u8]) {
        let mut rest = bytes;
        while !rest.is_empty() {
            let space = STDOUT_BUF_SIZE - self.buf.len();
            let n = if rest.len() < space { rest.len() } else { space };
            self.buf.extend_from_slice(&rest[..n]);
            rest = &rest[n..];
            if self.buf.len() == STDOUT_BUF_SIZE {
                self.flush();
            }
        }
    }

    pub fn flush(&mut self) {
        if !self.buf.is_empty() {
            let _ = self.fd.write_all(&self.buf);
            self.buf.clear();
        }
    }
}

/// `fprintf(stderr, ...)`: unbuffered raw byte output.
pub fn eput(bytes: &[u8]) {
    let mut err = io::stderr();
    let _ = err.write_all(bytes);
    let _ = err.flush();
}

/// The C string stored in a byte buffer: everything up to the first NUL.
pub fn cstr(bytes: &[u8]) -> &[u8] {
    match bytes.iter().position(|&b| b == 0) {
        Some(i) => &bytes[..i],
        None => bytes,
    }
}

/// `buf[strcspn(buf, "\n")] = 0;` applied to a freshly `fgets`-ed buffer.
pub fn chomp(bytes: &[u8]) -> Vec<u8> {
    let s = cstr(bytes);
    match s.iter().position(|&b| b == b'\n') {
        Some(i) => s[..i].to_vec(),
        None => s.to_vec(),
    }
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// `sscanf(s, "%d", &out)`: returns `Some(value)` when the conversion
/// succeeded (return value 1), `None` otherwise (0 or EOF).
///
/// glibc converts the digit run with `strtol`, which saturates at
/// `LONG_MAX` / `LONG_MIN`, and then stores the result into an `int`,
/// truncating the value.  That behaviour is reproduced here.
pub fn sscanf_int(s: &[u8]) -> Option<i32> {
    let mut i = 0usize;
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }
    let digits_start = i;
    let mut magnitude: u128 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = (s[i] - b'0') as u128;
        if !overflow {
            match magnitude.checked_mul(10).and_then(|m| m.checked_add(d)) {
                Some(m) => magnitude = m,
                None => overflow = true,
            }
        }
        i += 1;
    }
    if i == digits_start {
        // Matching failure (or input exhausted): not 1 conversion.
        return None;
    }

    const LONG_MAX: u128 = i64::MAX as u128;
    let as_long: i64 = if negative {
        if overflow || magnitude > LONG_MAX + 1 {
            i64::MIN
        } else {
            (-(magnitude as i128)) as i64
        }
    } else if overflow || magnitude > LONG_MAX {
        i64::MAX
    } else {
        magnitude as i64
    };

    Some(as_long as i32)
}
