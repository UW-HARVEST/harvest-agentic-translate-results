//! A minimal stand-in for a buffered stdio stream (`FILE *`).
//!
//! `BufWriter` is deliberately not used: it never splits a single `write_all`
//! across two flushes, whereas `fwrite` fills its buffer right up to the
//! boundary and flushes exactly one buffer's worth.  That difference is
//! observable whenever the log stream shares a file descriptor with something
//! else (e.g. `LOG_FILE=/dev/stdout`), so the stdio behaviour is reproduced
//! here — including glibc's choice of buffer size and buffering *mode*, which
//! depend on what the freshly opened file turns out to be.

use std::fs::File;
use std::io::Write;

/// glibc's `BUFSIZ`, the fallback buffer size in `_IO_file_doallocate`.
const BUFSIZ: usize = 8192;

/// `S_IFMT` / `S_IFCHR`
const S_IFMT: u32 = 0o170000;
const S_IFCHR: u32 = 0o020000;

extern "C" {
    fn isatty(fd: std::ffi::c_int) -> std::ffi::c_int;
}

pub struct StdioStream {
    file: File,
    buf: Vec<u8>,
    capacity: usize,
    /// `_IO_LINE_BUF`: set for character devices that are terminals.
    line_buffered: bool,
}

impl StdioStream {
    /// Picks the buffer exactly as glibc's `_IO_file_doallocate` does on the
    /// first write to a freshly `fopen`ed stream:
    ///
    /// ```c
    /// size = BUFSIZ;
    /// if (fstat (fd, &st) >= 0) {
    ///     if (S_ISCHR (st.st_mode) && isatty (fd))
    ///         fp->_flags |= _IO_LINE_BUF;
    ///     if (st.st_blksize > 0 && st.st_blksize < BUFSIZ)
    ///         size = st.st_blksize;
    /// }
    /// ```
    ///
    /// So a regular file on a 4 KiB filesystem gets a 4096-byte fully buffered
    /// stream, while a pty slave (`st_blksize == 1024`) gets a 1024-byte *line*
    /// buffered one.
    pub fn from_file(file: File) -> Self {
        let mut capacity = BUFSIZ;
        let mut line_buffered = false;

        if let Ok(md) = file.metadata() {
            use std::os::unix::fs::MetadataExt;
            use std::os::unix::io::AsRawFd;
            if md.mode() & S_IFMT == S_IFCHR && unsafe { isatty(file.as_raw_fd()) } != 0 {
                line_buffered = true;
            }
            let blksize = md.blksize() as usize;
            if blksize > 0 && blksize < BUFSIZ {
                capacity = blksize;
            }
        }

        StdioStream {
            file,
            buf: Vec::with_capacity(capacity),
            capacity,
            line_buffered,
        }
    }

    /// Explicit buffer configuration; used by the tests.
    pub fn new(file: File, capacity: usize) -> Self {
        StdioStream {
            file,
            buf: Vec::with_capacity(capacity),
            capacity,
            line_buffered: false,
        }
    }

    /// `fwrite(data, 1, data.len(), stream)`
    pub fn write(&mut self, data: &[u8]) {
        if self.line_buffered {
            // `_IO_new_file_xsputn` on a line-buffered stream flushes through
            // the *last* newline of the write; whatever follows it stays in the
            // buffer.
            self.buf.extend_from_slice(data);
            match self.buf.iter().rposition(|&b| b == b'\n') {
                Some(pos) => self.flush_prefix(pos + 1),
                None => {
                    while self.buf.len() >= self.capacity {
                        self.flush_prefix(self.capacity);
                    }
                }
            }
            return;
        }

        let mut rest = data;
        while !rest.is_empty() {
            let space = self.capacity - self.buf.len();
            let take = space.min(rest.len());
            self.buf.extend_from_slice(&rest[..take]);
            rest = &rest[take..];
            if self.buf.len() == self.capacity {
                self.flush();
            }
        }
    }

    fn flush_prefix(&mut self, n: usize) {
        let n = n.min(self.buf.len());
        if n == 0 {
            return;
        }
        let _ = self.file.write_all(&self.buf[..n]);
        self.buf.drain(..n);
    }

    /// `fflush(stream)`; write errors are ignored, just as the C code ignores
    /// the return value of `fprintf`/`fclose`.
    pub fn flush(&mut self) {
        if !self.buf.is_empty() {
            let _ = self.file.write_all(&self.buf);
            self.buf.clear();
        }
    }
}

impl Drop for StdioStream {
    /// `fclose(stream)`: flush, then close.
    fn drop(&mut self) {
        self.flush();
    }
}
