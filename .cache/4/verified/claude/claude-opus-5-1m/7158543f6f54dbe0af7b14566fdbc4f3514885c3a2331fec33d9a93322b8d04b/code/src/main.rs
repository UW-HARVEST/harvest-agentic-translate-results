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
//! The C program reads a single integer with `scanf("%d", &x)` and prints
//! `2 * x + 300` using `printf("%d\n", y)`. The translation reproduces the
//! original behaviour bit-for-bit, including:
//!
//! * `scanf` skipping leading whitespace (spaces, tabs, newlines, ...), so the
//!   value may appear on any line;
//! * a matching failure or end-of-file leaving `x` at its initial value of `0`;
//! * glibc's `%d` conversion accumulating the digits into a 64-bit `long`
//!   (saturating at `LONG_MAX` / `LONG_MIN` on overflow) and then storing the
//!   truncated low 32 bits into the `int` destination;
//! * the wrapping 32-bit arithmetic of `2*x` and `y += 300`;
//! * glibc's `stdin` buffering: one `read` of `st_blksize` (capped at `BUFSIZ`)
//!   bytes at a time, the single `ungetc` of the character that terminates the
//!   conversion, and the seek that restores a *seekable* `stdin` to the first
//!   unconsumed byte when the stream is cleaned up at exit — so a second reader
//!   sharing the descriptor sees exactly the same leftovers as with the C
//!   program.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::mem::ManuallyDrop;
use std::os::fd::FromRawFd;
use std::os::unix::fs::MetadataExt;

/// Bytes that C's `isspace()` reports as whitespace in the "C" locale, i.e. the
/// characters that `scanf`'s `%d` directive silently consumes before the number.
fn is_c_space(b: u8) -> bool {
    b == b' '        // space
        || b == b'\t' // horizontal tab
        || b == b'\n' // line feed
        || b == 0x0b_u8 // vertical tab
        || b == 0x0c_u8 // form feed
        || b == b'\r' // carriage return
}

/// `stdin` with glibc's `FILE` semantics, to the extent `scanf("%d")` can
/// observe them.
///
/// Rust's `io::Stdin` would over-read (an 8 KiB `BufReader`) and would never
/// give the unconsumed bytes back, which is observable by anything else that
/// shares file descriptor 0 — e.g. `{ ./driver; cat; } < file`. glibc instead
///
/// * fills a buffer of `st_blksize` bytes (capped at `BUFSIZ`, i.e. 8192) per
///   `read` — see `_IO_file_doallocate`,
/// * pushes the character that terminated the conversion back with `ungetc`,
/// * and, while cleaning the stream up at exit, `lseek`s a seekable descriptor
///   back to the first byte the conversion did not consume.
struct CStdin {
    /// File descriptor 0. `ManuallyDrop` because closing it is not ours to do.
    file: ManuallyDrop<File>,
    /// Backing storage, sized to the largest buffer glibc would pick (`BUFSIZ`).
    ///
    /// Deliberately *not* a `Vec`: glibc falls back to the one-byte `_shortbuf`
    /// inside the `FILE` when `malloc` fails (`_IO_doallocbuf`) and keeps
    /// working, whereas a failed Rust allocation aborts the process with a
    /// message on `stderr`. Reserving the space inline cannot fail, so a tight
    /// `RLIMIT_AS` can no longer turn a `384` into a `SIGABRT`.
    buf: [u8; BUFSIZ],
    /// How many of `buf`'s bytes glibc would use per `read`.
    chunk: usize,
    /// Index of the next buffered byte to hand to the parser.
    pos: usize,
    /// Number of valid bytes in `buf`.
    filled: usize,
    /// Bytes actually pulled out of the descriptor.
    read_total: u64,
    /// Bytes handed to the parser.
    handed: u64,
    /// Whether the last handed-out byte was pushed back (glibc's single-byte
    /// `ungetc`), meaning it must not count as consumed.
    pushed_back: bool,
    eof: bool,
}

/// glibc's `BUFSIZ`, the default (and maximum) `stdio` buffer size.
const BUFSIZ: usize = 8192;

impl CStdin {
    fn new() -> CStdin {
        // SAFETY: fd 0 is the process's standard input, which outlives this
        // wrapper; `ManuallyDrop` makes sure it is never closed here.
        let file = ManuallyDrop::new(unsafe { File::from_raw_fd(0) });

        // glibc `_IO_file_doallocate`: `size = BUFSIZ; if (st_blksize > 0 &&
        // st_blksize < BUFSIZ) size = st_blksize;`
        let blksize = file.metadata().map(|m| m.blksize()).unwrap_or(0);
        let chunk = if blksize > 0 && blksize < BUFSIZ as u64 {
            blksize as usize
        } else {
            BUFSIZ
        };

        CStdin {
            file,
            buf: [0u8; BUFSIZ],
            chunk,
            pos: 0,
            filled: 0,
            read_total: 0,
            handed: 0,
            pushed_back: false,
            eof: false,
        }
    }

    /// One character, or `None` at end of file / on a read error — glibc's
    /// `__underflow` does not retry, and an error is an input failure just like
    /// EOF, so neither does this.
    fn next_byte(&mut self) -> Option<u8> {
        if self.pos == self.filled {
            if self.eof {
                return None;
            }
            match self.file.read(&mut self.buf[..self.chunk]) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(n) => {
                    self.filled = n;
                    self.pos = 0;
                    self.read_total += n as u64;
                }
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
        let b = self.buf[self.pos];
        self.pos += 1;
        self.handed += 1;
        Some(b)
    }

    /// glibc's `ungetc_not_eof`: the character that terminated the conversion
    /// goes back onto the stream. At most one character is ever pushed back.
    fn push_back(&mut self) {
        self.pushed_back = true;
    }

    /// What glibc does to a seekable `stdin` when the stream is cleaned up:
    /// rewind the descriptor by however much was read ahead, so the shared file
    /// offset ends up exactly at the first unconsumed byte. Fails harmlessly
    /// with `ESPIPE` for pipes and terminals, exactly like glibc.
    fn restore_offset(&mut self) {
        let consumed = self.handed - u64::from(self.pushed_back);
        let read_ahead = self.read_total - consumed;
        if read_ahead > 0 {
            let _ = self.file.seek(SeekFrom::Current(-(read_ahead as i64)));
        }
    }
}

/// Equivalent of `scanf("%d", &x)`: returns `Some(value)` when the conversion
/// succeeds and `None` on a matching failure or input failure (in which case the
/// C code leaves `x` untouched).
fn scanf_d(input: &mut CStdin) -> Option<i32> {
    // Skip leading whitespace; `%d` crosses newlines while doing so. Running
    // into EOF here is an input failure with nothing to push back.
    let mut cur = loop {
        let b = input.next_byte()?;
        if !is_c_space(b) {
            break b;
        }
    };

    // Optional sign. glibc has already accepted it into its work buffer, so it
    // stays consumed even when the conversion fails afterwards.
    let negative = match cur {
        b'-' | b'+' => {
            let negative = cur == b'-';
            match input.next_byte() {
                Some(b) => cur = b,
                // EOF directly after the sign: a matching failure, but there is
                // no character to give back.
                None => return None,
            }
            negative
        }
        _ => false,
    };

    // At least one decimal digit is required, otherwise this is a matching
    // failure: glibc pushes the offending character back and leaves the
    // destination alone.
    if !cur.is_ascii_digit() {
        input.push_back();
        return None;
    }

    // Accumulate the magnitude the way glibc's `strtol` does: saturate at the
    // `long` limit while still consuming every remaining digit.
    let limit: u64 = if negative {
        // |LONG_MIN| == 2^63
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };
    let mut magnitude: u64 = 0;
    let mut saturated = false;

    loop {
        let digit = u64::from(cur - b'0');
        if !saturated {
            match magnitude
                .checked_mul(10)
                .and_then(|v| v.checked_add(digit))
            {
                Some(v) if v <= limit => magnitude = v,
                _ => {
                    saturated = true;
                    magnitude = limit;
                }
            }
        }

        match input.next_byte() {
            Some(b) if b.is_ascii_digit() => cur = b,
            // The first non-digit terminates the conversion and is pushed back
            // onto the stream; at EOF there is nothing to push back.
            Some(_) => {
                input.push_back();
                break;
            }
            None => break,
        }
    }

    // Value as stored in a `long`, then truncated to the `int` destination.
    let as_long: i64 = if negative {
        (magnitude.wrapping_neg()) as i64
    } else {
        magnitude as i64
    };
    Some(as_long as i32)
}

/// `printf("%d\n", y)` without allocating.
///
/// `std::io::stdout()` would lazily allocate a 1024-byte `LineWriter`, and a
/// failed Rust allocation aborts the process with a message on `stderr`, whereas
/// glibc's `printf` keeps working (it falls back to the `FILE`'s `_shortbuf`).
/// Formatting into a stack buffer and issuing the write directly keeps the whole
/// program allocation-free, so a tight `RLIMIT_AS` cannot turn the C's `384`
/// into a `SIGABRT`.
///
/// The emitted bytes and the syscall pattern are unchanged: glibc buffers the
/// four bytes and flushes them with a single `write`, and so does this.
fn print_d_line(v: i32) {
    // "-2147483648\n" is the longest possible result: 11 characters + newline.
    let mut buf = [0u8; 12];
    let mut start = buf.len();

    start -= 1;
    buf[start] = b'\n';

    let mut magnitude = v.unsigned_abs();
    loop {
        start -= 1;
        buf[start] = b'0' + (magnitude % 10) as u8;
        magnitude /= 10;
        if magnitude == 0 {
            break;
        }
    }
    if v < 0 {
        start -= 1;
        buf[start] = b'-';
    }

    // SAFETY: fd 1 is the process's standard output, which outlives this
    // borrow; `ManuallyDrop` makes sure it is never closed here.
    let mut out = ManuallyDrop::new(unsafe { File::from_raw_fd(1) });
    let mut written = 0usize;
    let bytes = &buf[start..];
    // glibc's `_IO_new_file_write` loops over partial writes and gives up on the
    // first error (`printf` then returns -1, which the C code ignores).
    while written < bytes.len() {
        match out.write(&bytes[written..]) {
            Ok(0) => break,
            Ok(n) => written += n,
            Err(_) => break,
        }
    }
}

/// Faithful translation of the C `driver` function (`register` is a no-op hint).
pub fn driver(x: i32) {
    let mut y: i32 = 2i32.wrapping_mul(x);
    y = y.wrapping_add(300);
    print_d_line(y);
}

/// Faithful translation of the C `main` body: read one integer, call `driver`,
/// return `0`.
///
/// This is the part that is shared verbatim with the `cdylib`'s exported `main`
/// (see `src/ffi.rs`); it deliberately touches no process-global state, exactly
/// like the C `main`.
pub fn run() {
    let mut x: i32 = 0;
    let mut input = CStdin::new();
    if let Some(v) = scanf_d(&mut input) {
        x = v;
    }
    driver(x);
    // glibc does this while cleaning up `stdin` at exit, i.e. after `printf`.
    input.restore_offset();
}

#[cfg(unix)]
mod sigpipe {
    //! Undoing Rust's `SIGPIPE` policy.
    //!
    //! A C program runs with whatever `SIGPIPE` disposition it inherited across
    //! `execve` — either `SIG_DFL` (the usual case, which *kills* the process
    //! when it writes to a pipe with no reader) or `SIG_IGN` (when the parent
    //! ignored it; only `SIG_DFL` and `SIG_IGN` survive `execve`). Rust's
    //! runtime overwrites it with `SIG_IGN` before `main` runs, so
    //!
    //! * with a normal parent, the C dies from signal 13 while an unpatched
    //!   Rust translation ignores the failed `write` and exits `0`;
    //! * with a `SIGPIPE`-ignoring parent (a `fork`+`exec` daemon, or anything
    //!   launched from CPython, which ignores `SIGPIPE` itself), it is the other
    //!   way round — unconditionally forcing `SIG_DFL` would then *add* a death
    //!   the C program does not have.
    //!
    //! Both cases are handled by recording the inherited disposition in an ELF
    //! `.init_array` constructor — which the loader runs *before* Rust's runtime
    //! initialisation — and putting it back at the start of `main`.

    use std::sync::atomic::{AtomicUsize, Ordering};

    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    /// `SIG_ERR` is `(void (*)(int)) -1`.
    const SIG_ERR: usize = usize::MAX;

    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }

    static INHERITED: AtomicUsize = AtomicUsize::new(SIG_DFL);

    /// Reads the current `SIGPIPE` disposition without changing it: `signal`
    /// returns the previous handler, which is immediately reinstalled.
    extern "C" fn capture() {
        // SAFETY: `signal` is valid for `SIGPIPE`; the disposition is restored
        // to exactly the value that was read before anything can write to a
        // pipe, so the swap is not observable.
        unsafe {
            let prev = signal(SIGPIPE, SIG_DFL);
            if prev == SIG_ERR {
                return;
            }
            signal(SIGPIPE, prev);
            INHERITED.store(prev, Ordering::Relaxed);
        }
    }

    /// Run by the dynamic loader before Rust's runtime replaces the disposition.
    #[used]
    #[link_section = ".init_array"]
    static CAPTURE_CTOR: extern "C" fn() = capture;

    /// Puts the inherited disposition back, undoing Rust's `SIG_IGN`.
    ///
    /// Only the *executable* entry point does this: the C `main` compiled into a
    /// shared object does not touch signal dispositions, so the `cdylib` export
    /// in `src/ffi.rs` calls [`super::run`] directly instead.
    pub fn restore_inherited() {
        // SAFETY: as above; the value stored by `capture` is a disposition that
        // was already installed for `SIGPIPE` in this process.
        unsafe {
            signal(SIGPIPE, INHERITED.load(Ordering::Relaxed));
        }
    }
}

#[cfg(not(unix))]
mod sigpipe {
    pub fn restore_inherited() {}
}

pub fn main() {
    sigpipe::restore_inherited();
    run();
}
