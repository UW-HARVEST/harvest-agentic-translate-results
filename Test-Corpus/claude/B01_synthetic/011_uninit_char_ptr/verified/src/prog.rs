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
//! This module is `#[path]`-included by BOTH `src/main.rs` (the `driver`
//! executable, the artifact the C `CMakeLists.txt` builds) and `src/lib.rs`
//! (the cdylib whose `#[no_mangle]` wrappers mirror the symbols a shared
//! library build of `main.c` exports).  It is deliberately *not* an `rlib`
//! dependency of those two targets, see Cargo.toml for the reason.
//!
//! The C program is:
//!
//! ```c
//! void printLine(const char *line) { if (line != NULL) printf("%s\n", line); }
//! void bad()  { char *data;                 printLine(data); }
//! void good() { char *data; data = "string"; printLine(data); }
//! int  main() { int x = 0; scanf("%d", &x); if (x) good(); else bad(); return 0; }
//! ```
//!
//! Nothing here "fixes" the C.  In particular [`CStdin`] reproduces the parts of
//! glibc's `stdin` that a caller can actually observe, and [`scanf_int`]
//! reproduces `%d`'s `long` accumulation with clamping, its truncation to `int`,
//! and its one-character push-back - all measured against the C, see the
//! comments on each item.

use std::io::{Read, Seek, SeekFrom, Write};
use std::mem::ManuallyDrop;
use std::os::unix::fs::MetadataExt;
use std::os::unix::io::FromRawFd;

/// The bytes `bad()` ends up handing to `printLine()`.
///
/// `bad()` reads the *uninitialized* automatic `char *data`, which is undefined
/// behaviour in C: the value is whatever the caller happened to leave in that
/// stack slot, so it is a property of the emitted binary and not of the source.
/// Measured outcomes for this one `main.c` range from `"\n"` through a run of the
/// library's own machine code to `SIGSEGV` (see ERRORS.md row 22).
///
/// Every path the *program* takes agrees, though: reached through `main`, in the
/// executable built by `c_src/CMakeLists.txt` **and** in a shared-library build,
/// the slot holds a non-NULL pointer to a zero byte, `printLine()` passes its
/// `!= NULL` test, and `printf("%s\n", line)` emits a single `'\n'`.  That was
/// verified for every input tried, including 1 MB ones and thousands of
/// randomized fuzz cases, so the empty string below reproduces it byte-for-byte.
pub const BAD_DATA: &[u8] = b"";

/// `good()`'s `data = "string";`
pub const GOOD_DATA: &[u8] = b"string";

/// Mirrors `void printLine(const char *line)`.
///
/// `None` models a NULL pointer (printing nothing), `Some(bytes)` a valid C
/// string, given as raw bytes because `printf("%s\n", ...)` copies the bytes up
/// to the terminating NUL verbatim without caring about encoding.
pub fn print_line<W: Write>(line: Option<&[u8]>, out: &mut W) {
    if let Some(line) = line {
        // printf("%s\n", line);  (gcc lowers this to puts(line))
        // Return values are dropped exactly like the C code drops printf's.
        let _ = out.write_all(line);
        let _ = out.write_all(b"\n");
    }
}

/// Mirrors `void bad()`.
pub fn bad<W: Write>(out: &mut W) {
    let data: Option<&[u8]> = Some(BAD_DATA);
    print_line(data, out);
}

/// Mirrors `void good()`.
pub fn good<W: Write>(out: &mut W) {
    let data: Option<&[u8]> = Some(GOOD_DATA);
    print_line(data, out);
}

// ---------------------------------------------------------------------------
// The input stream
// ---------------------------------------------------------------------------

/// A byte source with one character of push-back - i.e. what `getc`/`ungetc`
/// give a `scanf` conversion.
pub trait ByteSource {
    /// Next byte, or `None` at EOF / on a read error.
    fn next_byte(&mut self) -> Option<u8>;
    /// Returns one byte to the stream. At most one is ever outstanding.
    fn push_back(&mut self, b: u8);
}

/// glibc's fallback buffer size when `st_blksize` is unusable.
const BUFSIZ: usize = 8192;

/// Models the parts of glibc's `stdin` `FILE` that a caller can observe.
///
/// Three behaviours here were measured against the C rather than assumed, and
/// each one is observable:
///
/// 1. **Refill granularity.** glibc allocates the buffer with `st_blksize`
///    (4096 for both a pipe and a regular file on this system, `BUFSIZ`
///    otherwise) and refills it with a single `read(2)` of that size. A process
///    that shares fd 0 sees exactly how much was consumed: after reading `1`
///    from a 100 002-byte **pipe** the C leaves 95 906 bytes, i.e. it swallowed
///    4096. A naive translation using Rust's `std::io::stdin()` swallows 8192
///    and leaves 91 810.
/// 2. **Repositioning at exit.** For a *seekable* fd, libc's exit-time stream
///    cleanup seeks back to the logical position, so the buffered-but-unread
///    bytes reappear: `{ ./driver; cat; } < file` containing `"1 hello world"`
///    prints `" hello world"` for the C. The fd offset ends at 1, not 13.
/// 3. **One character of push-back**, see [`scanf_int`].
///
/// Reads are done through `File`, whose `read` is a bare `read(2)` with no
/// buffering of its own, so the syscall pattern is exactly the modelled one.
pub struct CStdin {
    /// fd 0. `ManuallyDrop` because closing the process' stdin would be wrong.
    file: ManuallyDrop<std::fs::File>,
    buf: Vec<u8>,
    /// Next unconsumed byte in `buf`.
    pos: usize,
    /// Number of valid bytes in `buf`.
    filled: usize,
    /// The `ungetc` slot.
    pending: Option<u8>,
}

impl CStdin {
    /// Opens fd 0 with a glibc-sized buffer.
    pub fn new() -> Self {
        // SAFETY: fd 0 is owned by the process; `ManuallyDrop` keeps this
        // wrapper from closing it.
        let file = ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(0) });
        let size = match file.metadata() {
            Ok(m) => {
                let b = m.blksize() as usize;
                if b > 0 {
                    b
                } else {
                    BUFSIZ
                }
            }
            // fstat failing (e.g. fd 0 closed) leaves glibc with BUFSIZ.
            Err(_) => BUFSIZ,
        };
        CStdin {
            file,
            buf: vec![0u8; size],
            pos: 0,
            filled: 0,
            pending: None,
        }
    }

    /// One `read(2)` of the full buffer size, retrying on `EINTR`.
    fn refill(&mut self) -> bool {
        self.pos = 0;
        self.filled = 0;
        loop {
            match self.file.read(&mut self.buf) {
                Ok(0) => return false,
                Ok(n) => {
                    self.filled = n;
                    return true;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return false,
            }
        }
    }

    /// Bytes read from the fd but not logically consumed by a conversion.
    fn unconsumed(&self) -> u64 {
        (self.filled - self.pos) as u64 + u64::from(self.pending.is_some())
    }

    /// Reproduces libc's exit-time repositioning of a seekable input stream.
    ///
    /// Does nothing for a pipe (where the C cannot seek either, so the bytes it
    /// buffered really are gone) and nothing when there is nothing to give back.
    pub fn reposition_if_seekable(&mut self) {
        let back = self.unconsumed();
        if back == 0 {
            return;
        }
        if (&mut *self.file)
            .seek(SeekFrom::Current(-(back as i64)))
            .is_ok()
        {
            // The buffer is now stale: those bytes are back in the file.
            self.pos = 0;
            self.filled = 0;
            self.pending = None;
        }
    }
}

impl Default for CStdin {
    fn default() -> Self {
        Self::new()
    }
}

impl ByteSource for CStdin {
    fn next_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.pending.take() {
            return Some(b);
        }
        if self.pos == self.filled && !self.refill() {
            return None;
        }
        let b = self.buf[self.pos];
        self.pos += 1;
        Some(b)
    }

    fn push_back(&mut self, b: u8) {
        debug_assert!(self.pending.is_none(), "at most one ungetc is outstanding");
        self.pending = Some(b);
    }
}

// ---------------------------------------------------------------------------
// The conversion
// ---------------------------------------------------------------------------

/// The characters C's `isspace()` accepts in the C locale; a `%d` directive
/// skips these (crossing newlines) before converting.
///
/// The set is verified exhaustively against the C for all 256 byte values in
/// three positions, see `cfg_exe_byte_classification_exhaustive`.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Emulates glibc's `scanf("%d", &x)`.
///
/// Returns `Some(value)` when the conversion succeeds (scanf returned 1) and
/// `None` on a matching failure or on EOF, in which case scanf stores nothing
/// and the caller's variable keeps its previous value.
///
/// Two measured details:
///
/// * **`long` accumulation with clamping, then truncation to `int`.** glibc
///   accumulates into a `long`, clamps to `LONG_MAX`/`LONG_MIN` on overflow and
///   stores the low 32 bits. So `"4294967296"` yields `0` (truncation, not
///   saturation), `"9223372036854775808"` yields `-1` (`LONG_MAX` truncated) and
///   `"-9223372036854775809"` yields `0` (`LONG_MIN` truncated). `"2^64"` would
///   yield `0` if glibc wrapped instead of clamping - it does not.
/// * **Exactly one character of push-back.** On success the terminating
///   non-digit is pushed back; on a matching failure the offending character is
///   pushed back but a sign already consumed is *not* restored. Measured by
///   calling the exported `main` repeatedly on one stream: `"--5"` gives
///   bad/good/bad (so the second `-` came back but the first did not) while
///   `"-a5"` gives bad/bad/bad (so `a` came back and `5` was never reached).
///   Only visible to a caller that runs more than one conversion, which is why
///   the `.so` shares one [`CStdin`] across calls.
pub fn scanf_int<S: ByteSource>(input: &mut S) -> Option<i32> {
    // Skip leading whitespace.
    let mut cur = loop {
        match input.next_byte() {
            Some(b) if is_c_space(b) => continue,
            Some(b) => break b,
            None => return None, // EOF before any input: scanf returns EOF
        }
    };

    // Optional sign.
    let negative = match cur {
        b'-' | b'+' => {
            let neg = cur == b'-';
            match input.next_byte() {
                Some(b) => cur = b,
                None => return None, // sign then EOF: matching failure
            }
            neg
        }
        _ => false,
    };

    if !cur.is_ascii_digit() {
        // Matching failure: the offending character goes back, the sign does not.
        input.push_back(cur);
        return None;
    }

    let mut acc: i64 = 0;
    let mut saturated = false;
    loop {
        if !saturated {
            let digit = i64::from(cur - b'0');
            let next = acc.checked_mul(10).and_then(|v| {
                if negative {
                    v.checked_sub(digit)
                } else {
                    v.checked_add(digit)
                }
            });
            acc = match next {
                Some(v) => v,
                None => {
                    // glibc clamps the accumulator and keeps eating digits.
                    saturated = true;
                    if negative {
                        i64::MIN
                    } else {
                        i64::MAX
                    }
                }
            };
        }

        match input.next_byte() {
            Some(b) if b.is_ascii_digit() => cur = b,
            // The terminating non-digit is pushed back by scanf.
            Some(b) => {
                input.push_back(b);
                break;
            }
            None => break,
        }
    }

    // Store the low 32 bits, exactly like the `%d` (int) store does.
    Some(acc as i32)
}

/// Mirrors `int main()`.
pub fn run<S: ByteSource, W: Write>(input: &mut S, out: &mut W) -> i32 {
    let mut x: i32 = 0;
    if let Some(v) = scanf_int(input) {
        x = v;
    }

    if x != 0 {
        good(out);
    } else {
        bad(out);
    }
    0
}
