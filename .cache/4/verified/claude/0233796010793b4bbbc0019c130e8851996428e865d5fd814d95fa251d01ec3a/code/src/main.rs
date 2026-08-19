// Rust translation of c_src/src/main.c
//
// Takes two arguments, a base and an exponent, and prints base^exponent.
//
// The original C program relies on the exact semantics of the C library
// routines `strtod` and `pow` (including the `errno` values they produce),
// so those two functions -- and only those two -- are used through FFI in
// order to reproduce byte-identical behaviour.  Everything else is plain
// safe Rust.

use std::ffi::CString;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::raw::c_char;

// <errno.h>
const EDOM: i32 = 33;
const ERANGE: i32 = 34;

// <signal.h>
const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;

extern "C" {
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> f64;
    fn pow(x: f64, y: f64) -> f64;
    fn __errno_location() -> *mut i32;
    fn signal(signum: i32, handler: usize) -> usize;
}

/// The Rust runtime installs `SIG_IGN` for `SIGPIPE` before `main` runs, which
/// a C program does not do: a C program writing to a closed pipe is killed by
/// `SIGPIPE`, whereas Rust would silently observe `EPIPE` and keep going.
/// Restore the default disposition so the process dies exactly like the C one.
fn restore_default_sigpipe() {
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn errno_get() -> i32 {
    unsafe { *__errno_location() }
}

fn errno_set(value: i32) {
    unsafe {
        *__errno_location() = value;
    }
}

/// Result of a `strtod` call: the converted value, the errno afterwards and
/// whether the whole input string was consumed (i.e. `*endptr == '\0'`).
struct StrtodResult {
    value: f64,
    errno: i32,
    fully_consumed: bool,
}

/// Calls the C library `strtod` on `bytes` (a NUL-free byte string), exactly
/// as the C code does, resetting `errno` to 0 beforehand.
fn c_strtod(bytes: &[u8]) -> StrtodResult {
    // Command line arguments never contain interior NUL bytes; if one somehow
    // did, mirror C by only considering the part up to the first NUL.
    let cstr = match CString::new(bytes) {
        Ok(s) => s,
        Err(e) => {
            let pos = e.nul_position();
            CString::new(&e.into_vec()[..pos]).expect("no interior NUL")
        }
    };

    let ptr = cstr.as_ptr();
    let mut endptr: *mut c_char = std::ptr::null_mut();

    errno_set(0);
    let value = unsafe { strtod(ptr, &mut endptr) };
    let errno = errno_get();

    // *endptr != '\0' ?
    let fully_consumed = unsafe { !endptr.is_null() && *endptr == 0 };

    StrtodResult {
        value,
        errno,
        fully_consumed,
    }
}

/// Formats a double the way glibc's `printf("%.2f", v)` does.
fn fmt_f64_2(v: f64) -> String {
    if v.is_nan() {
        if v.is_sign_negative() {
            String::from("-nan")
        } else {
            String::from("nan")
        }
    } else if v.is_infinite() {
        if v.is_sign_negative() {
            String::from("-inf")
        } else {
            String::from("inf")
        }
    } else {
        format!("{:.2}", v)
    }
}

/// Concatenates the pieces of a message into one byte buffer.
fn join(parts: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for p in parts {
        out.extend_from_slice(p);
    }
    out
}

fn write_stderr(bytes: &[u8]) {
    let stderr = std::io::stderr();
    let mut handle = stderr.lock();
    let _ = handle.write_all(bytes);
    let _ = handle.flush();
}

fn write_stdout(bytes: &[u8]) {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(bytes);
    let _ = handle.flush();
}

fn main() {
    restore_default_sigpipe();

    let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let argc = argv.len();

    if argc != 3 {
        // fprintf(stderr, "Usage: %s base exponent\n", argv[0]);
        let prog: &[u8] = if argc >= 1 {
            argv[0].as_bytes()
        } else {
            // glibc prints "(null)" for a NULL %s argument
            b"(null)"
        };
        write_stderr(&join(&[&b"Usage: "[..], prog, &b" base exponent\n"[..]]));
        std::process::exit(1);
    }

    let arg1 = argv[1].as_bytes();
    let arg2 = argv[2].as_bytes();

    // Convert base
    let r1 = c_strtod(arg1);
    if r1.errno == ERANGE {
        write_stderr(&join(&[
            &b"Range error while converting base '"[..],
            arg1,
            &b"'\n"[..],
        ]));
        std::process::exit(1);
    } else if !r1.fully_consumed {
        write_stderr(&join(&[
            &b"Invalid numeric input for base: '"[..],
            arg1,
            &b"'\n"[..],
        ]));
        std::process::exit(1);
    }
    let base = r1.value;

    // Convert exponent
    let r2 = c_strtod(arg2);
    if r2.errno == ERANGE {
        write_stderr(&join(&[
            &b"Range error while converting exponent '"[..],
            arg2,
            &b"'\n"[..],
        ]));
        std::process::exit(1);
    } else if !r2.fully_consumed {
        write_stderr(&join(&[
            &b"Invalid numeric input for exponent: '"[..],
            arg2,
            &b"'\n"[..],
        ]));
        std::process::exit(1);
    }
    let exponent = r2.value;

    // Calculate power
    errno_set(0);
    let result = unsafe { pow(base, exponent) };
    let err = errno_get();
    if err == EDOM {
        write_stderr(
            format!(
                "Domain error: pow({}, {}) is undefined in the real number domain.\n",
                fmt_f64_2(base),
                fmt_f64_2(exponent)
            )
            .as_bytes(),
        );
        std::process::exit(1);
    } else if err == ERANGE {
        write_stderr(
            format!(
                "Range error: pow({}, {}) caused overflow or underflow.\n",
                fmt_f64_2(base),
                fmt_f64_2(exponent)
            )
            .as_bytes(),
        );
        std::process::exit(1);
    }

    write_stdout(format!("Result: {}\n", fmt_f64_2(result)).as_bytes());
    std::process::exit(0);
}
