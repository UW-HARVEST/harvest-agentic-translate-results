// Rust translation of c_src/src/main.c
//
// Original C:
//     int main() {
//         int x = 1, y = 1;
//         scanf("%d %d", &x, &y);
//         div_t result = div(x, y);
//         printf("quotient: %d, remainder: %d\n", result.quot, result.rem);
//         return 0;
//     }
//
// Behaviour that must be reproduced exactly:
//
//   * scanf("%d %d") skips arbitrary leading whitespace (including newlines)
//     before each conversion, and leaves a variable untouched when its
//     conversion does not happen (matching failure / input failure), so the
//     initial value of 1 survives.
//   * glibc converts %d through a `long`, so out-of-range input saturates at
//     LONG_MAX / LONG_MIN and is then truncated to `int`
//     (e.g. "99999999999999999999999" -> -1, "4294967296" -> 0).
//   * stdin is consumed lazily, in blocks, exactly like glibc's buffered
//     FILE*: the program must not block waiting for EOF once both conversions
//     have finished, because the C program does not.
//   * div(x, 0) and div(INT_MIN, -1) are undefined behaviour in C; on x86-64
//     the hardware `idiv` instruction raises SIGFPE, which is what the
//     original binary does (killed by signal 8).
//   * printf/fflush failures are NOT fatal in C: printf returns a negative
//     value and main still returns 0. Rust's `print!` macro instead panics on
//     a write error, so it must not be used here.
//   * The C program never touches SIGPIPE, so it runs with whatever
//     disposition it inherited. The Rust runtime forcibly sets SIGPIPE to
//     SIG_IGN before main, which would make the program survive a broken
//     stdout where the C program dies of SIGPIPE. The original disposition is
//     therefore captured in an ELF constructor (which runs before the Rust
//     runtime initialises) and restored at the top of main.

use std::io::Read;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

// ---------------------------------------------------------------------------
// Preserving the inherited SIGPIPE disposition
// ---------------------------------------------------------------------------

const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;
const SIG_ERR: usize = usize::MAX;

extern "C" {
    /// `signal(2)`: installs `handler` and returns the previous handler.
    fn signal(signum: i32, handler: usize) -> usize;
}

/// Sentinel meaning "constructor never ran / query failed".
static ORIGINAL_SIGPIPE: AtomicUsize = AtomicUsize::new(SIG_ERR);

/// Runs from `.init_array`, i.e. during process start-up and therefore *before*
/// `std::rt::init` replaces the SIGPIPE disposition with SIG_IGN. Reads the
/// inherited disposition and immediately puts it back.
extern "C" fn capture_sigpipe_disposition() {
    unsafe {
        let previous = signal(SIGPIPE, SIG_DFL);
        if previous != SIG_ERR {
            // Restore what we just displaced; we only wanted to observe it.
            signal(SIGPIPE, previous);
            ORIGINAL_SIGPIPE.store(previous, Ordering::SeqCst);
        }
    }
}

#[used]
#[link_section = ".init_array"]
static SIGPIPE_CTOR: extern "C" fn() = capture_sigpipe_disposition;

/// Undo the Rust runtime's `SIGPIPE -> SIG_IGN`, so that stdout behaves the way
/// it does for the C program.
fn restore_inherited_sigpipe() {
    let original = ORIGINAL_SIGPIPE.load(Ordering::SeqCst);
    if original != SIG_ERR {
        unsafe {
            signal(SIGPIPE, original);
        }
    }
}

// ---------------------------------------------------------------------------
// A lazily filled stdin buffer, mirroring a fully buffered glibc FILE*
// ---------------------------------------------------------------------------

/// glibc's default stdio buffer size.
const BUFSIZ: usize = 4096;

struct Input {
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
}

impl Input {
    fn new() -> Self {
        Input {
            buf: Vec::new(),
            pos: 0,
            eof: false,
        }
    }

    /// Returns the next byte without consuming it, reading a fresh block from
    /// stdin only when the buffer is exhausted. `None` means end-of-file (or an
    /// unrecoverable read error, which glibc also reports as "no more input").
    fn peek(&mut self) -> Option<u8> {
        if self.pos < self.buf.len() {
            return Some(self.buf[self.pos]);
        }
        if self.eof {
            return None;
        }

        self.buf.clear();
        self.pos = 0;
        self.buf.resize(BUFSIZ, 0);

        let mut stdin = std::io::stdin().lock();
        loop {
            match stdin.read(&mut self.buf) {
                Ok(0) => {
                    self.eof = true;
                    self.buf.clear();
                    return None;
                }
                Ok(n) => {
                    self.buf.truncate(n);
                    return Some(self.buf[0]);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    self.buf.clear();
                    return None;
                }
            }
        }
    }

    fn bump(&mut self) {
        self.pos += 1;
    }
}

/// The bytes `isspace()` treats as whitespace in the C locale, which is what
/// `scanf` skips before a `%d` conversion.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Emulates a single `%d` conversion of glibc's scanf.
///
/// Returns `Some(value)` when the conversion succeeded, `None` on input failure
/// (EOF before any character) or matching failure (no digits present).
fn scan_int(input: &mut Input) -> Option<i32> {
    // Directive whitespace plus %d's own leading-whitespace skip.
    while let Some(b) = input.peek() {
        if is_c_space(b) {
            input.bump();
        } else {
            break;
        }
    }

    // Optional sign.
    let mut negative = false;
    match input.peek() {
        None => return None, // input failure
        Some(b) if b == b'+' || b == b'-' => {
            negative = b == b'-';
            input.bump();
        }
        Some(_) => {}
    }

    // At least one digit is required, otherwise it is a matching failure.
    match input.peek() {
        Some(b) if b.is_ascii_digit() => {}
        _ => return None,
    }

    // Accumulate as a C `long` (64-bit) with strtol-style saturation.
    let mut acc: i64 = 0;
    let mut saturated = false;
    while let Some(b) = input.peek() {
        if !b.is_ascii_digit() {
            break;
        }
        let digit = (b - b'0') as i64;
        input.bump();
        if saturated {
            continue;
        }
        match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
            Some(v) => acc = v,
            None => saturated = true,
        }
    }

    let value: i64 = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        // `-acc` cannot overflow: acc <= i64::MAX, and the one input that would
        // reach i64::MIN (9223372036854775808) sets `saturated` instead.
        -acc
    } else {
        acc
    };

    // Storing a `long` into an `int`: implementation-defined truncation, which
    // on every target we care about is a plain two's-complement narrowing.
    Some(value as u64 as u32 as i32)
}

// ---------------------------------------------------------------------------
// div()
// ---------------------------------------------------------------------------

/// `div(num, den)` as executed by the original binary: a raw hardware signed
/// division, so division by zero and INT_MIN / -1 raise SIGFPE just like the C
/// program instead of producing a Rust panic.
#[cfg(target_arch = "x86_64")]
fn c_div(num: i32, den: i32) -> (i32, i32) {
    let quot: i32;
    let rem: i32;
    unsafe {
        std::arch::asm!(
            "cdq",
            "idiv {den:e}",
            den = in(reg) den,
            inout("eax") num => quot,
            out("edx") rem,
            options(nostack),
        );
    }
    (quot, rem)
}

#[cfg(not(target_arch = "x86_64"))]
fn c_div(num: i32, den: i32) -> (i32, i32) {
    match (num.checked_div(den), num.checked_rem(den)) {
        (Some(q), Some(r)) => (q, r),
        // Mirror the fatal arithmetic fault of the original program.
        _ => {
            unsafe {
                libc_raise_sigfpe();
            }
            unreachable!()
        }
    }
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn libc_raise_sigfpe() {
    extern "C" {
        fn raise(sig: i32) -> i32;
    }
    const SIGFPE: i32 = 8;
    // Ensure SIGFPE is fatal even if it was ignored, matching a hardware fault.
    signal(SIGFPE, SIG_DFL);
    raise(SIGFPE);
    std::process::abort();
}

fn main() {
    restore_inherited_sigpipe();

    let mut x: i32 = 1;
    let mut y: i32 = 1;

    let mut input = Input::new();
    if let Some(v) = scan_int(&mut input) {
        x = v;
        if let Some(v) = scan_int(&mut input) {
            y = v;
        }
    }

    let (quot, rem) = c_div(x, y);

    // C's printf/fflush report failure through a return value; they never
    // abort the program. So all write errors are deliberately discarded.
    let text = format!("quotient: {}, remainder: {}\n", quot, rem);
    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(text.as_bytes());
    let _ = stdout.flush();
}
