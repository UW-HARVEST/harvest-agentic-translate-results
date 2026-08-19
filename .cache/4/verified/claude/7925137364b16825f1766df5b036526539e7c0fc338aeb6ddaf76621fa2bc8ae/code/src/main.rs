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
//
// Rust translation of c_src/src/main.c
//
// The C program declares `y` as a file-scope (static) `int` initialized to 123,
// then reads three integers with a single `scanf("%d %d %d", &x, &y, &z)` whose
// return value is ignored.  Any variable that scanf fails to convert keeps its
// previous value (x = 0, y = 123, z = 0).  The translation reproduces this
// exactly, including the "partial conversion" behavior on malformed / short
// input.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::mem::ManuallyDrop;
use std::os::fd::FromRawFd;
use std::os::unix::fs::MetadataExt;

/// glibc's `BUFSIZ`.
const BUFSIZ: usize = 8192;

// ---------------------------------------------------------------------------
// Global mutable state mirroring the C file-scope `static int y = 123;`
// ---------------------------------------------------------------------------
struct Globals {
    y: i32,
}

// ---------------------------------------------------------------------------
// Stand-in for glibc's `stdin` FILE stream.
//
// It is not enough to "read stdin somehow": how much of the descriptor a C
// program swallows is observable by whatever reads the same descriptor next
// (`{ driver; cat; } < file`).  glibc behaves like this:
//
//   * the stream buffer is `st_blksize` bytes of the descriptor (falling back to
//     `BUFSIZ` when it cannot be stat'ed) — `_IO_file_doallocate`;
//   * each underflow performs exactly one `read()` into that buffer;
//   * at `exit()`, `_IO_cleanup` unbuffers every stream, which syncs a seekable
//     input descriptor back to the stream's *logical* position, i.e. the
//     read-ahead that `scanf` never consumed is handed back.
//
// Using `std::io::stdin()` instead would read 8 KiB chunks and never seek back,
// leaving a different file offset / different pipe contents behind.
// ---------------------------------------------------------------------------
struct CStdin {
    // fd 0 is owned by the process, not by this wrapper: never close it.
    file: ManuallyDrop<File>,
    buf: Vec<u8>,
    /// Index of the next unread byte in `buf` (the stream's logical position).
    pos: usize,
    /// Number of valid bytes in `buf`.
    len: usize,
    eof: bool,
}

impl CStdin {
    fn new() -> Self {
        // Safety: fd 0 stays open for the whole program and `ManuallyDrop`
        // keeps this wrapper from closing it.
        let file = ManuallyDrop::new(unsafe { File::from_raw_fd(0) });
        let size = match file.metadata() {
            Ok(md) => {
                let bs = md.blksize() as usize;
                if bs > 0 && bs < BUFSIZ {
                    bs
                } else {
                    BUFSIZ
                }
            }
            // Not stat-able (e.g. fd 0 closed): glibc keeps BUFSIZ.
            Err(_) => BUFSIZ,
        };
        CStdin {
            file,
            buf: vec![0u8; size],
            pos: 0,
            len: 0,
            eof: false,
        }
    }

    /// Returns the next byte of the stream, or None at end-of-file / read error
    /// (glibc reports both as a `scanf` input failure).
    fn next_byte(&mut self) -> Option<u8> {
        while self.pos == self.len {
            if self.eof {
                return None;
            }
            match self.file.read(&mut self.buf) {
                Ok(0) => {
                    self.eof = true;
                    self.pos = 0;
                    self.len = 0;
                    return None;
                }
                Ok(n) => {
                    self.pos = 0;
                    self.len = n;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    self.pos = 0;
                    self.len = 0;
                    return None;
                }
            }
        }
        let b = self.buf[self.pos];
        self.pos += 1;
        Some(b)
    }

    /// Equivalent of glibc pushing the read pointer back by one byte, which is
    /// what `%d` does with the first character that is not part of the number.
    /// Only ever called immediately after the byte was read.
    fn unread(&mut self) {
        debug_assert!(self.pos > 0);
        self.pos -= 1;
    }

    /// Mirrors the exit-time `_IO_cleanup` sync: give the unconsumed read-ahead
    /// back to a seekable descriptor.  Fails harmlessly on pipes (ESPIPE), just
    /// like glibc's own attempt.
    fn sync(&mut self) {
        let unread = self.len - self.pos;
        if unread > 0 {
            let _ = self.file.seek(SeekFrom::Current(-(unread as i64)));
        }
    }
}

// ---------------------------------------------------------------------------
// Minimal `scanf("%d")`-compatible reader on top of that stream.
// ---------------------------------------------------------------------------
struct ScanReader {
    src: CStdin,
}

impl ScanReader {
    fn new(src: CStdin) -> Self {
        ScanReader { src }
    }

    /// Returns the next byte of the stream, or None at end-of-file.
    fn next_byte(&mut self) -> Option<u8> {
        self.src.next_byte()
    }

    /// Equivalent of ungetc(): pushes a single byte back onto the stream.
    fn unget(&mut self, _b: u8) {
        self.src.unread();
    }

    /// C `isspace()` for the "C" locale.
    fn is_space(b: u8) -> bool {
        matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    }

    /// Skips leading whitespace, exactly as a `%d` directive (and the literal
    /// spaces in the format string) does.  Whitespace includes newlines, so a
    /// single scanf call happily reads across line boundaries.
    fn skip_whitespace(&mut self) {
        while let Some(b) = self.next_byte() {
            if !Self::is_space(b) {
                self.unget(b);
                return;
            }
        }
    }

    /// Performs one `%d` conversion.
    ///
    /// Returns `Some(value)` on success, or `None` on input failure (EOF before
    /// any non-whitespace character) or matching failure (no digits present).
    /// On overflow the value saturates like glibc's internal `strtol` and is
    /// then truncated to `int`, matching the observable glibc behavior.
    fn scan_i32(&mut self) -> Option<i32> {
        self.skip_whitespace();

        let mut negative = false;
        let first = self.next_byte()?; // EOF -> input failure
        let mut cur = match first {
            b'-' => {
                negative = true;
                self.next_byte()
            }
            b'+' => self.next_byte(),
            other => Some(other),
        };

        let mut digits = 0usize;
        let mut acc: i128 = 0;
        let mut saturated = false;

        while let Some(c) = cur {
            if !c.is_ascii_digit() {
                self.unget(c);
                break;
            }
            digits += 1;
            if !saturated {
                acc = acc * 10 + i128::from(c - b'0');
                if acc > i128::from(u64::MAX) {
                    // Far past any 64-bit magnitude; clamping is unavoidable.
                    saturated = true;
                }
            }
            cur = self.next_byte();
        }

        if digits == 0 {
            // Matching failure: scanf stops here without assigning anything.
            return None;
        }

        let signed: i128 = if negative { -acc } else { acc };
        let clamped: i64 = if signed > i128::from(i64::MAX) {
            i64::MAX
        } else if signed < i128::from(i64::MIN) {
            i64::MIN
        } else {
            signed as i64
        };
        Some(clamped as i32)
    }
}

// ---------------------------------------------------------------------------
// Translation of `static int multi_stage(int x, int z)`
// ---------------------------------------------------------------------------
fn multi_stage<W: Write>(out: &mut W, g: &Globals, x: i32, z: i32) -> i32 {
    // The three validation stages, in the exact order the C code checks them.
    // `Err(code)` corresponds to a `goto fail` with `result = code`.
    let stages: Result<(), i32> = (|| {
        if x != 1 {
            print_str(out, "Error: x != 1\n");
            return Err(1);
        }

        if g.y != 2 {
            print_str(out, "Error: x == 1 but y != 2\n");
            return Err(2);
        }

        if z != 3 {
            print_str(out, "Error: x == 1 and y == 2, but z != 3\n");
            return Err(3);
        }

        Ok(())
    })();

    let result = match stages {
        Ok(()) => {
            print_str(out, "Ok!\n");
            return 0; // `result` is still 0 on the success path
        }
        Err(code) => code,
    };

    // fail:
    print_str(out, "Operation failed\n");
    result
}

fn print_str<W: Write>(out: &mut W, s: &str) {
    // printf() ignores write errors as far as this program is concerned.
    let _ = out.write_all(s.as_bytes());
}

// ---------------------------------------------------------------------------
// A C process starts with the default disposition for SIGPIPE, so a failing
// write to a closed pipe kills the process with signal 13.  The Rust runtime
// sets SIGPIPE to SIG_IGN before calling `main`, which would instead turn the
// failed `printf` into an ignored `EPIPE` and an exit status of 0.  Restore the
// C behaviour so the translation is observably identical.
// ---------------------------------------------------------------------------
#[cfg(unix)]
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;

    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }

    // Safety: `signal` with SIG_DFL is always valid; the return value (the old
    // handler) is intentionally discarded, exactly as a C program's default
    // startup state would leave it.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

// ---------------------------------------------------------------------------
// Translation of `int main()`
// ---------------------------------------------------------------------------
fn main() {
    restore_default_sigpipe();

    let mut g = Globals { y: 123 };

    let mut x: i32 = 0;
    let mut z: i32 = 0;

    // scanf("%d %d %d", &x, &y, &z); -- return value ignored, so variables that
    // are not converted retain their prior values.  A failed conversion aborts
    // the whole call, leaving the remaining arguments untouched.
    let mut reader = ScanReader::new(CStdin::new());
    if let Some(v) = reader.scan_i32() {
        x = v;
        if let Some(v) = reader.scan_i32() {
            g.y = v;
            if let Some(v) = reader.scan_i32() {
                z = v;
            }
        }
    }

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    let result = multi_stage(&mut out, &g, x, z);
    let _ = writeln!(out, "Result: {}", result);

    // `return 0;` from main -> exit() -> glibc's _IO_cleanup: output streams are
    // flushed first, then every stream is unbuffered, which returns the unused
    // stdin read-ahead to a seekable descriptor.
    let _ = out.flush();
    reader.src.sync();
}
