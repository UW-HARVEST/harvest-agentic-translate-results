// Rust translation of c_src/src/main.c
//
// Original C:
//     #include <stdio.h>
//
//     /* interactive echo; ignores arguments, copies stdin to stdout */
//     int main() {
//         char text[128];
//
//         while (fgets(text, 128, stdin)) {
//             fputs(text, stdout);
//         }
//         return 0;
//     }
//
// The program echoes stdin to stdout, ignores argv, and always exits 0.
//
// Behaviors that must be reproduced exactly:
//
//   * `fgets(text, 128, stdin)` reads at most 127 bytes, stopping early after a
//     newline (which is kept in the buffer). A line longer than 127 bytes is
//     therefore echoed in several chunks; the concatenation is unchanged.
//   * `fgets` returns NULL only when it managed to read nothing at all (EOF or
//     error on the first byte). If bytes were read and *then* an error occurred,
//     glibc still returns the buffer, so those bytes are echoed.
//   * `fputs` writes a NUL-terminated C string, so an embedded NUL byte in the
//     input truncates whatever remains of that chunk, even though `fgets` read
//     past it.
//   * `fputs`'s return value is discarded, so a write failure does not stop the
//     loop: the program keeps draining stdin and still returns 0.
//   * C stdio buffers stdout: line-buffered when it is a terminal, otherwise
//     block-buffered in `st_blksize` units. This is observable when the process
//     is killed by a signal, because unflushed bytes are lost.
//   * The default SIGPIPE disposition is in effect, so writing to a pipe with no
//     reader kills the process with SIGPIPE (exit status 141 via a shell).

use std::fs::File;
use std::io::{Read, Write};
use std::mem::ManuallyDrop;
use std::process::ExitCode;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::os::unix::io::FromRawFd;

/// Size of `char text[128]` in the C original.
const BUF_SIZE: usize = 128;

/// glibc's `BUFSIZ`, used as the fallback when `st_blksize` is unavailable.
const FALLBACK_BUFSIZ: usize = 8192;

const STDIN_FD: i32 = 0;
const STDOUT_FD: i32 = 1;

#[cfg(unix)]
extern "C" {
    fn isatty(fd: i32) -> i32;
    fn signal(signum: i32, handler: usize) -> usize;
}

/// Borrow an inherited file descriptor as a `File` without ever closing it.
#[cfg(unix)]
fn borrow_fd(fd: i32) -> ManuallyDrop<File> {
    // SAFETY: `fd` is one of the standard descriptors this process inherited. It
    // stays valid for the whole run, and `ManuallyDrop` prevents the returned
    // `File` from closing it when dropped.
    ManuallyDrop::new(unsafe { File::from_raw_fd(fd) })
}

/// The buffer size C stdio picks for a stream: `st_blksize`, or `BUFSIZ` when
/// that is unusable (0, or the descriptor cannot be stat'ed).
#[cfg(unix)]
fn stdio_buffer_size(fd: i32) -> usize {
    let file = borrow_fd(fd);
    match file.metadata() {
        Ok(meta) => {
            let blksize = meta.blksize() as usize;
            if blksize == 0 {
                FALLBACK_BUFSIZ
            } else {
                blksize
            }
        }
        Err(_) => FALLBACK_BUFSIZ,
    }
}

#[cfg(not(unix))]
fn stdio_buffer_size(_fd: i32) -> usize {
    FALLBACK_BUFSIZ
}

#[cfg(unix)]
fn fd_is_tty(fd: i32) -> bool {
    // SAFETY: `isatty` only inspects the descriptor; it touches no memory.
    unsafe { isatty(fd) == 1 }
}

#[cfg(not(unix))]
fn fd_is_tty(_fd: i32) -> bool {
    false
}

/// Rust's runtime ignores SIGPIPE; a C program keeps the default disposition and
/// is killed when the reader of its stdout goes away. Restore that.
fn restore_default_sigpipe() {
    #[cfg(unix)]
    {
        const SIGPIPE: i32 = 13;
        const SIG_DFL: usize = 0;
        // SAFETY: installing the default disposition for a signal is a simple
        // libc call with no memory involved.
        unsafe {
            signal(SIGPIPE, SIG_DFL);
        }
    }
}

/// A buffered reader over a raw descriptor, filling in the same `st_blksize`
/// units C stdio uses so that the amount consumed from stdin matches.
struct CStdin {
    file: ManuallyDrop<File>,
    buf: Vec<u8>,
    pos: usize,
    len: usize,
    /// Sticky: no more bytes will ever arrive (EOF or a hard read error).
    done: bool,
}

impl CStdin {
    fn new() -> Self {
        let cap = stdio_buffer_size(STDIN_FD);
        Self {
            file: borrow_fd(STDIN_FD),
            buf: vec![0u8; cap],
            pos: 0,
            len: 0,
            done: false,
        }
    }

    /// Return the next byte of input, or `None` at EOF / on a read error.
    fn next_byte(&mut self) -> Option<u8> {
        if self.pos == self.len {
            if self.done {
                return None;
            }
            loop {
                match self.file.read(&mut self.buf) {
                    Ok(0) => {
                        self.done = true;
                        return None;
                    }
                    Ok(n) => {
                        self.pos = 0;
                        self.len = n;
                        break;
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => {
                        // C stdio sets the error flag; fgets then reports failure
                        // for this call.
                        self.done = true;
                        return None;
                    }
                }
            }
        }
        let byte = self.buf[self.pos];
        self.pos += 1;
        Some(byte)
    }
}

/// Emulate `fgets(text, BUF_SIZE, stdin)`.
///
/// Reads at most `BUF_SIZE - 1` bytes, stopping after a newline (kept in the
/// result). Returns `None` when nothing at all could be read, which is the only
/// case in which glibc's `fgets` yields NULL. The result holds the raw bytes,
/// without the NUL terminator C appends.
fn fgets(input: &mut CStdin) -> Option<Vec<u8>> {
    let limit = BUF_SIZE - 1;
    let mut out: Vec<u8> = Vec::with_capacity(limit);

    while out.len() < limit {
        match input.next_byte() {
            Some(byte) => {
                out.push(byte);
                if byte == b'\n' {
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

/// A stdout writer that buffers the way C stdio does, so that output surviving a
/// signal-triggered death matches the C program byte for byte.
struct CStdout {
    file: ManuallyDrop<File>,
    buf: Vec<u8>,
    cap: usize,
    line_buffered: bool,
}

impl CStdout {
    fn new() -> Self {
        let cap = stdio_buffer_size(STDOUT_FD);
        Self {
            file: borrow_fd(STDOUT_FD),
            buf: Vec::with_capacity(cap),
            cap,
            line_buffered: fd_is_tty(STDOUT_FD),
        }
    }

    /// Push bytes out to the descriptor, retrying short writes and EINTR.
    /// Errors are swallowed: the C original discards `fputs`'s return value.
    fn write_out(&mut self, mut data: &[u8]) {
        while !data.is_empty() {
            match self.file.write(data) {
                Ok(0) => break,
                Ok(n) => data = &data[n..],
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
    }

    /// Emit `count` leading bytes of the buffer and drop them.
    fn flush_prefix(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        let prefix: Vec<u8> = self.buf[..count].to_vec();
        self.write_out(&prefix);
        self.buf.drain(..count);
    }

    fn flush_all(&mut self) {
        let count = self.buf.len();
        self.flush_prefix(count);
    }

    /// Equivalent of `fputs(s, stdout)` for an already NUL-trimmed slice.
    fn puts(&mut self, data: &[u8]) {
        if self.line_buffered {
            self.buf.extend_from_slice(data);
            // C stdio flushes a line-buffered stream through the last newline.
            if let Some(idx) = self.buf.iter().rposition(|&b| b == b'\n') {
                self.flush_prefix(idx + 1);
            }
            if self.buf.len() >= self.cap {
                self.flush_all();
            }
            return;
        }

        // Block buffered: fill to an exact buffer boundary, flush the full
        // buffer, then carry on. This reproduces C stdio's flush boundaries.
        let mut rest = data;
        while !rest.is_empty() {
            let space = self.cap - self.buf.len();
            let take = space.min(rest.len());
            self.buf.extend_from_slice(&rest[..take]);
            rest = &rest[take..];
            if self.buf.len() >= self.cap {
                self.flush_all();
            }
        }
    }
}

fn main() -> ExitCode {
    restore_default_sigpipe();

    let mut input = CStdin::new();
    let mut output = CStdout::new();

    while let Some(text) = fgets(&mut input) {
        // fputs writes a C string: bytes up to the first NUL. `fgets` may have
        // read past an embedded NUL, and those bytes are silently dropped.
        let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
        output.puts(&text[..end]);
    }

    // exit(0) flushes C stdio's streams.
    output.flush_all();
    ExitCode::from(0)
}
