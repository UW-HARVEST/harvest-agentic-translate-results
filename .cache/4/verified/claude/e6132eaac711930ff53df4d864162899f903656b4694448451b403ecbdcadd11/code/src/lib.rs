// Rust translation of c_src/src/main.c
//
// Original C (the ONLY translation unit in c_src/):
//
//     #include <stdio.h>
//     #include <stdlib.h>
//
//     int main() {
//         int x = 1, y = 1;
//         scanf("%d %d", &x, &y);
//         div_t result = div(x, y);
//         printf("quotient: %d, remainder: %d\n", result.quot, result.rem);
//         return 0;
//     }
//
// Behaviours that are reproduced EXACTLY (including the original bugs):
//
//   * The return value of scanf() is ignored, so on input failure or matching
//     failure the corresponding variable keeps its initial value of 1.  A
//     failure on the first conversion means the second conversion is never
//     attempted, so BOTH x and y stay 1.
//
//   * "%d %d": glibc eats leading whitespace (isspace() in the "C" locale)
//     before the conversion, then matches an optional sign followed by one or
//     more decimal digits.  The literal ' ' in the format is redundant because
//     "%d" already skips whitespace.
//
//   * glibc's vfscanf collects the digits into a work buffer and runs
//     __strtol_internal() over it, i.e. the value is accumulated in a
//     `long int` and CLAMPED to LONG_MAX / LONG_MIN on overflow, and only then
//     assigned through the `int *` argument -- which truncates.  So
//     "9999999999999999999999" yields (int)LONG_MAX == -1, and
//     "-9999999999999999999999" yields (int)LONG_MIN == 0.
//
//   * A leading "0" is consumed by glibc's base-prefix probe but, because "%d"
//     forces base 10, "0x10" scans as 0 and leaves "x10" in the stream (so the
//     second conversion then fails and y stays 1).
//
//   * div(x, y) is a real libc call.  glibc compiles it down to
//         mov %edi,%eax ; cltd ; idiv %esi
//     (the ANSI-truncation fix-up in glibc's source is provably dead on
//     two's-complement machines and is optimised away).  Consequently
//     div(x, 0) and div(INT_MIN, -1) execute a trapping `idiv`, raising a
//     *hardware* SIGFPE that kills the process before anything is printed.
//     This translation issues the very same `idiv` instruction so that the
//     fault is byte-for-byte indistinguishable (same signal, same si_code,
//     and it is still delivered when SIGFPE is blocked or ignored, because
//     the kernel force-delivers synchronous faults).
//
//   * printf()'s return value is ignored and a failing write (EPIPE, ENOSPC,
//     EBADF, ...) does NOT change the exit status: main still `return 0`s.
//     Rust's `print!` macro would panic instead, so the output is written with
//     `write_all` and the error is discarded.  Likewise, C never touches the
//     SIGPIPE disposition it inherited, whereas the Rust runtime forces it to
//     SIG_IGN before `main`; the inherited disposition is captured from an
//     ELF .init_array constructor and restored, so a broken pipe kills this
//     program with SIGPIPE just like the C one.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::mem::ManuallyDrop;
use std::os::raw::c_int;
use std::os::unix::fs::MetadataExt;
use std::os::unix::io::FromRawFd;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// ---------------------------------------------------------------------------
// libc bits.  libc is already linked by the Rust standard library, so no
// external crate is required.
// ---------------------------------------------------------------------------

extern "C" {
    /// signal(2).  Returns the previous handler, or SIG_ERR on failure.
    fn signal(sig: c_int, handler: usize) -> usize;
    /// raise(3) -- only used by the non-x86_64 fallback path.
    #[allow(dead_code)]
    fn raise(sig: c_int) -> c_int;
}

/// Only referenced by the generic (non-x86_64, non-aarch64) `machine_div`.
#[allow(dead_code)]
const SIGFPE: c_int = 8;
const SIGPIPE: c_int = 13;
const SIG_IGN: usize = 1;
const SIG_ERR: usize = usize::MAX;

// ---------------------------------------------------------------------------
// Capture the SIGPIPE disposition this process was started with.
//
// ELF .init_array constructors run before libc calls `main`, and therefore
// before Rust's `lang_start` overrides SIGPIPE with SIG_IGN.  For a cdylib the
// constructor runs at dlopen() time, which is likewise before `driver_main` is
// reached.  Either way we observe exactly what the C program would have
// inherited.
// ---------------------------------------------------------------------------

static ORIG_SIGPIPE: AtomicUsize = AtomicUsize::new(0);
static ORIG_SIGPIPE_VALID: AtomicBool = AtomicBool::new(false);

extern "C" fn capture_inherited_signal_dispositions() {
    unsafe {
        // signal() atomically reports the old handler; immediately put it back
        // so that the observation itself has no effect.
        let prev = signal(SIGPIPE, SIG_IGN);
        if prev != SIG_ERR {
            signal(SIGPIPE, prev);
            ORIG_SIGPIPE.store(prev, Ordering::Relaxed);
            ORIG_SIGPIPE_VALID.store(true, Ordering::Relaxed);
        }
    }
}

#[used]
#[cfg_attr(
    any(target_os = "linux", target_os = "android", target_os = "freebsd"),
    link_section = ".init_array"
)]
#[cfg_attr(target_vendor = "apple", link_section = "__DATA,__mod_init_func")]
static INIT_ARRAY_ENTRY: extern "C" fn() = capture_inherited_signal_dispositions;

fn restore_inherited_signal_dispositions() {
    if ORIG_SIGPIPE_VALID.load(Ordering::Relaxed) {
        let orig = ORIG_SIGPIPE.load(Ordering::Relaxed);
        unsafe {
            signal(SIGPIPE, orig);
        }
    }
}

// ---------------------------------------------------------------------------
// stdin, modelled on glibc's `FILE` for fd 0.
//
// A one-byte-at-a-time `read()` loop would give the right *characters*, but it
// would NOT give the right side effect on the shared file offset of fd 0.  That
// offset is observable to whoever else holds the same open file description --
// `{ ./driver; cat; } < input` is the classic case -- so it is part of the
// program's byte-identical behaviour and is reproduced here:
//
//   * glibc reads in blocks.  `_IO_file_doallocate` picks `st_blksize` when
//     that is a positive value below `BUFSIZ` (8192 on glibc), otherwise
//     `BUFSIZ`; `_IO_new_file_underflow` then issues ONE `read()` of exactly
//     that size (a short read is accepted as-is, never retried).
//   * EOF is sticky (C99), so once a `read()` returns 0 no further syscall is
//     made.  A read *error* likewise ends the conversion with input failure,
//     and this program never retries afterwards.
//   * `ungetc()` on the character that was just handed out of the buffer merely
//     steps the read pointer back (`_IO_sputbackc`), so the pushed-back byte
//     counts as *not consumed*.
//   * At exit `_IO_cleanup` -> `_IO_unbuffer_all` -> `_IO_new_file_sync` rewinds
//     the descriptor by the number of buffered-but-unconsumed bytes:
//     `lseek(0, read_ptr - read_end, SEEK_CUR)`, ignoring `ESPIPE` on
//     unseekable input.  It runs *after* the stdout flush, and not at all when
//     the process dies from a signal.
//
// A read error is otherwise indistinguishable from EOF for this program: glibc
// sets the stream's error flag, the conversion reports input failure, and the
// caller's variable keeps its previous value -- the same thing EOF does.
// ---------------------------------------------------------------------------

/// glibc's `BUFSIZ`.
const BUFSIZ: usize = 8192;

struct Input {
    /// fd 0, borrowed -- never closed, since the C program does not close it.
    fd0: ManuallyDrop<File>,
    buf: Vec<u8>,
    /// glibc's `_IO_read_ptr`.
    pos: usize,
    /// glibc's `_IO_read_end`.
    end: usize,
    eof: bool,
}

impl Input {
    fn new() -> Self {
        Input {
            fd0: ManuallyDrop::new(unsafe { File::from_raw_fd(0) }),
            buf: Vec::new(),
            pos: 0,
            end: 0,
            eof: false,
        }
    }

    /// glibc's `_IO_file_doallocate`, which fstat()s the descriptor to size the
    /// buffer. Called lazily, on the first underflow, exactly as glibc does.
    fn allocate(&mut self) {
        if !self.buf.is_empty() {
            return;
        }
        let mut size = BUFSIZ;
        if let Ok(md) = self.fd0.metadata() {
            let bs = md.blksize();
            if bs > 0 && bs < BUFSIZ as u64 {
                size = bs as usize;
            }
        }
        self.buf = vec![0u8; size];
    }

    fn next_byte(&mut self) -> Option<u8> {
        if self.pos < self.end {
            let b = self.buf[self.pos];
            self.pos += 1;
            return Some(b);
        }
        if self.eof {
            return None; // C99 requires EOF to be sticky.
        }
        self.allocate();
        // One read() of the whole buffer, like `_IO_SYSREAD`; short reads are
        // accepted as-is and EINTR is *not* retried (neither does glibc).
        let n = {
            let Self { fd0, buf, .. } = self;
            match fd0.read(&mut buf[..]) {
                Ok(n) => n,
                Err(_) => 0,
            }
        };
        self.pos = 0;
        self.end = n;
        if n == 0 {
            self.eof = true;
            return None;
        }
        self.pos = 1;
        Some(self.buf[0])
    }

    /// glibc's `_IO_sputbackc`: the byte came straight out of the buffer, so the
    /// read pointer is simply stepped back and the byte counts as unconsumed.
    fn unget(&mut self, _b: u8) {
        debug_assert!(self.pos > 0);
        self.pos -= 1;
    }

    /// glibc's `_IO_new_file_sync`, reached from `_IO_cleanup()` at exit.
    fn sync_at_exit(&mut self) {
        let delta = self.pos as i64 - self.end as i64;
        if delta != 0 {
            // Fails with ESPIPE on pipes/ttys, which glibc deliberately ignores.
            let _ = self.fd0.seek(SeekFrom::Current(delta));
        }
    }
}

/// The characters C's isspace() accepts in the "C" locale, i.e. the set a
/// scanf conversion skips over.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// One "%d" conversion.
///
/// Returns `None` on input failure (EOF/error before any non-whitespace) or on
/// matching failure (no digits), in which case the caller must leave its
/// variable untouched -- exactly like scanf.
fn scan_i32(input: &mut Input) -> Option<i32> {
    // glibc eats whitespace for every conversion except %c, %[, %n.
    let mut c = loop {
        match input.next_byte() {
            None => return None, // input failure
            Some(b) if is_c_space(b) => continue,
            Some(b) => break b,
        }
    };

    // Optional sign.
    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        match input.next_byte() {
            None => return None, // sign then EOF => matching failure
            Some(b) => c = b,
        }
    }

    if !c.is_ascii_digit() {
        input.unget(c);
        return None; // matching failure
    }

    // Accumulate the magnitude.  Saturation at u64::MAX is enough to detect
    // "overflowed a long", which is all strtol()'s clamping needs to know.
    let mut magnitude: u64 = 0;
    loop {
        let digit = u64::from(c - b'0');
        magnitude = magnitude.saturating_mul(10).saturating_add(digit);
        match input.next_byte() {
            None => break,
            Some(b) if b.is_ascii_digit() => c = b,
            Some(b) => {
                input.unget(b);
                break;
            }
        }
    }

    // strtol() clamps an out-of-range result to LONG_MAX / LONG_MIN; glibc then
    // truncates the `long` when storing it through the `int *` argument.
    const LONG_MAX_MAG: u64 = i64::MAX as u64; // 2^63 - 1
    let as_long: i64 = if negative {
        if magnitude > LONG_MAX_MAG + 1 {
            i64::MIN
        } else {
            (magnitude as i128).wrapping_neg() as i64
        }
    } else if magnitude > LONG_MAX_MAG {
        i64::MAX
    } else {
        magnitude as i64
    };

    Some(as_long as i32)
}

// ---------------------------------------------------------------------------
// div(3)
// ---------------------------------------------------------------------------

/// `numer / denom` and `numer % denom` computed by the *hardware* divide
/// instruction, so that the trapping cases (denom == 0, and
/// INT_MIN / -1) raise a genuine SIGFPE exactly as glibc's `div` does.
#[cfg(target_arch = "x86_64")]
fn machine_div(numer: i32, denom: i32) -> (i32, i32) {
    let quot: i32;
    let rem: i32;
    unsafe {
        // Byte-for-byte what glibc's div() compiles to:
        //     mov %edi,%eax ; cltd ; idiv %esi
        core::arch::asm!(
            "cdq",
            "idiv {denom:e}",
            denom = in(reg) denom,
            inout("eax") numer => quot,
            out("edx") rem,
        );
    }
    (quot, rem)
}

#[cfg(target_arch = "aarch64")]
fn machine_div(numer: i32, denom: i32) -> (i32, i32) {
    // AArch64's SDIV does not trap: x/0 == 0 and INT_MIN/-1 == INT_MIN.  The
    // remainder is then formed with MSUB, i.e. numer - quot * denom.
    let quot = if denom == 0 {
        0
    } else {
        numer.wrapping_div(denom)
    };
    let rem = numer.wrapping_sub(quot.wrapping_mul(denom));
    (quot, rem)
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn machine_div(numer: i32, denom: i32) -> (i32, i32) {
    // Unknown ISA: division by zero / INT_MIN by -1 is undefined in C.  Raise
    // the fault the reference platform raises rather than inventing a value.
    if denom == 0 || (numer == i32::MIN && denom == -1) {
        unsafe {
            signal(SIGFPE, 0 /* SIG_DFL */);
            raise(SIGFPE);
        }
        std::process::abort();
    }
    (numer / denom, numer % denom)
}

/// glibc's `div_t div (int numer, int denom)`.
fn div(numer: i32, denom: i32) -> (i32, i32) {
    let (mut quot, mut rem) = machine_div(numer, denom);
    // glibc's ANSI-truncation fix-up, transcribed verbatim.  It is provably
    // dead on every two's-complement machine (a truncating division always
    // gives a remainder with the sign of the numerator), which is why the
    // compiler deletes it -- but the C source contains it, so it is kept here.
    if numer >= 0 && rem < 0 {
        quot = quot.wrapping_add(1);
        rem = rem.wrapping_sub(denom);
    }
    (quot, rem)
}

// ---------------------------------------------------------------------------
// The translated `main`.
// ---------------------------------------------------------------------------

/// The body of the C program's `main`, exported across the C ABI under the
/// same name that `gcc -shared -Dmain=driver_main c_src/src/main.c` exports.
#[no_mangle]
pub extern "C" fn driver_main() -> c_int {
    restore_inherited_signal_dispositions();

    let mut x: i32 = 1;
    let mut y: i32 = 1;

    // scanf("%d %d", &x, &y);  -- return value ignored, just like the C code.
    // A failure on the first conversion aborts the whole scanf, so the second
    // conversion is only attempted when the first one succeeded.
    let mut input = Input::new();
    if let Some(v) = scan_i32(&mut input) {
        x = v;
        if let Some(v) = scan_i32(&mut input) {
            y = v;
        }
    }

    // div_t result = div(x, y);   <-- traps for y == 0 and for INT_MIN / -1
    let (quot, rem) = div(x, y);

    // printf("quotient: %d, remainder: %d\n", ...);  -- errors are ignored, and
    // they do not affect the exit status.
    let line = format!("quotient: {}, remainder: {}\n", quot, rem);
    let mut out = io::stdout();
    let _ = out.write_all(line.as_bytes());
    let _ = out.flush();

    // libc's exit() runs _IO_cleanup(): stdout is flushed (above) and then the
    // unconsumed part of stdin's buffer is given back to the descriptor.  This
    // is skipped when the process dies from a signal, which is why it lives
    // after the div() above rather than in a guard object.
    input.sync_at_exit();

    0 // return 0;
}
