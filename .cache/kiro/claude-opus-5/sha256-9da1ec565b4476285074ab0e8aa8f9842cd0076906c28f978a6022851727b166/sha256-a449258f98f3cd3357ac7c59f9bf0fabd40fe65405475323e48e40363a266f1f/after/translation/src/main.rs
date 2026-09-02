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

use std::io::{Read, Write};

// ---------------------------------------------------------------------------
// stdin: byte-at-a-time reader with one byte of push-back, mirroring the way
// C's `scanf` consumes exactly as much of the stream as it needs (and, unlike
// `fgets`, freely skips over newlines while looking for a conversion).
// ---------------------------------------------------------------------------

struct CStdin {
    inner: std::io::Stdin,
    pushed_back: Option<u8>,
}

impl CStdin {
    fn new() -> Self {
        CStdin {
            inner: std::io::stdin(),
            pushed_back: None,
        }
    }

    /// Equivalent of `getchar()`: returns `None` at EOF (or on read error,
    /// which C also surfaces as a failed conversion).
    fn getc(&mut self) -> Option<u8> {
        if let Some(b) = self.pushed_back.take() {
            return Some(b);
        }
        let mut buf = [0u8; 1];
        match self.inner.read(&mut buf) {
            Ok(1) => Some(buf[0]),
            _ => None,
        }
    }

    /// Equivalent of `ungetc()`: one byte of push-back is all `scanf` needs.
    fn ungetc(&mut self, b: u8) {
        self.pushed_back = Some(b);
    }
}

/// True for the characters C's `isspace()` accepts in the "C" locale; these are
/// the ones a `%d` directive skips before starting its conversion.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// `scanf("%d", out)` for a single directive.
///
/// Returns the number of successful assignments (1), 0 on a matching failure,
/// or -1 (C's `EOF`) when input ends before any non-whitespace is seen. On
/// anything other than 1 the destination is left untouched, exactly as in C.
///
/// Accumulation is done in `i64` with saturation and then narrowed to `i32`,
/// which reproduces glibc's behaviour of running the digits through `strtol`
/// (saturating at `LONG_MAX`/`LONG_MIN`) and storing the result into an `int`.
fn scanf_d(stdin: &mut CStdin, out: &mut i32) -> i32 {
    // Skip leading whitespace, including across newlines.
    let mut c = loop {
        match stdin.getc() {
            None => return -1, // EOF before any input item
            Some(b) if is_c_space(b) => continue,
            Some(b) => break b,
        }
    };

    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        match stdin.getc() {
            // A sign with nothing after it is a matching failure, not EOF:
            // glibc has already consumed an input character by this point.
            None => return 0,
            Some(b) => c = b,
        }
    }

    if !c.is_ascii_digit() {
        stdin.ungetc(c);
        return 0; // matching failure
    }

    let mut acc: i64 = 0;
    loop {
        let digit = i64::from(c - b'0');
        acc = acc
            .saturating_mul(10)
            .saturating_add(if negative { -digit } else { digit });

        match stdin.getc() {
            None => break,
            Some(b) if b.is_ascii_digit() => c = b,
            Some(b) => {
                stdin.ungetc(b);
                break;
            }
        }
    }

    *out = acc as i32;
    1
}

// ---------------------------------------------------------------------------
// stdout helpers, matching the C printf formats exactly.
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        let _ = write!(out, "{}\n", line);
    }
}

fn print_int_line(int_number: i32) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", int_number);
}

// ---------------------------------------------------------------------------
// Model of the stack region carved out by `alloca`.
//
// `bad()` requests only 10 bytes but then stores ten 4-byte ints through the
// pointer, running 30 bytes past the end of the allocation and scribbling over
// adjacent stack storage. That is the original defect and it must be preserved,
// so the region is modelled as a byte-addressed scratch area: `alloc_len`
// records how many bytes were actually requested, while the backing store is
// large enough that the out-of-bounds stores land in neighbouring frame bytes
// rather than triggering a Rust panic -- the same thing that happens in C.
// ---------------------------------------------------------------------------

const FRAME_BYTES: usize = 64;

struct StackFrame {
    bytes: [u8; FRAME_BYTES],
    /// Bytes actually requested from `alloca`; recorded to keep the original
    /// under-allocation visible even though the stores are not bounded by it.
    #[allow(dead_code)]
    alloc_len: usize,
}

impl StackFrame {
    /// `alloca(size)`: hand back a region of `size` bytes out of the frame.
    fn alloca(size: usize) -> Self {
        StackFrame {
            bytes: [0u8; FRAME_BYTES],
            alloc_len: size,
        }
    }

    /// `data[index] = value` for an `int *`, in native (little-endian x86_64)
    /// byte order. Stores past `alloc_len` are the out-of-bounds writes.
    fn store_int(&mut self, index: usize, value: i32) {
        let offset = index * std::mem::size_of::<i32>();
        let end = offset + std::mem::size_of::<i32>();
        if end <= self.bytes.len() {
            self.bytes[offset..end].copy_from_slice(&value.to_ne_bytes());
        }
    }

    /// `data[index]` for an `int *`.
    fn load_int(&self, index: usize) -> i32 {
        let offset = index * std::mem::size_of::<i32>();
        let end = offset + std::mem::size_of::<i32>();
        if end <= self.bytes.len() {
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&self.bytes[offset..end]);
            i32::from_ne_bytes(buf)
        } else {
            0
        }
    }
}

// ---------------------------------------------------------------------------

fn bad() {
    let data: &mut StackFrame;
    // alloca(10): only 10 bytes, not 10 * sizeof(int) -- the original bug.
    let mut region = StackFrame::alloca(10);
    data = &mut region;
    {
        let source = [0i32; 10];
        let mut i: usize;
        i = 0;
        while i < 10 {
            data.store_int(i, source[i]);
            i += 1;
        }
        print_int_line(data.load_int(0));
    }
}

fn good() {
    let data: &mut StackFrame;
    let mut region = StackFrame::alloca(10 * std::mem::size_of::<i32>());
    data = &mut region;
    {
        let source = [0i32; 10];
        let mut i: usize;
        i = 0;
        while i < 10 {
            data.store_int(i, source[i]);
            i += 1;
        }
        print_int_line(data.load_int(0));
    }
}

// ---------------------------------------------------------------------------
// SIGPIPE fidelity.
//
// A C program does nothing to SIGPIPE, so it runs with whatever disposition it
// inherited across `exec`: `SIG_DFL` when launched from a shell (a write to a
// pipe with no reader kills it with signal 13), or `SIG_IGN` when the parent had
// ignored it (the write fails with EPIPE and the program still exits 0, because
// nothing here checks printf's return value).
//
// The Rust standard library instead forces SIGPIPE to `SIG_IGN` before `main`
// runs, which loses that inherited setting and makes the exit status differ from
// the C program's whenever the parent used `SIG_DFL`.
//
// To behave exactly like the C program in either environment, an `.init_array`
// constructor -- which the dynamic loader runs before `main`, and therefore
// before the standard library's own initialisation -- records the inherited
// disposition, and `main` puts it back as its first action.
// ---------------------------------------------------------------------------
#[cfg(unix)]
mod sigpipe {
    use std::sync::atomic::{AtomicUsize, Ordering};

    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    /// `signal()` failure value, also used as "nothing captured" because in that
    /// case there is nothing to restore either.
    const SIG_ERR: usize = usize::MAX;

    extern "C" {
        /// Returns the previous handler, or `SIG_ERR` on failure.
        fn signal(signum: i32, handler: usize) -> usize;
    }

    static INHERITED: AtomicUsize = AtomicUsize::new(SIG_ERR);

    /// Runs before `main`, and so before the standard library replaces the
    /// inherited disposition with `SIG_IGN`. Reads the current handler by
    /// swapping in `SIG_DFL` and immediately putting the old value back.
    unsafe extern "C" fn capture_inherited() {
        let previous = signal(SIGPIPE, SIG_DFL);
        if previous != SIG_ERR {
            signal(SIGPIPE, previous);
        }
        INHERITED.store(previous, Ordering::SeqCst);
    }

    #[used]
    #[cfg_attr(target_os = "linux", link_section = ".init_array")]
    #[cfg_attr(target_vendor = "apple", link_section = "__DATA,__mod_init_func")]
    static CAPTURE_INHERITED: unsafe extern "C" fn() = capture_inherited;

    /// Undo the standard library's `SIG_IGN`, restoring what was inherited.
    pub fn restore_inherited() {
        let previous = INHERITED.load(Ordering::SeqCst);
        if previous != SIG_ERR {
            unsafe {
                signal(SIGPIPE, previous);
            }
        }
    }
}

#[cfg(not(unix))]
mod sigpipe {
    pub fn restore_inherited() {}
}

fn main() {
    sigpipe::restore_inherited();

    let mut stdin = CStdin::new();

    let mut x: i32 = 0;
    scanf_d(&mut stdin, &mut x);

    if x != 0 {
        good();
    } else {
        bad();
    }

    let _ = std::io::stdout().flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive `scanf_d` over a fixed byte slice instead of the real stdin.
    fn scan(input: &[u8]) -> (i32, i32) {
        struct Slice<'a> {
            data: &'a [u8],
            pos: usize,
        }
        impl<'a> Read for Slice<'a> {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                if self.pos >= self.data.len() || buf.is_empty() {
                    return Ok(0);
                }
                buf[0] = self.data[self.pos];
                self.pos += 1;
                Ok(1)
            }
        }

        // Re-implement the reader over a slice, mirroring CStdin exactly.
        let mut src = Slice { data: input, pos: 0 };
        let mut pushed: Option<u8> = None;
        let mut getc = move || -> Option<u8> {
            if let Some(b) = pushed.take() {
                return Some(b);
            }
            let mut buf = [0u8; 1];
            match src.read(&mut buf) {
                Ok(1) => Some(buf[0]),
                _ => None,
            }
        };

        // Inline equivalent of scanf_d driven by the closure above.
        let mut out: i32 = 0;
        let ret = (|| -> i32 {
            let mut c = loop {
                match getc() {
                    None => return -1,
                    Some(b) if is_c_space(b) => continue,
                    Some(b) => break b,
                }
            };
            let mut negative = false;
            if c == b'+' || c == b'-' {
                negative = c == b'-';
                match getc() {
                    None => return 0,
                    Some(b) => c = b,
                }
            }
            if !c.is_ascii_digit() {
                return 0;
            }
            let mut acc: i64 = 0;
            loop {
                let digit = i64::from(c - b'0');
                acc = acc
                    .saturating_mul(10)
                    .saturating_add(if negative { -digit } else { digit });
                match getc() {
                    Some(b) if b.is_ascii_digit() => c = b,
                    _ => break,
                }
            }
            out = acc as i32;
            1
        })();

        (ret, out)
    }

    /// Expected values captured from glibc's `scanf("%d", &x)` on x86_64.
    #[test]
    fn matches_glibc_scanf() {
        assert_eq!(scan(b""), (-1, 0));
        assert_eq!(scan(b"0"), (1, 0));
        assert_eq!(scan(b"1"), (1, 1));
        assert_eq!(scan(b"-1"), (1, -1));
        assert_eq!(scan(b"   \n 5"), (1, 5));
        assert_eq!(scan(b"abc"), (0, 0));
        assert_eq!(scan(b"+7"), (1, 7));
        assert_eq!(scan(b"-"), (0, 0));
        assert_eq!(scan(b"0x10"), (1, 0));
        assert_eq!(scan(b"007"), (1, 7));
        assert_eq!(scan(b"2147483647"), (1, 2147483647));
        assert_eq!(scan(b"2147483648"), (1, -2147483648));
        assert_eq!(scan(b"4294967296"), (1, 0));
        assert_eq!(scan(b"99999999999999999999"), (1, -1));
        assert_eq!(scan(b"-99999999999999999999"), (1, 0));
        assert_eq!(scan(b"1abc"), (1, 1));
        assert_eq!(scan(b"-0"), (1, 0));
        assert_eq!(scan(b"\n\n"), (-1, 0));
    }

    /// Both paths read back a zeroed `data[0]`, so both print 0.
    #[test]
    fn both_paths_yield_zero() {
        for len in [10usize, 10 * std::mem::size_of::<i32>()] {
            let mut region = StackFrame::alloca(len);
            let source = [0i32; 10];
            for i in 0..10 {
                region.store_int(i, source[i]);
            }
            assert_eq!(region.load_int(0), 0);
        }
    }
}
