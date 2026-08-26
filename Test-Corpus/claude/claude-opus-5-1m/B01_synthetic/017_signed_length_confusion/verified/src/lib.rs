// Rust translation of c_src/src/main.c
//
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
// The translation intentionally reproduces the original (buggy) behavior of the
// C program, including the negative-length `strncpy` call that the C code
// performs when the parsed value is negative.

use std::ffi::c_char;
use std::io::{IsTerminal, Read, Write};
use std::sync::Mutex;

/// Mimics C's single global `stdout` FILE* and its buffering discipline: line
/// buffered when attached to a terminal, fully buffered otherwise (data is only
/// written at flush time).  This matters because the program can die from a
/// memory fault before any flush happens, in which case a fully buffered stream
/// loses its contents.
///
/// Like C's `stdout` this is process-global state shared by every caller of
/// `printLine`, and the buffering mode is decided lazily on first use (that is
/// when glibc calls `isatty` on the descriptor).
struct CStdout {
    buf: Vec<u8>,
    line_buffered: Option<bool>,
    atexit_registered: bool,
}

static C_STDOUT: Mutex<CStdout> = Mutex::new(CStdout {
    buf: Vec::new(),
    line_buffered: None,
    atexit_registered: false,
});

extern "C" {
    /// Used to reproduce glibc's flush-of-`stdout`-at-`exit` behavior, which is
    /// what makes buffered output observable to callers that simply return from
    /// `main` (or call `exit`) without an explicit `fflush`.
    fn atexit(cb: extern "C" fn()) -> i32;
}

extern "C" fn flush_at_exit() {
    flush_stdout();
}

/// `printf("%s\n", line)` on a NUL-terminated byte string.
fn printf_line(line: &[u8]) {
    let mut out = match C_STDOUT.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    if out.line_buffered.is_none() {
        out.line_buffered = Some(std::io::stdout().is_terminal());
    }
    if !out.atexit_registered {
        out.atexit_registered = true;
        unsafe {
            atexit(flush_at_exit);
        }
    }

    out.buf.extend_from_slice(line);
    out.buf.push(b'\n');

    if out.line_buffered == Some(true) {
        let buf = std::mem::take(&mut out.buf);
        drop(out);
        write_through(&buf);
    }
}

fn write_through(bytes: &[u8]) {
    if bytes.is_empty() {
        return;
    }
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(bytes);
    let _ = handle.flush();
}

/// `fflush(stdout)`.
pub fn flush_stdout() {
    let buf = {
        let mut out = match C_STDOUT.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        std::mem::take(&mut out.buf)
    };
    write_through(&buf);
}

/// void printLine (const char * line)
fn print_line(line: Option<&[u8]>) {
    if let Some(line) = line {
        printf_line(line);
    }
}

/// `void printLine (const char * line)` — exported with C linkage so that
/// external callers see the same symbol the C translation unit provides.
///
/// # Safety
/// `line` must either be NULL or point to a NUL-terminated string, exactly as
/// required by the C function.
#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let mut len = 0usize;
        while *line.add(len) != 0 {
            len += 1;
        }
        let bytes = std::slice::from_raw_parts(line as *const u8, len);
        print_line(Some(bytes));
    }
}

/// Length of a NUL-terminated string held in a byte buffer (`strlen`).
fn c_strlen(buf: &[u8]) -> usize {
    match buf.iter().position(|&b| b == 0) {
        Some(pos) => pos,
        None => buf.len(),
    }
}

/// Borrow the NUL-terminated string stored at the start of `buf`.
fn c_str(buf: &[u8]) -> &[u8] {
    &buf[..c_strlen(buf)]
}

/// C's global `stdin` FILE*, reproducing glibc's read discipline. This is
/// observable to anything sharing descriptor 0 with this process:
///
/// * glibc allocates the stream buffer from `fstat`'s `st_blksize` (4096 here,
///   `BUFSIZ` as a fallback) and refills it with **one** `read(2)` of that size
///   — so a 3-byte line read from a pipe still consumes 4096 bytes of it.
/// * at `exit`, stream cleanup seeks a *seekable* stdin back to the stream's
///   logical position, as C11/POSIX require of `fclose`; a process that dies
///   from a signal never gets that far, leaving the raw read offset in place.
struct CStdin {
    buf: Vec<u8>,
    pos: usize,
    len: usize,
    start_off: Option<i64>,
    consumed: i64,
    initialized: bool,
}

static C_STDIN: Mutex<CStdin> = Mutex::new(CStdin {
    buf: Vec::new(),
    pos: 0,
    len: 0,
    start_off: None,
    consumed: 0,
    initialized: false,
});

extern "C" {
    fn lseek(fd: i32, offset: i64, whence: i32) -> i64;
}

const SEEK_SET: i32 = 0;
const SEEK_CUR: i32 = 1;
const BUFSIZ: usize = 8192;

/// A `File` for descriptor 0 that must not close it when dropped.
fn stdin_file() -> std::mem::ManuallyDrop<std::fs::File> {
    use std::os::fd::FromRawFd;
    std::mem::ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(0) })
}

impl CStdin {
    fn init(&mut self) {
        if self.initialized {
            return;
        }
        self.initialized = true;

        // glibc: buffer size comes from fstat's st_blksize.
        let blksize = {
            use std::os::unix::fs::MetadataExt;
            let f = stdin_file();
            match f.metadata() {
                Ok(m) if m.blksize() > 0 => m.blksize() as usize,
                _ => BUFSIZ,
            }
        };
        self.buf = vec![0u8; blksize];

        // Remember where the stream started, if the descriptor is seekable.
        let off = unsafe { lseek(0, 0, SEEK_CUR) };
        self.start_off = if off < 0 { None } else { Some(off) };
    }

    /// One `read(2)` of the whole buffer, as `_IO_file_underflow` does.
    /// Returns false on EOF or error.
    fn underflow(&mut self) -> bool {
        let n = {
            let mut f = stdin_file();
            match f.read(&mut self.buf) {
                Ok(n) => n,
                // glibc's stdio does not retry interrupted reads.
                Err(_) => 0,
            }
        };
        self.pos = 0;
        self.len = n;
        n > 0
    }
}

/// `fgets(buf, size, stdin)` for a buffer of `buf.len()` bytes.
///
/// Reads at most `buf.len() - 1` bytes, stopping right after a newline, and
/// NUL-terminates.  Returns `false` (the C `NULL` return) when end-of-file or
/// an error is hit before any character is read.
fn fgets(buf: &mut [u8]) -> bool {
    let limit = buf.len() - 1;
    let mut stdin = match C_STDIN.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    stdin.init();

    let mut count = 0usize;
    while count < limit {
        if stdin.pos == stdin.len && !stdin.underflow() {
            break; // EOF or error
        }
        let byte = stdin.buf[stdin.pos];
        stdin.pos += 1;
        buf[count] = byte;
        count += 1;
        if byte == b'\n' {
            break;
        }
    }
    stdin.consumed += count as i64;

    if count == 0 {
        return false;
    }
    buf[count] = 0;
    true
}

/// glibc's exit-time stream cleanup for `stdin`: on a seekable descriptor the
/// file offset is rewound from wherever the block read landed to the stream's
/// logical position, so a process sharing the descriptor sees only the bytes
/// this program actually consumed.
fn cleanup_stdin() {
    let stdin = match C_STDIN.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if !stdin.initialized {
        return;
    }
    if let Some(start) = stdin.start_off {
        unsafe {
            lseek(0, start + stdin.consumed, SEEK_SET);
        }
    }
}

/// Everything C's `exit` does to the standard streams: flush `stdout` and put a
/// seekable `stdin` back at its logical position.
pub fn cleanup_streams() {
    flush_stdout();
    cleanup_stdin();
}

/// `atoi(nptr)`, i.e. glibc's `(int) strtol(nptr, NULL, 10)`:
/// skip leading whitespace, optional sign, decimal digits, saturate at the
/// `long` limits, then truncate to `int`.
fn atoi(s: &[u8]) -> i32 {
    let s = c_str(s);
    let mut i = 0usize;

    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let mut value: i64 = 0;
    let mut saturated = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = i64::from(s[i] - b'0');
        if !saturated {
            match value.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => value = v,
                None => saturated = true,
            }
        }
        i += 1;
    }

    let result: i64 = if saturated {
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

    result as i32
}

/// `strncpy(dest, src, n)` faithful to glibc: copy up to `n` bytes of the
/// NUL-terminated `src`, then pad the remainder of the `n` bytes with NUL.
///
/// The C program calls this with a negative `int` length that converts to a
/// huge `size_t`, so the padding step walks far past the end of the
/// destination buffer and faults.  That fault is part of the observable
/// behavior being reproduced here, so the write is performed for real.
fn strncpy(dest: &mut [u8; 100], src: &[u8; 100], n: usize) {
    let src_len = c_strlen(src);
    let to_copy = if src_len < n { src_len } else { n };

    if to_copy <= dest.len() {
        dest[..to_copy].copy_from_slice(&src[..to_copy]);
    } else {
        // Not reachable for this program (src_len is 99), kept for fidelity.
        unsafe {
            std::ptr::copy_nonoverlapping(src.as_ptr(), dest.as_mut_ptr(), to_copy);
        }
    }

    if n > to_copy {
        let pad = n - to_copy;
        if to_copy + pad <= dest.len() {
            for b in dest[to_copy..to_copy + pad].iter_mut() {
                *b = 0;
            }
        } else {
            // Reproduce the out-of-bounds fill performed by the C code.
            unsafe {
                std::ptr::write_bytes(dest.as_mut_ptr().add(to_copy), 0u8, pad);
            }
        }
    }
}

/// `int main()`
pub fn run() -> i32 {
    let mut data: i32;
    data = -1;
    {
        let mut input_buffer = [0u8; 14];
        if fgets(&mut input_buffer) {
            /* Convert to int */
            data = atoi(&input_buffer);
        } else {
            print_line(Some(b"fgets() failed."));
        }
    }
    {
        let mut source = [0u8; 100];
        let mut dest = [0u8; 100];
        for b in source[..100 - 1].iter_mut() {
            *b = b'A';
        }
        source[100 - 1] = 0;
        if data < 100 {
            strncpy(&mut dest, &source, data as usize);
            let idx = data as usize;
            if idx < dest.len() {
                dest[idx] = 0;
            } else {
                unsafe {
                    std::ptr::write(dest.as_mut_ptr().add(idx), 0u8);
                }
            }
        }
        let line = c_str(&dest).to_vec();
        print_line(Some(&line));
    }

    0
}
