// Rust translation of c_src/src/main.c
//
// Index into a passed string and print the substring indexed by [start, stop).
// If there is no start, use 0.
// If there is no stop, use the end of the string.
//
// This translation reproduces the original C behavior exactly, including its
// bugs and quirks:
//   * `start > len` / `stop > len` compare an `int` against a `size_t`, so the
//     `int` is converted to `size_t` (sign extension, reinterpreted as
//     unsigned). Any negative start/stop therefore becomes a huge unsigned
//     value and is reported as "off the end of the string".
//   * The third-argument check tests the *stale* `end` pointer left over from
//     parsing argv[2] (strtol is called with a NULL endptr for argv[3]), so it
//     can never fire.
//   * The two "must be an integer!" messages are printed without a trailing
//     newline.
//   * strtol saturates at LONG_MIN/LONG_MAX and the result is then truncated
//     when stored into an `int`.
//   * printf's return value is never checked, so write errors are ignored.

use std::io::Write;
use std::os::unix::ffi::OsStrExt;

// ---------------------------------------------------------------------------
// SIGPIPE disposition
//
// The Rust runtime sets SIGPIPE to SIG_IGN before `main` runs; a C program never
// does that -- it simply inherits whatever disposition its parent had. That
// difference is observable on EVERY code path here, because every branch writes
// to stdout:
//
//   * Parent leaves SIGPIPE at SIG_DFL (what every shell does). `driver hello |
//     true` must be KILLED by SIGPIPE. With SIG_IGN left in place the write
//     would instead fail with EPIPE, be silently ignored (C never checks
//     printf's return value either), and the process would exit 0/1.
//   * Parent has already set SIGPIPE to SIG_IGN. Then the C program inherits
//     SIG_IGN and SURVIVES the broken pipe, exiting 0/1. So unconditionally
//     forcing SIG_DFL would be wrong in the opposite direction.
//
// To get both right the original disposition has to be captured *before* the
// Rust runtime overwrites it. An ELF `.init_array` constructor runs during
// `__libc_start_main`, i.e. ahead of `main` and therefore ahead of
// `std::rt::init`, so it still sees the inherited value.
// ---------------------------------------------------------------------------

const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;
/// `(sighandler_t) -1`, the error return of `signal()`.
const SIG_ERR: usize = usize::MAX;

extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
}

/// The SIGPIPE disposition inherited from the parent process.
static INHERITED_SIGPIPE: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(SIG_DFL);

#[used]
#[link_section = ".init_array"]
static CAPTURE_SIGPIPE_CTOR: extern "C" fn() = capture_inherited_sigpipe;

/// Runs from `.init_array`, before the Rust runtime installs SIG_IGN.
extern "C" fn capture_inherited_sigpipe() {
    unsafe {
        // `signal` returns the previous handler; put it straight back. No other
        // threads exist yet and nothing has been written, so this is safe.
        let previous = signal(SIGPIPE, SIG_DFL);
        if previous != SIG_ERR {
            signal(SIGPIPE, previous);
            INHERITED_SIGPIPE.store(previous, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

/// Undo the Rust runtime's `SIGPIPE` -> `SIG_IGN`, restoring exactly what the
/// parent handed us, so stdout write failures behave as they do for the C.
fn restore_inherited_sigpipe() {
    unsafe {
        signal(
            SIGPIPE,
            INHERITED_SIGPIPE.load(std::sync::atomic::Ordering::Relaxed),
        );
    }
}

/// A stand-in for a `char *` pointing into one of the argv strings:
/// (argv index, byte offset). Comparing these tuples yields the same result as
/// comparing the real C pointers, because a pointer derived from argv[2] can
/// never be equal to the start of argv[3].
type CharPtr = (usize, usize);

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// `strtol(nptr, &end, 10)` over raw bytes.
///
/// Returns the converted `long` value and the number of bytes consumed, which
/// is the offset of the `end` pointer from the start of the string. A consumed
/// count of 0 means no conversion could be performed (i.e. `end == nptr`).
fn c_strtol_base10(s: &[u8]) -> (i64, usize) {
    let mut i = 0usize;

    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    let mut acc: i64 = 0;
    let mut overflowed = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = i64::from(s[i] - b'0');
        if !overflowed {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => overflowed = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits: strtol performs no conversion and sets end = nptr.
        return (0, 0);
    }

    let value = if overflowed {
        // strtol clamps to LONG_MIN / LONG_MAX and sets ERANGE.
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

    (value, i)
}

/// `printf("%.*s\n", precision, bytes)`: emit at most `precision` bytes (a
/// negative precision is treated by printf as if it were omitted).
fn print_precision_string(out: &mut impl Write, precision: i32, bytes: &[u8]) {
    let count = if precision < 0 {
        bytes.len()
    } else {
        std::cmp::min(precision as usize, bytes.len())
    };
    let _ = out.write_all(&bytes[..count]);
    let _ = out.write_all(b"\n");
}

fn main() {
    restore_inherited_sigpipe();

    let argv: Vec<Vec<u8>> = std::env::args_os()
        .map(|a| a.as_bytes().to_vec())
        .collect();
    let argc = argv.len();

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    if (argc > 4) || (argc == 1) {
        let _ = out.write_all(b"Error: there should be one to three arguments passed:\n");
        let _ = out.write_all(b"<string> [start] [stop]\n");
        let _ = out.flush();
        std::process::exit(1);
    }

    let len: u64 = argv[1].len() as u64; // size_t len = strlen(argv[1]);
    let start: i32;
    let stop: i32;

    // char *end; -- uninitialized in C; only ever read after being set below.
    let mut end: Option<CharPtr> = None;

    if argc >= 3 {
        let (value, consumed) = c_strtol_base10(&argv[2]);
        end = Some((2, consumed));
        start = value as i32; // long -> int truncation
        if end == Some((2, 0)) {
            // if (end == argv[2])
            let _ = out.write_all(b"Second argument must be an integer!");
            let _ = out.flush();
            std::process::exit(1);
        }
        // if (start > len): int is converted to size_t, so negatives are huge.
        if (start as i64 as u64) > len {
            let _ = out.write_all(b"Error: start is off the end of the string!\n");
            let _ = out.flush();
            std::process::exit(1);
        }
    } else {
        start = 0;
    }

    if argc == 4 {
        // strtol(argv[3], NULL, 10): `end` is NOT updated here.
        let (value, _consumed) = c_strtol_base10(&argv[3]);
        stop = value as i32; // long -> int truncation
        // if (end == argv[3]): compares the stale `end` from argv[2], which can
        // never equal the start of argv[3], so this branch is unreachable.
        if end == Some((3, 0)) {
            let _ = out.write_all(b"Third argument must be an integer!");
            let _ = out.flush();
            std::process::exit(1);
        }

        // if (stop > len): int is converted to size_t, so negatives are huge.
        if (stop as i64 as u64) > len {
            let _ = out.write_all(b"Error: stop is off the end of the string!\n");
            let _ = out.flush();
            std::process::exit(1);
        }

        if stop <= start {
            let _ = out.write_all(b"Error: stop must come after start!\n");
            let _ = out.flush();
            std::process::exit(1);
        }
    } else {
        stop = len as i32; // size_t -> int truncation
    }

    // char arithmetic: skip ahead `start` characters in the array
    let offset = start as usize;
    let tail = &argv[1][offset..];
    print_precision_string(&mut out, stop.wrapping_sub(start), tail);

    let _ = out.flush();
    std::process::exit(0);
}
