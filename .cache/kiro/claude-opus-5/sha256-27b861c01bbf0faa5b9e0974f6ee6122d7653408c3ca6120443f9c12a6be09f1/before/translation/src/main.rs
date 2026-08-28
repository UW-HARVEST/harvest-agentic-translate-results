// Rust translation of c_src/src/main.c
//
// Count from a starting point, stopping when the count ends in 9 (base 10).
//
// This is a behavior-preserving translation: the original C program's quirks
// (strtol saturation, long -> int truncation, C's sign-preserving `%`, and the
// signed overflow wrap on `val++`) are reproduced rather than "fixed".

use std::io::Write;

/// `isspace()` in the C locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Equivalent of `strtol(s, &end, 10)`.
///
/// Returns `(value, end_index)` where `end_index` is the offset of the byte
/// `end` points at. Per the C standard, when no conversion can be performed the
/// value is 0 and `end` is set back to the start of the string, so `end_index`
/// is 0 in that case (a successful conversion always consumes at least one
/// digit, so 0 is unambiguous).
///
/// On overflow, `strtol` clamps to `LONG_MAX` / `LONG_MIN` but still consumes
/// the whole digit run; `long` is 64-bit on the platforms this targets.
fn strtol_base10(s: &[u8]) -> (i64, usize) {
    let mut i = 0usize;

    // Skip leading whitespace.
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    // Optional sign.
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    // Digit run.
    let digits_start = i;
    let mut acc: i64 = 0;
    let mut overflowed = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = i64::from(s[i] - b'0');
        if !overflowed {
            match acc.checked_mul(10).and_then(|a| a.checked_add(digit)) {
                Some(v) => acc = v,
                None => overflowed = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No subject sequence: value 0, end == start of string.
        return (0, 0);
    }

    if overflowed {
        // Note: the exact value LONG_MIN takes this path too, and clamping
        // yields LONG_MIN, which is the correct result anyway.
        return (if negative { i64::MIN } else { i64::MAX }, i);
    }

    (if negative { -acc } else { acc }, i)
}

fn run(out: &mut impl Write) -> i32 {
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();

    // C's argc counts argv[0], so `argc != 2` means "not exactly one argument".
    if args.len() != 2 {
        let _ = write!(out, "Error: should only be a single (integer) argument!\n");
        return 1;
    }

    let arg = os_bytes(&args[1]);
    let (parsed, end_index) = strtol_base10(&arg);
    if end_index == 0 {
        // end is set to start of string if nothing parsed
        let _ = write!(out, "Error: first argument must be an integer!\n");
        return 1;
    }

    // `int val = strtol(...)` truncates the long to int.
    let mut val = parsed as i32;

    loop {
        let _ = write!(out, "{}\n", val);
        // C's `%` truncates toward zero, so negative values never match 9.
        if val % 10 == 9 {
            break;
        }
        // Signed overflow is UB in C; real compilers wrap.
        val = val.wrapping_add(1);
    }

    0
}

#[cfg(unix)]
fn os_bytes(s: &std::ffi::OsStr) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    s.as_bytes().to_vec()
}

#[cfg(not(unix))]
fn os_bytes(s: &std::ffi::OsStr) -> Vec<u8> {
    s.to_string_lossy().into_owned().into_bytes()
}

/// Rust's runtime sets `SIGPIPE` to `SIG_IGN`; C leaves it at `SIG_DFL`.
/// Restore the C behavior so that a closed stdout terminates the process the
/// same way the original program does instead of looping to completion.
#[cfg(unix)]
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    // SAFETY: `signal` with SIG_DFL for SIGPIPE is a well-defined libc call and
    // takes no pointers; this is the one unavoidable bit of unsafety needed to
    // match the C program's signal disposition.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

fn main() {
    restore_default_sigpipe();

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    let code = run(&mut out);
    let _ = out.flush();
    std::process::exit(code);
}
