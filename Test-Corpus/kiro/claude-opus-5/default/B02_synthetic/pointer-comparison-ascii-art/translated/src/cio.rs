//! Minimal emulation of the C stdio behaviour that the original program relies
//! on: a byte oriented `stdin` shared by `fgets`/`scanf`/`getchar`, a block
//! buffered `stdout` (line buffered on a terminal, like glibc) and an
//! unbuffered `stderr`.

use std::io::{IsTerminal, Read, Write};

/// glibc picks the buffer size from `st_blksize`, which is 4096 for pipes and
/// regular files on Linux.
const BUFSIZ: usize = 4096;

/// A `printf` argument. The original code only ever uses `%s`, `%d` and `%p`.
pub enum Arg<'a> {
    S(&'a [u8]),
    D(i32),
    P(usize),
}

pub struct Out {
    buf: Vec<u8>,
    line_buffered: bool,
}

impl Out {
    pub fn new() -> Out {
        Out {
            buf: Vec::with_capacity(BUFSIZ),
            line_buffered: std::io::stdout().is_terminal(),
        }
    }

    pub fn put(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
        if self.line_buffered {
            // glibc flushes the whole buffer as soon as a newline is written.
            if bytes.contains(&b'\n') {
                self.flush();
            }
        } else {
            while self.buf.len() >= BUFSIZ {
                let rest = self.buf.split_off(BUFSIZ);
                let chunk = std::mem::replace(&mut self.buf, rest);
                write_stdout(&chunk);
            }
        }
    }

    pub fn flush(&mut self) {
        if !self.buf.is_empty() {
            let chunk = std::mem::take(&mut self.buf);
            write_stdout(&chunk);
        }
    }
}

fn write_stdout(bytes: &[u8]) {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(bytes);
    let _ = lock.flush();
}

pub struct In {
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
}

impl In {
    pub fn new() -> In {
        In {
            buf: Vec::new(),
            pos: 0,
            eof: false,
        }
    }

    /// `getchar()`
    pub fn getchar(&mut self) -> Option<u8> {
        if self.pos >= self.buf.len() {
            if self.eof {
                return None;
            }
            let mut tmp = [0u8; BUFSIZ];
            match std::io::stdin().read(&mut tmp) {
                Ok(0) | Err(_) => {
                    self.eof = true;
                    return None;
                }
                Ok(n) => {
                    self.buf.clear();
                    self.buf.extend_from_slice(&tmp[..n]);
                    self.pos = 0;
                }
            }
        }
        let c = self.buf[self.pos];
        self.pos += 1;
        Some(c)
    }

    /// `ungetc()` of the character that was just read.
    fn unget(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }

    /// `fgets(buf, size, stdin)`. Returns the raw bytes stored in the buffer
    /// (newline included) or `None` when `fgets` would return `NULL`.
    pub fn fgets(&mut self, size: usize) -> Option<Vec<u8>> {
        if size == 0 {
            return None;
        }
        let mut out: Vec<u8> = Vec::new();
        while out.len() < size - 1 {
            match self.getchar() {
                None => break,
                Some(c) => {
                    out.push(c);
                    if c == b'\n' {
                        break;
                    }
                }
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    /// `scanf("%d", &x)`. `None` models any return value other than 1.
    pub fn scan_int(&mut self) -> Option<i32> {
        scan_int_generic(self)
    }

    /// `while (getchar() != '\n');`
    ///
    /// NOTE: this reproduces the original bug -- at end of file `getchar()`
    /// keeps returning `EOF`, which never equals `'\n'`, so the C program spins
    /// forever. The behaviour is preserved intentionally.
    pub fn skip_to_newline(&mut self) {
        loop {
            match self.getchar() {
                Some(b'\n') => return,
                _ => continue,
            }
        }
    }
}

fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Shared abstraction over a byte stream with one character of push-back,
/// which is all `scanf` needs.
trait ByteSource {
    fn getc(&mut self) -> Option<u8>;
    fn unget(&mut self);
}

impl ByteSource for In {
    fn getc(&mut self) -> Option<u8> {
        self.getchar()
    }
    fn unget(&mut self) {
        In::unget(self)
    }
}

impl ByteSource for FileIn {
    fn getc(&mut self) -> Option<u8> {
        FileIn::getc(self)
    }
    fn unget(&mut self) {
        FileIn::unget(self)
    }
}

fn scan_int_generic(src: &mut dyn ByteSource) -> Option<i32> {
    // Skip leading white space.
    let mut c = loop {
        match src.getc() {
            None => return None, // EOF before any conversion
            Some(c) if is_c_space(c) => continue,
            Some(c) => break c,
        }
    };

    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        match src.getc() {
            None => return None,
            Some(n) => c = n,
        }
    }

    if !c.is_ascii_digit() {
        src.unget(); // glibc pushes the offending character back
        return None;
    }

    // glibc parses with strtol semantics, so out of range values saturate at
    // LONG_MAX / LONG_MIN and are then truncated to `int`.
    let mut acc: i64 = 0;
    loop {
        let d = (c - b'0') as i64;
        acc = acc.saturating_mul(10).saturating_add(d);
        match src.getc() {
            None => break,
            Some(n) => {
                if n.is_ascii_digit() {
                    c = n;
                } else {
                    src.unget();
                    break;
                }
            }
        }
    }
    let value = if negative { acc.wrapping_neg() } else { acc };
    Some(value as i32)
}

/// A cursor over an in-memory file, used for `scene_load`.
pub struct FileIn {
    data: Vec<u8>,
    pos: usize,
}

impl FileIn {
    pub fn new(data: Vec<u8>) -> FileIn {
        FileIn { data, pos: 0 }
    }

    fn getc(&mut self) -> Option<u8> {
        if self.pos >= self.data.len() {
            return None;
        }
        let c = self.data[self.pos];
        self.pos += 1;
        Some(c)
    }

    fn unget(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }

    pub fn fgets(&mut self, size: usize) -> Option<Vec<u8>> {
        if size == 0 {
            return None;
        }
        let mut out: Vec<u8> = Vec::new();
        while out.len() < size - 1 {
            match self.getc() {
                None => break,
                Some(c) => {
                    out.push(c);
                    if c == b'\n' {
                        break;
                    }
                }
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    /// `fscanf(file, "%d\n", &x)`. The trailing white space directive only
    /// consumes white space, which the next conversion would skip anyway.
    pub fn scan_int(&mut self) -> Option<i32> {
        let r = scan_int_generic(self);
        if r.is_some() {
            // "%d\n": the literal white space directive skips any run of white
            // space, pushing back the first non-white-space character.
            loop {
                match self.getc() {
                    None => break,
                    Some(c) if is_c_space(c) => continue,
                    Some(_) => {
                        self.unget();
                        break;
                    }
                }
            }
        }
        r
    }
}

pub fn cprintf(out: &mut Out, fmt: &[u8], args: &[Arg]) {
    let mut ai = 0usize;
    let mut i = 0usize;
    while i < fmt.len() {
        if fmt[i] == b'%' && i + 1 < fmt.len() {
            let spec = fmt[i + 1];
            match spec {
                b's' | b'd' | b'p' => {
                    match &args[ai] {
                        Arg::S(s) => out.put(s),
                        Arg::D(v) => out.put(format!("{}", v).as_bytes()),
                        Arg::P(p) => {
                            if *p == 0 {
                                out.put(b"(nil)");
                            } else {
                                out.put(format!("0x{:x}", p).as_bytes());
                            }
                        }
                    }
                    ai += 1;
                    i += 2;
                }
                b'%' => {
                    out.put(b"%");
                    i += 2;
                }
                _ => {
                    out.put(&fmt[i..i + 1]);
                    i += 1;
                }
            }
        } else {
            out.put(&fmt[i..i + 1]);
            i += 1;
        }
    }
}

/// `fprintf(stderr, ...)` -- stderr is unbuffered in C.
pub fn ceprintf(fmt: &[u8], args: &[Arg]) {
    let mut tmp = Out {
        buf: Vec::new(),
        line_buffered: false,
    };
    // Render into the scratch buffer without ever reaching the 4096 byte
    // flush threshold, then write it out in one shot.
    cprintf(&mut tmp, fmt, args);
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    let _ = lock.write_all(&tmp.buf);
    let _ = lock.flush();
}

/// The C code applies `s[strcspn(s, "\n")] = 0` to `fgets` results. The
/// resulting C string ends at the first newline or the first NUL byte.
pub fn trim_at_newline(buf: &[u8]) -> &[u8] {
    let end = buf
        .iter()
        .position(|&c| c == b'\n' || c == 0)
        .unwrap_or(buf.len());
    &buf[..end]
}
