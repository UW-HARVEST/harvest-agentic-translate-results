// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Rust translation of `c_src/src/main.c`.
//!
//! The original is a CWE-457 (use of uninitialized variable) test case. The
//! translation intentionally preserves the original behavior, including the
//! defect, rather than fixing it. See `bad()` for details.

use std::io::Write;

/// Translation of:
/// ```c
/// void printLine(const char *line)
/// {
///     if (line != NULL) { printf("%s\n", line); }
/// }
/// ```
///
/// `Option<&str>` models `const char *`: `None` is the NULL pointer, `Some`
/// is a valid pointer to a NUL-terminated string. `printf("%s\n", ...)` emits
/// the string bytes followed by a single `\n`.
fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        // printf("%s\n", line)
        let _ = out.write_all(line.as_bytes());
        let _ = out.write_all(b"\n");
    }
}

/// Translation of:
/// ```c
/// void bad()
/// {
///     char *data;          /* never initialized -- CWE-457 */
///     printLine(data);
/// }
/// ```
///
/// Reading `data` is undefined behavior in C, so there is no portable "correct"
/// output. The reference build (`c_src/CMakeLists.txt`, i.e. cc with no
/// optimization flags, glibc/x86-64) leaves the stack slot holding residue from
/// the preceding `scanf` call: a non-NULL pointer whose first byte is `0`.
/// `printLine` therefore takes the non-NULL branch and prints an empty string
/// followed by a newline, i.e. a single `\n`.
///
/// Reproducing that observed behavior in safe Rust means passing `Some("")`;
/// reading an actually-uninitialized value here would be UB in Rust as well and
/// is deliberately avoided. The defect is preserved, not repaired: `bad()`
/// still emits a bare newline instead of any meaningful string.
fn bad() {
    // Uninitialized `char *data` as observed in the reference build:
    // non-NULL, pointing at a zero byte.
    let data: Option<&str> = Some("");
    print_line(data);
}

/// Translation of:
/// ```c
/// void good()
/// {
///     char *data;
///     data = "string";
///     printLine(data);
/// }
/// ```
fn good() {
    let data: Option<&str> = Some("string");
    print_line(data);
}

// Raw libc entry points. These are used instead of `std::io::Stdin` so that the
// observable *stream* behavior matches glibc's `FILE *` exactly: see `CStdin`.
// libc is already linked on the `*-unknown-linux-gnu` targets, so no extra
// dependency is required.
extern "C" {
    fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
    fn lseek(fd: i32, offset: i64, whence: i32) -> i64;
    fn signal(signum: i32, handler: usize) -> usize;
}

const STDIN_FILENO: i32 = 0;
const SEEK_CUR: i32 = 1;
const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;

/// Restore the default disposition of `SIGPIPE`.
///
/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main`, so a write to a
/// closed pipe returns `EPIPE` and the process survives. A C program keeps the
/// default disposition and is *killed by signal 13*. Without this the exit
/// status diverges (C: killed by SIGPIPE, i.e. `141` as reported by a shell;
/// Rust: `0`) whenever stdout is a pipe whose read end has been closed.
fn reset_sigpipe() {
    // SAFETY: `signal` with `SIG_DFL` is async-signal-safe and is called before
    // any output is produced.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

/// glibc buffer size for stdin.
///
/// glibc sizes a `FILE` buffer from `st_blksize` of the underlying fd, which is
/// 4096 for both regular files and pipes on Linux. The value is observable: it
/// decides how many bytes a single `scanf` call removes from a *pipe* and thus
/// what a later reader of the same descriptor sees.
const BUFSIZ: usize = 4096;

/// Byte-at-a-time reader over stdin that reproduces glibc's `FILE *` semantics
/// for `scanf`, including the two effects that are visible outside the process:
///
/// 1. **Block reads.** Bytes are pulled from fd 0 in `BUFSIZ` chunks, not one
///    at a time, so a `scanf` on a pipe consumes up to `BUFSIZ` bytes even when
///    it only converts a few of them.
/// 2. **Offset restoration.** When the descriptor is seekable, glibc rewinds
///    the file offset by the unconsumed remainder of the buffer while the
///    stream is being cleaned up at exit, leaving the offset exactly past the
///    last byte the conversion consumed. A later reader of the same descriptor
///    (`{ ./driver; cat; } < file`) therefore sees the rest of the file.
///
/// `ungetc` is modelled by stepping the read pointer back one byte, which is
/// what glibc does when the pushed-back byte is the one just read out of the
/// buffer -- the only case `%d` can produce.
struct CStdin {
    buf: [u8; BUFSIZ],
    /// Number of valid bytes in `buf`.
    filled: usize,
    /// Read cursor into `buf`; `filled - pos` bytes are buffered but unconsumed.
    pos: usize,
    /// Sticky end-of-file / error flag, mirroring the stream's `_IO_EOF_SEEN`.
    eof: bool,
}

impl CStdin {
    fn new() -> Self {
        CStdin {
            buf: [0u8; BUFSIZ],
            filled: 0,
            pos: 0,
            eof: false,
        }
    }

    /// `getc()`: returns `None` on EOF (or read error, which `scanf` also
    /// treats as an input failure).
    fn getc(&mut self) -> Option<u8> {
        if self.pos == self.filled {
            if self.eof {
                return None;
            }
            // SAFETY: `buf` is a valid, writable, `BUFSIZ`-byte allocation.
            let n = unsafe { read(STDIN_FILENO, self.buf.as_mut_ptr(), BUFSIZ) };
            if n <= 0 {
                // 0 == EOF; < 0 == read error, which `scanf` also reports as an
                // input failure. glibc does not retry, so neither do we.
                self.eof = true;
                self.filled = 0;
                self.pos = 0;
                return None;
            }
            self.filled = n as usize;
            self.pos = 0;
        }
        let b = self.buf[self.pos];
        self.pos += 1;
        Some(b)
    }

    /// `ungetc()`: push the just-read byte back onto the stream.
    fn ungetc(&mut self, _b: u8) {
        debug_assert!(self.pos > 0);
        self.pos -= 1;
    }

    /// Stream cleanup at exit: hand back the bytes that were buffered but never
    /// consumed, so the shared file offset ends up where glibc leaves it. Fails
    /// harmlessly with `ESPIPE` on pipes and other non-seekable descriptors,
    /// which is also what glibc ends up doing (i.e. nothing).
    fn restore_offset(&self) {
        let unconsumed = self.filled - self.pos;
        if unconsumed == 0 {
            return;
        }
        // SAFETY: plain syscall wrapper; a failure is ignored, as in glibc.
        unsafe {
            lseek(STDIN_FILENO, -(unconsumed as i64), SEEK_CUR);
        }
    }
}

/// C `isspace()` for the C locale: space, \t, \n, \v, \f, \r.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Translation of `scanf("%d", &x)`.
///
/// Matches glibc's behavior:
/// * leading whitespace is skipped, *including newlines* (so the conversion
///   reads across line boundaries until it finds a non-space byte),
/// * an optional `+`/`-` sign is accepted, followed by one or more decimal
///   digits,
/// * the first non-digit byte terminates the conversion and is pushed back,
/// * accumulation is done at `long` width and saturates at `LONG_MAX` /
///   `LONG_MIN` on overflow, then the result is truncated to `int` when stored
///   (verified against the reference build: `18446744073709551617` stores `-1`,
///   `-9223372036854775809` stores `0`, `4294967296` stores `0`),
/// * on a matching failure or EOF before any digit, `*x` is left untouched.
///
/// Returns the `scanf` return value (`1`, `0`, or `EOF` = `-1`). The original
/// program ignores it; it is returned for fidelity.
fn scanf_d(input: &mut CStdin, x: &mut i32) -> i32 {
    // Skip leading whitespace; EOF here is an input failure -> EOF return.
    let mut b = loop {
        match input.getc() {
            None => return -1,
            Some(c) if is_c_space(c) => continue,
            Some(c) => break c,
        }
    };

    let mut negative = false;
    if b == b'+' || b == b'-' {
        negative = b == b'-';
        match input.getc() {
            None => {
                // Sign then EOF: nothing was converted.
                return -1;
            }
            Some(c) => b = c,
        }
    }

    if !b.is_ascii_digit() {
        // Matching failure: the offending byte is pushed back, *x unchanged.
        input.ungetc(b);
        return 0;
    }

    // Accumulate at `long` (i64) width with strtol-style saturation.
    let mut acc: i64 = 0;
    let mut saturated = false;
    loop {
        let digit = (b - b'0') as i64;
        if !saturated {
            match acc
                .checked_mul(10)
                .and_then(|v| if negative { v.checked_sub(digit) } else { v.checked_add(digit) })
            {
                Some(v) => acc = v,
                None => {
                    saturated = true;
                    acc = if negative { i64::MIN } else { i64::MAX };
                }
            }
        }

        match input.getc() {
            None => break,
            Some(c) if c.is_ascii_digit() => b = c,
            Some(c) => {
                input.ungetc(c);
                break;
            }
        }
    }

    // Stored through an `int *`: truncate to 32 bits.
    *x = acc as i32;
    1
}

/// Translation of:
/// ```c
/// int main()
/// {
///     int x = 0;
///     scanf("%d", &x);
///     if (x) { good(); } else { bad(); }
///     return 0;
/// }
/// ```
fn main() {
    // The Rust runtime ignores SIGPIPE before `main`; C does not.
    reset_sigpipe();

    let mut x: i32 = 0;
    let mut input = CStdin::new();
    let _ = scanf_d(&mut input, &mut x);

    if x != 0 {
        good();
    } else {
        bad();
    }

    // C's exit from main tears down the streams: stdout is flushed and stdin's
    // unconsumed buffer is given back to the shared file offset.
    let _ = std::io::stdout().flush();
    input.restore_offset();
    std::process::exit(0);
}
