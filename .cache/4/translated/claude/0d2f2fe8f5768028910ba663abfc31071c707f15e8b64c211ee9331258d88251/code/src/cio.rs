//! Minimal emulation of the C stdio behaviour that the original program
//! relies on: a fully buffered `stdout` (as glibc uses when stdout is not a
//! tty), an unbuffered `stderr`, `fgets` and `sscanf("%d")`.

use std::fs::File;
use std::io::{BufRead, BufReader, Stdin, Write};
use std::os::fd::AsFd;

/// Size of the buffer glibc picks for a fully buffered stream (st_blksize).
const STDIO_BUFSIZE: usize = 4096;

/// Fully buffered stdout emulation.
///
/// `std::io::Stdout` adds its own line buffering, which would split the
/// emulated 4096 byte flushes at newlines; writing through a duplicate of file
/// descriptor 1 keeps the write pattern (and therefore the interleaving with
/// stderr) identical to glibc's.
pub struct Out {
    buf: Vec<u8>,
    fd: Option<File>,
}

impl Out {
    pub fn new() -> Out {
        let fd = std::io::stdout()
            .as_fd()
            .try_clone_to_owned()
            .ok()
            .map(File::from);
        Out {
            buf: Vec::with_capacity(STDIO_BUFSIZE * 2),
            fd,
        }
    }

    fn raw_write(&mut self, data: &[u8]) {
        // Ignore write errors, just like printf's return value is ignored.
        match self.fd.as_mut() {
            Some(file) => {
                let _ = file.write_all(data);
            }
            None => {
                let stdout = std::io::stdout();
                let mut lock = stdout.lock();
                let _ = lock.write_all(data);
                let _ = lock.flush();
            }
        }
    }

    /// Equivalent of writing `data` through a fully buffered FILE*.
    pub fn write(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
        while self.buf.len() >= STDIO_BUFSIZE {
            let mut chunk = self.buf.split_off(STDIO_BUFSIZE);
            std::mem::swap(&mut chunk, &mut self.buf);
            // `chunk` now holds the first STDIO_BUFSIZE bytes.
            self.raw_write(&chunk);
        }
    }

    /// Equivalent of writing a plain string literal.
    pub fn s(&mut self, data: &str) {
        self.write(data.as_bytes());
    }

    pub fn flush(&mut self) {
        if !self.buf.is_empty() {
            let pending = std::mem::take(&mut self.buf);
            self.raw_write(&pending);
        }
    }
}

/// The C program dies from a fatal signal here.  Neither a fatal signal nor
/// `abort()` flushes glibc's stdio buffers, so whatever is still buffered for
/// stdout is lost - which is why nothing is flushed before terminating.
pub fn die_signal() -> ! {
    // The original overruns a fixed size array on the stack, which takes the
    // process down with SIGSEGV (occasionally SIGBUS, depending on the layout
    // ASLR happens to pick).
    unsafe {
        std::ptr::write_volatile(std::ptr::null_mut::<u8>(), 0u8);
    }
    std::process::abort()
}

/// `malloc_printerr()` + `abort()` as performed by glibc's heap checks.
pub fn die_abort() -> ! {
    std::process::abort()
}

/// Unbuffered stderr emulation (`fprintf(stderr, ...)`).
pub fn err(data: &[u8]) {
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    let _ = lock.write_all(data);
    let _ = lock.flush();
}

#[allow(dead_code)]
pub fn err_s(data: &str) {
    err(data.as_bytes());
}

/// Buffered stdin used to emulate `fgets`.
pub struct In {
    r: BufReader<Stdin>,
}

impl In {
    pub fn new() -> In {
        In {
            r: BufReader::new(std::io::stdin()),
        }
    }

    fn read_byte(&mut self) -> Option<u8> {
        let b = match self.r.fill_buf() {
            Ok(buf) => {
                if buf.is_empty() {
                    return None;
                }
                buf[0]
            }
            Err(_) => return None,
        };
        self.r.consume(1);
        Some(b)
    }

    /// `fgets(buf, size, stdin)`: reads at most `size - 1` bytes, stops after a
    /// newline (which is kept).  Returns `None` when EOF is hit before any
    /// character was read (i.e. when C's `fgets` returns NULL).
    pub fn fgets(&mut self, size: usize) -> Option<Vec<u8>> {
        let mut out: Vec<u8> = Vec::new();
        while out.len() + 1 < size {
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

/// The portion of a `char` buffer that forms the C string (up to the first NUL).
pub fn cstr(buf: &[u8]) -> &[u8] {
    match buf.iter().position(|&b| b == 0) {
        Some(p) => &buf[..p],
        None => buf,
    }
}

/// `s[strcspn(s, "\n")] = 0;` applied to a C string.
pub fn strip_newline(buf: &[u8]) -> &[u8] {
    let s = cstr(buf);
    match s.iter().position(|&b| b == b'\n') {
        Some(p) => &s[..p],
        None => s,
    }
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// `sscanf(s, "%d", &out)`: returns `Some(value)` when the conversion
/// succeeded (return value 1), `None` otherwise.
pub fn sscanf_d(s: &[u8]) -> Option<i32> {
    let s = cstr(s);
    let mut i = 0usize;
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }
    let start = i;
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
    if i == start {
        return None;
    }
    // glibc converts through `long int` and truncates on assignment; on
    // overflow the clamped LONG_MAX / LONG_MIN value is stored.
    let long_value = if overflow {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        value.wrapping_neg()
    } else {
        value
    };
    Some(long_value as i32)
}
