// Translation of c_src/src/main.c to Rust (shared by the bin and the cdylib).
//
// Original C:
//   static void print_hex(unsigned char *p, int len) {
//       for (int i = 0; i < len; i++) printf("%02x", p[i]);
//       printf("\n");
//   }
//   void driver(int x) {
//       char raw[sizeof(x)];
//       memcpy(raw, &x, sizeof(x));
//       print_hex((unsigned char *)raw, sizeof(raw));
//   }
//   int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
//
// Behaviour reproduced from the C program (glibc / x86-64 Linux):
//
//  * `scanf("%d", &x)` skips leading whitespace (`isspace` in the "C" locale,
//    newlines included), then accepts an optional `+`/`-`, then decimal digits.
//    The first byte that cannot continue the conversion is pushed back and is
//    *not* consumed.
//  * On matching failure (no digit) or input failure (EOF/read error) the
//    conversion does not store anything, so `x` keeps its initial value `0`.
//    The C code ignores `scanf`'s return value, so nothing else changes.
//  * glibc converts the digits with `strtol` into a `long`, saturating at
//    LONG_MAX / LONG_MIN when `strtol` reports ERANGE, and then assigns
//    (truncates) that `long` into the `int` object.
//  * The 4 bytes of the `int` are copied verbatim (native byte order) and
//    printed as lowercase two-digit hex followed by a newline.
//  * stdin is read exactly the way glibc's `FILE` does, because that is
//    observable on the descriptor itself:
//      - one process-wide buffer (like the global `stdin`), so consecutive
//        conversions continue where the previous one stopped, and EOF is
//        "sticky" (C99), while a read *error* is not;
//      - the buffer size is `st_blksize` when `0 < st_blksize < BUFSIZ`, else
//        `BUFSIZ` (8192) — glibc's `_IO_file_doallocate`;
//      - one `read(2)` of that size per refill;
//      - when the *program* exits, glibc's `_IO_cleanup` seeks the descriptor
//        back over the bytes that were buffered but never consumed
//        (`_IO_new_file_sync`), ignoring ESPIPE on non-seekable descriptors.
//        `run(true)` does that; the `main` exported from the cdylib uses
//        `run(false)`, because returning from `main` inside a dlopen'd library
//        does not run libc's exit-time cleanup.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::mem::ManuallyDrop;
use std::os::unix::fs::MetadataExt;
use std::os::unix::io::FromRawFd;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// glibc's `BUFSIZ`.
const IO_BUFSIZ: usize = 8192;

/// fd 0 as a `File` that is never closed.
fn fd0() -> ManuallyDrop<File> {
    // SAFETY: fd 0 is owned by the process; `ManuallyDrop` keeps it open.
    unsafe { ManuallyDrop::new(File::from_raw_fd(0)) }
}

/// The moral equivalent of glibc's `stdin` `FILE`: a single process-wide,
/// block-buffered reader over fd 0 with one-byte-at-a-time inspection.
struct StdinFile {
    buf: Vec<u8>,
    /// `_IO_read_ptr`
    pos: usize,
    /// `_IO_read_end`
    end: usize,
    /// `_IO_EOF_SEEN` — C99 requires EOF to be sticky.
    eof_seen: bool,
}

impl StdinFile {
    const fn new() -> StdinFile {
        StdinFile {
            buf: Vec::new(),
            pos: 0,
            end: 0,
            eof_seen: false,
        }
    }

    /// `_IO_file_doallocate`: buffer of `st_blksize` bytes when that is in
    /// `(0, BUFSIZ)`, otherwise `BUFSIZ`.
    fn ensure_buf(&mut self) {
        if self.buf.is_empty() {
            let size = match fd0().metadata() {
                Ok(m) => {
                    let b = m.blksize() as usize;
                    if b > 0 && b < IO_BUFSIZ {
                        b
                    } else {
                        IO_BUFSIZ
                    }
                }
                Err(_) => IO_BUFSIZ,
            };
            self.buf = vec![0u8; size];
        }
    }

    /// Look at the next byte without consuming it (`_IO_peekc`).
    fn peek(&mut self) -> Option<u8> {
        if self.pos < self.end {
            return Some(self.buf[self.pos]);
        }
        // `_IO_new_file_underflow`: EOF is sticky, a read error is not.
        if self.eof_seen {
            return None;
        }
        self.ensure_buf();
        let f = fd0();
        match (&*f).read(&mut self.buf) {
            Ok(0) => {
                self.eof_seen = true;
                None
            }
            Ok(n) => {
                self.pos = 0;
                self.end = n;
                Some(self.buf[0])
            }
            // glibc does not restart an interrupted `read`: it sets
            // `_IO_ERR_SEEN` and reports EOF for this attempt.
            Err(_) => None,
        }
    }

    /// Consume the byte returned by the last `peek`.
    fn bump(&mut self) {
        self.pos += 1;
    }

    /// `_IO_new_file_sync`, as reached from `_IO_cleanup` at process exit: put
    /// the descriptor back at the first byte the program did not consume.
    fn sync(&mut self) {
        let delta = self.end - self.pos;
        if delta != 0 {
            let f = fd0();
            if (&*f).seek(SeekFrom::Current(-(delta as i64))).is_ok() {
                self.end = self.pos;
            }
            // ESPIPE (and any other error) is ignored, exactly like glibc.
        }
    }
}

fn stdin_file() -> MutexGuard<'static, StdinFile> {
    static S: OnceLock<Mutex<StdinFile>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(StdinFile::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// C `isspace` for the "C" locale.
fn c_isspace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Emulates a single `scanf("%d", &out)` conversion.
///
/// Returns `Some(value)` on a successful conversion, `None` on input failure
/// (EOF/read error before any digit) or matching failure (no digits), in which
/// case the caller's variable is left unmodified — exactly as C does.
fn scanf_int(r: &mut StdinFile) -> Option<i32> {
    // Skip leading whitespace (crosses newlines, like scanf).
    loop {
        match r.peek() {
            Some(b) if c_isspace(b) => r.bump(),
            Some(_) => break,
            None => return None, // input failure (EOF / read error)
        }
    }

    let mut negative = false;
    match r.peek() {
        Some(b'-') => {
            negative = true;
            r.bump();
        }
        Some(b'+') => {
            r.bump();
        }
        _ => {}
    }

    // Accumulate the magnitude; stop growing once it can no longer matter.
    const CAP: u128 = 1u128 << 70;
    let mut magnitude: u128 = 0;
    let mut digits = 0usize;
    while let Some(b) = r.peek() {
        if b.is_ascii_digit() {
            r.bump();
            digits += 1;
            if magnitude < CAP {
                magnitude = magnitude * 10 + u128::from(b - b'0');
            }
        } else {
            break;
        }
    }

    if digits == 0 {
        return None; // matching failure
    }

    // strtol-style saturation into `long` (64-bit), then truncation to `int`.
    let as_long: i64 = if negative {
        if magnitude >= (1u128 << 63) {
            i64::MIN
        } else {
            -(magnitude as i64)
        }
    } else if magnitude > i64::MAX as u128 {
        i64::MAX
    } else {
        magnitude as i64
    };

    Some(as_long as i32)
}

/// `static void print_hex(unsigned char *p, int len)`
fn print_hex(out: &mut Vec<u8>, p: &[u8], len: usize) {
    for i in 0..len {
        let _ = write!(out, "{:02x}", p[i]);
    }
    let _ = write!(out, "\n");
}

fn write_stdout(bytes: &[u8]) {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(bytes);
    let _ = lock.flush();
}

/// `void driver(int x)`
pub fn driver(x: i32) {
    // char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));
    let raw = x.to_ne_bytes();
    let mut out: Vec<u8> = Vec::new();
    print_hex(&mut out, &raw, raw.len());
    write_stdout(&out);
}

/// `int main()`
///
/// `sync_at_exit` mirrors libc's exit-time `_IO_cleanup`: the program (bin)
/// returns from `main` into `__libc_start_main`, which calls `exit`, which seeks
/// stdin back over the unconsumed buffered bytes.  A `main` invoked through
/// `dlsym` returns without any of that happening.
pub fn program_main(sync_at_exit: bool) -> i32 {
    let mut x: i32 = 0;

    {
        let mut s = stdin_file();
        if let Some(v) = scanf_int(&mut s) {
            x = v;
        }
    }

    driver(x);

    if sync_at_exit {
        stdin_file().sync();
    }

    0
}
