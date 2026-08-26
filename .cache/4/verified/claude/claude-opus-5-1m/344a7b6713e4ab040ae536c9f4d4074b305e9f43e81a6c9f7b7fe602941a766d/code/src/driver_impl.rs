// Rust translation of c_src/src/main.c
//
// Original copyright notice from the C source:
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
// This module holds the whole translation.  It is compiled into both the
// `driver` binary (src/main.rs) and the `libdriver.so` shared object
// (src/lib.rs), so the binary and the exported C ABI symbols always share one
// implementation.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::mem::ManuallyDrop;
use std::os::raw::c_int;
use std::os::unix::fs::MetadataExt;
use std::os::unix::io::FromRawFd;
use std::sync::{Condvar, Mutex, MutexGuard};
use std::thread::ThreadId;

// ---------------------------------------------------------------------------
// The `stdin` stream
//
// `scanf` reads through glibc's `stdin` `FILE`, whose state is observable from
// outside the process, so the translation reproduces that stream instead of
// using `io::stdin()`'s private reader:
//
//   * the buffer is process-global and sized the way `_IO_file_doallocate`
//     sizes it (`st_blksize` of fd 0 when `0 < st_blksize < BUFSIZ`, else
//     `BUFSIZ`), so a conversion consumes exactly as many bytes from the
//     descriptor as the C program does;
//   * `_IO_new_file_underflow` starts with `if (_flags & _IO_EOF_SEEN) return
//     EOF;` — C99 requires the end-of-file indicator to be sticky — so once a
//     conversion has seen EOF no later one issues another `read`.  The error
//     indicator, by contrast, is *not* consulted, so a failed `read` is retried
//     by the next conversion;
//   * the byte a conversion stops on is handed back to the stream (`ungetc`,
//     which also clears the EOF indicator), so the next conversion — the next
//     call of the exported `main` — sees it again ("12x34" leaves `x`, "12-34"
//     leaves `-`);
//   * `scanf` holds the stream lock (`flockfile`) for the whole conversion, and
//     that lock is recursive for the owning thread, so concurrent callers can
//     never split one number between them while a signal handler that re-enters
//     the conversion still makes progress;
//   * at process exit glibc's `_IO_cleanup` seeks a seekable descriptor back
//     over whatever it read ahead but did not consume, so the next reader of the
//     descriptor (`{ ./driver; cat; } < file`) sees the same bytes it sees with
//     the C build.  That is what the `atexit` hook below does.
// ---------------------------------------------------------------------------

/// glibc's `BUFSIZ`.
const BUFSIZ: usize = 8192;

struct CStdinState {
    /// The stream buffer, allocated on first use like glibc's.
    buf: Vec<u8>,
    /// `_IO_read_ptr`: how much of `buf[..len]` has been handed out.
    pos: usize,
    /// `_IO_read_end`: how much of `buf` holds data from the descriptor.
    len: usize,
    /// The byte handed back to the stream by `ungetc`, i.e. `buf[pos - 1]`.
    pushback: Option<u8>,
    /// `_IO_EOF_SEEN`: sticky, and checked before every refill.
    eof_seen: bool,
    /// Whether the exit-time "seek back over the read-ahead" hook is installed.
    atexit_installed: bool,
}

static C_STDIN: Mutex<CStdinState> = Mutex::new(CStdinState {
    buf: Vec::new(),
    pos: 0,
    len: 0,
    pushback: None,
    eof_seen: false,
    atexit_installed: false,
});

fn lock_c_stdin() -> MutexGuard<'static, CStdinState> {
    // A poisoned lock would mean a panic while the stream was borrowed; the data
    // is still consistent, and C has no such concept, so keep going.
    C_STDIN.lock().unwrap_or_else(|e| e.into_inner())
}

// --- `flockfile(stdin)`: recursive, held for a whole conversion -------------

struct StreamLock {
    owner: Mutex<Option<(ThreadId, u32)>>,
    idle: Condvar,
}

static STREAM_LOCK: StreamLock = StreamLock {
    owner: Mutex::new(None),
    idle: Condvar::new(),
};

/// Released when dropped, like `funlockfile`.
pub struct StreamGuard;

/// `flockfile(stdin)`.  Recursive for the owning thread, so re-entering the
/// conversion (from a signal handler, as glibc's recursive `_IO_lock_t` allows)
/// makes progress instead of dead-locking.
fn lock_stream() -> StreamGuard {
    let me = std::thread::current().id();
    let mut owner = STREAM_LOCK
        .owner
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    loop {
        match *owner {
            None => {
                *owner = Some((me, 1));
                return StreamGuard;
            }
            Some((holder, depth)) if holder == me => {
                *owner = Some((holder, depth + 1));
                return StreamGuard;
            }
            Some(_) => {
                owner = STREAM_LOCK
                    .idle
                    .wait(owner)
                    .unwrap_or_else(|e| e.into_inner());
            }
        }
    }
}

impl Drop for StreamGuard {
    fn drop(&mut self) {
        let mut owner = STREAM_LOCK
            .owner
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        match *owner {
            Some((holder, depth)) if depth > 1 => *owner = Some((holder, depth - 1)),
            _ => {
                *owner = None;
                STREAM_LOCK.idle.notify_one();
            }
        }
    }
}

/// fd 0 as a `File` that must not be closed when it goes out of scope.
fn fd0() -> ManuallyDrop<File> {
    // SAFETY: fd 0 stays owned by the process; `ManuallyDrop` keeps the `File`
    // from closing it, and only `read`/`fstat`/`lseek` are used through it.
    unsafe { ManuallyDrop::new(File::from_raw_fd(0)) }
}

/// `_IO_file_doallocate`'s buffer size: `st_blksize` when it is smaller than
/// `BUFSIZ`, `BUFSIZ` otherwise (and when `fstat` fails).
fn stream_buffer_size() -> usize {
    let f = fd0();
    match (&*f).metadata() {
        Ok(md) => {
            let bs = md.blksize() as usize;
            if bs > 0 && bs < BUFSIZ {
                bs
            } else {
                BUFSIZ
            }
        }
        Err(_) => BUFSIZ,
    }
}

/// Allocate the stream buffer without ever aborting: when the allocation fails
/// glibc falls back to the one-byte `_shortbuf`, i.e. an unbuffered stream.
fn allocate_buffer(size: usize) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();
    let want = if v.try_reserve_exact(size).is_ok() {
        size
    } else if v.try_reserve_exact(1).is_ok() {
        1
    } else {
        0
    };
    v.resize(want, 0); // within the reserved capacity, so this cannot reallocate
    v
}

/// `_IO_cleanup`'s effect on a read stream: give the descriptor back the bytes
/// that were read ahead but never consumed.  `lseek` failing (a pipe, a
/// terminal) is ignored, exactly as glibc ignores it.
extern "C" fn c_stdin_atexit() {
    // glibc's `_IO_cleanup` runs without waiting for anybody, so if another
    // thread is in the middle of a conversion the rewind is skipped rather than
    // blocking process exit.
    if let Ok(st) = C_STDIN.try_lock() {
        let unread = (st.len - st.pos) + usize::from(st.pushback.is_some());
        if unread > 0 {
            let f = fd0();
            let _ = (&*f).seek(SeekFrom::Current(-(unread as i64)));
        }
    }
}

fn install_atexit_hook(st: &mut CStdinState) {
    if st.atexit_installed {
        return;
    }
    st.atexit_installed = true;
    extern "C" {
        fn atexit(cb: extern "C" fn()) -> c_int;
    }
    // SAFETY: registering a plain `extern "C" fn` with no arguments; this is the
    // documented C interface and the callback lives as long as the code does.
    unsafe {
        atexit(c_stdin_atexit);
    }
}

/// The `stdin` stream, seen as a byte source.
pub struct CStdin;

impl Read for CStdin {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        {
            let mut st = lock_c_stdin();
            // Bytes still in the stream buffer.
            if st.pos < st.len {
                let n = (st.len - st.pos).min(out.len());
                out[..n].copy_from_slice(&st.buf[st.pos..st.pos + n]);
                st.pos += n;
                return Ok(n);
            }
            // `_IO_new_file_underflow`'s first statement: EOF is sticky.
            if st.eof_seen {
                return Ok(0);
            }
        }

        // Underflow: exactly one `read` of the whole buffer.  The buffer is
        // detached from the shared state first so that a blocking `read` never
        // holds the state lock — the exit hook and a re-entrant caller must not
        // be blocked by it.
        let mut buf = {
            let mut st = lock_c_stdin();
            if st.buf.is_empty() {
                st.buf = allocate_buffer(stream_buffer_size());
                install_atexit_hook(&mut st);
            }
            st.pos = 0;
            st.len = 0;
            std::mem::take(&mut st.buf)
        };
        let res = {
            let f = fd0();
            (&*f).read(&mut buf)
        };
        let mut st = lock_c_stdin();
        st.buf = buf;
        st.pos = 0;
        match res {
            Ok(0) => {
                st.len = 0;
                st.eof_seen = true;
                Ok(0)
            }
            Ok(n) => {
                st.len = n;
                let k = n.min(out.len());
                out[..k].copy_from_slice(&st.buf[..k]);
                st.pos = k;
                Ok(k)
            }
            // A failed `read` — `EINTR` included — sets `_IO_ERR_SEEN` and
            // reports end of input for this conversion; glibc never retries it
            // inside the conversion, and never consults the flag afterwards.
            Err(e) => {
                st.len = 0;
                Err(e)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The `stdout` stream
// ---------------------------------------------------------------------------

/// Writing to `stdout` the way C's `printf`/`putchar` do: one locked append to
/// the process-wide stdout buffer per call, so concurrent callers interleave at
/// the same granularity the C code allows.
pub struct CStdout;

impl Write for CStdout {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        io::stdout().write(buf)
    }
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        io::stdout().write_all(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        io::stdout().flush()
    }
}

/// A byte-at-a-time reader with one byte of lookahead, mirroring how C's
/// `scanf` consumes only the characters it needs.
pub struct Scanner<R: Read> {
    inner: R,
    peeked: Option<u8>,
    eof: bool,
}

impl<R: Read> Scanner<R> {
    pub fn new(inner: R) -> Self {
        Scanner {
            inner,
            peeked: None,
            eof: false,
        }
    }

    /// Like [`Scanner::new`], but seeded with the byte an earlier conversion
    /// handed back to the stream with `ungetc`.
    fn new_stdin(inner: R) -> Self {
        Scanner {
            inner,
            peeked: lock_c_stdin().pushback.take(),
            // This flag is only the per-conversion `c == EOF` short-circuit of
            // glibc's `inchar()`; the *stream's* sticky EOF indicator lives in
            // `C_STDIN.eof_seen` and is checked by `CStdin::read`.
            eof: false,
        }
    }

    /// Hand the lookahead byte back to the stream (C's `ungetc`) so the next
    /// conversion — and the exit-time seek-back — account for it.  `ungetc` also
    /// clears the stream's end-of-file indicator.
    fn save_stdin_state(&self) {
        let mut st = lock_c_stdin();
        st.pushback = self.peeked;
        if self.peeked.is_some() {
            st.eof_seen = false;
        }
    }

    /// Look at the next byte without consuming it. `None` means EOF (or a
    /// read error, which C's stdio also reports as a stream failure).
    fn peek(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        match self.inner.read(&mut buf) {
            Ok(0) => {
                self.eof = true;
                None
            }
            Ok(_) => {
                self.peeked = Some(buf[0]);
                Some(buf[0])
            }
            // Every failed `read` — `EINTR` included — makes glibc's
            // `_IO_new_file_underflow` set the stream's error indicator and
            // report end of input; it never retries.
            Err(_) => {
                self.eof = true;
                None
            }
        }
    }

    /// Consume the next byte. Every call site peeks first, so the byte is
    /// already cached; peeking again keeps this correct regardless.
    fn bump(&mut self) {
        if self.peeked.is_none() {
            let _ = self.peek();
        }
        self.peeked = None;
    }
}

/// C `isspace` for the default "C" locale (note: includes vertical tab, which
/// Rust's `u8::is_ascii_whitespace` omits).
fn c_isspace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | b'\x0c' | b'\r')
}

/// Emulates `scanf("%d", &x)`: leading whitespace (including newlines) is
/// skipped, then an optional sign followed by decimal digits is consumed.
/// Returns `None` on input failure or matching failure, in which case the
/// caller's variable is left untouched (as in C).
///
/// On overflow glibc's `%d` saturates at `LONG_MAX`/`LONG_MIN` (its internal
/// `strtol` behavior) and then truncates the result to `int`; that is
/// reproduced here.
pub fn scan_i32<R: Read>(sc: &mut Scanner<R>) -> Option<i32> {
    // Skip leading whitespace; EOF here is an input failure.
    loop {
        match sc.peek() {
            Some(b) if c_isspace(b) => sc.bump(),
            Some(_) => break,
            None => return None,
        }
    }

    let mut negative = false;
    match sc.peek() {
        Some(b'+') => sc.bump(),
        Some(b'-') => {
            negative = true;
            sc.bump();
        }
        _ => {}
    }

    let mut saw_digit = false;
    let mut acc: i64 = 0;
    let mut overflowed = false;
    while let Some(b) = sc.peek() {
        if !b.is_ascii_digit() {
            break;
        }
        sc.bump();
        saw_digit = true;
        let digit = i64::from(b - b'0');
        if !overflowed {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => overflowed = true,
            }
        }
    }

    if !saw_digit {
        // Matching failure: no digits were converted.
        return None;
    }

    let wide: i64 = if overflowed {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -acc
    } else {
        acc
    };

    // Assignment to an `int *` truncates.
    Some(wide as i32)
}

/// `static void print_hex(unsigned char *p, int len)`
///
/// One `write_all` per C `printf("%02x", …)` call and one for the `putchar`, so
/// the writer sees exactly the sequence of appends the C code performs (this is
/// what decides how concurrent callers may interleave).
pub fn print_hex(out: &mut impl Write, p: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for &b in p {
        let pair = [HEX[usize::from(b >> 4)], HEX[usize::from(b & 0x0f)]];
        let _ = out.write_all(&pair);
    }
    let _ = out.write_all(b"\n");
}

/// `void driver(int x)` — the C code reinterprets the bytes of the `int` in
/// host byte order.
pub fn driver(out: &mut impl Write, x: i32) {
    print_hex(out, &x.to_ne_bytes());
}

/// `void driver(int x)` writing to the process's real stdout, for the exported
/// C ABI symbol.
pub fn driver_stdout(x: i32) {
    driver(&mut CStdout, x);
}

/// `int main(void)`
pub fn run_main() -> i32 {
    // The stream state (buffer, `ungetc` pushback, EOF indicator) is
    // process-global, exactly like glibc's `stdin`, so a second conversion
    // continues where this one stops instead of losing the read-ahead with a
    // local reader.  `scanf` holds the stream lock for the whole conversion, so
    // two threads can never take alternate digits of the same number.
    let _stream = lock_stream();
    let mut sc = Scanner::new_stdin(CStdin);

    let mut x: i32 = 0;
    if let Some(v) = scan_i32(&mut sc) {
        x = v;
    }
    sc.save_stdin_state();
    driver(&mut CStdout, x);
    0
}
