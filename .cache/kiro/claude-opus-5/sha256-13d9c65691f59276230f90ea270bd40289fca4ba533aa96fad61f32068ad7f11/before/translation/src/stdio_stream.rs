//! A minimal stand-in for a buffered stdio stream (`FILE *`).
//!
//! `BufWriter` is deliberately not used: it never splits a single `write_all`
//! across two flushes, whereas `fwrite` fills its buffer right up to the
//! boundary and flushes exactly `BUFSIZ` bytes.  That difference is observable
//! whenever the log stream shares a file descriptor with something else (e.g.
//! `LOG_FILE=/dev/stdout`), so the stdio behaviour is reproduced here.

use std::fs::File;
use std::io::Write;

pub struct StdioStream {
    file: File,
    buf: Vec<u8>,
    capacity: usize,
}

impl StdioStream {
    pub fn new(file: File, capacity: usize) -> Self {
        StdioStream {
            file,
            buf: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// `fwrite(data, 1, data.len(), stream)`
    pub fn write(&mut self, data: &[u8]) {
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
