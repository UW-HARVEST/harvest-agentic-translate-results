//! Differential-test runner.
//!
//! Loads a shared object (either `target/cbuild/libc_driver.so`, built from
//! `c_src/src/main.c`, or `target/debug/libdriver.so`, built from the Rust
//! translation) with `libloading` and invokes its **exported C symbols**. It is
//! never linked against either library, so both implementations are exercised
//! exactly the way an external caller would use them.
//!
//! Usage:
//!   so_runner main        <so>                  # call `int main(void)`; stdin/stdout inherited
//!   so_runner driver      <so> <casefile>       # call `void driver(s1, s2)` once per case line
//!   so_runner driver-null <so> <s1|s2|both> <hex>   # call `driver` with NULL pointer(s)
//!   so_runner driver-bogus <so> <s1|s2|both> <hex>  # ... with an invalid (non-NULL) pointer
//!
//! Set `RUNNER_RESET_SIGPIPE=1` to restore the C default `SIGPIPE` disposition
//! before calling into the library (a Rust host process ignores `SIGPIPE`).
//!
//! `casefile` format: one case per line, `<hex_s1> <hex_s2>`, where a lone `.`
//! means the empty string. The decoded bytes are copied into a heap buffer with
//! an appended NUL, so arbitrary bytes (including interior NULs) can be passed.
//!
//! The runner itself writes nothing to stdout: stdout contains only what the
//! loaded library printed, so the two runs can be compared byte for byte.

use std::os::raw::{c_char, c_int};
use std::process::ExitCode;

type DriverFn = unsafe extern "C" fn(*const c_char, *const c_char);
type MainFn = unsafe extern "C" fn() -> c_int;

fn hex_decode(s: &str) -> Vec<u8> {
    if s == "." {
        return Vec::new();
    }
    let b = s.as_bytes();
    assert!(b.len() % 2 == 0, "odd hex length in {s:?}");
    let mut out = Vec::with_capacity(b.len() / 2);
    let val = |c: u8| -> u8 {
        match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            b'A'..=b'F' => c - b'A' + 10,
            _ => panic!("bad hex digit {c:?}"),
        }
    };
    for pair in b.chunks(2) {
        out.push((val(pair[0]) << 4) | val(pair[1]));
    }
    out
}

/// NUL-terminated heap copy, returned as (buffer, pointer to its first byte).
fn cstring_buf(bytes: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(bytes.len() + 1);
    v.extend_from_slice(bytes);
    v.push(0);
    v
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: {} <main|driver|driver-null> <so> [...]", args[0]);
        return ExitCode::from(2);
    }
    let mode = args[1].clone();
    let so_path = args[2].clone();

    // A Rust binary starts with SIGPIPE set to SIG_IGN, a C program does not.
    // When the differential test wants to observe the C behaviour of a broken
    // stdout pipe it asks the runner to restore the default disposition first,
    // so that both loaded libraries see the very same environment.
    if std::env::var_os("RUNNER_RESET_SIGPIPE").is_some() {
        const SIGPIPE: i32 = 13;
        extern "C" {
            fn signal(signum: i32, handler: usize) -> usize;
        }
        unsafe { signal(SIGPIPE, 0) };
    }

    // SAFETY: loading a trusted, locally built shared object.
    let lib = unsafe {
        match libloading::Library::new(&so_path) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("failed to load {so_path}: {e}");
                return ExitCode::from(3);
            }
        }
    };

    match mode.as_str() {
        "main" => {
            let f: libloading::Symbol<MainFn> = unsafe {
                match lib.get(b"main\0") {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("no `main` symbol in {so_path}: {e}");
                        return ExitCode::from(4);
                    }
                }
            };
            let rc = unsafe { f() };
            // Exit through libc so that the C library's buffered stdout is
            // flushed by the normal atexit handlers.
            std::process::exit(rc);
        }
        "main-repeat" => {
            // Call the exported `main` N times in the *same* process: the C
            // library keeps whatever its FILE* stdin buffered between calls, so
            // the Rust translation must do the same.
            let f: libloading::Symbol<MainFn> = unsafe {
                match lib.get(b"main\0") {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("no `main` symbol in {so_path}: {e}");
                        return ExitCode::from(4);
                    }
                }
            };
            let n: usize = args
                .get(3)
                .map(|s| s.parse().expect("bad repeat count"))
                .unwrap_or(2);
            let mut rc = 0;
            for _ in 0..n {
                rc = unsafe { f() };
            }
            std::process::exit(rc);
        }
        "driver" => {
            let f: libloading::Symbol<DriverFn> = unsafe {
                match lib.get(b"driver\0") {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("no `driver` symbol in {so_path}: {e}");
                        return ExitCode::from(4);
                    }
                }
            };
            let casefile = args.get(3).expect("missing casefile argument");
            let text = std::fs::read_to_string(casefile).expect("cannot read casefile");
            for (lineno, line) in text.lines().enumerate() {
                let line = line.trim_end();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let mut it = line.split_whitespace();
                let h1 = it.next().unwrap_or_else(|| panic!("line {lineno}: missing s1"));
                let h2 = it.next().unwrap_or_else(|| panic!("line {lineno}: missing s2"));
                let b1 = cstring_buf(&hex_decode(h1));
                let b2 = cstring_buf(&hex_decode(h2));
                unsafe { f(b1.as_ptr() as *const c_char, b2.as_ptr() as *const c_char) };
            }
            std::process::exit(0);
        }
        "driver-null" => {
            let f: libloading::Symbol<DriverFn> = unsafe {
                match lib.get(b"driver\0") {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("no `driver` symbol in {so_path}: {e}");
                        return ExitCode::from(4);
                    }
                }
            };
            let which = args.get(3).expect("missing which argument").clone();
            let other = cstring_buf(&hex_decode(args.get(4).map(|s| s.as_str()).unwrap_or(".")));
            let p = other.as_ptr() as *const c_char;
            let null = std::ptr::null::<c_char>();
            unsafe {
                match which.as_str() {
                    "s1" => f(null, p),
                    "s2" => f(p, null),
                    "both" => f(null, null),
                    other => panic!("bad which: {other}"),
                }
            }
            std::process::exit(0);
        }
        "driver-bogus" => {
            // Non-NULL but invalid pointer (address 1): C's strcspn faults, and
            // so must the Rust translation.
            let f: libloading::Symbol<DriverFn> = unsafe {
                match lib.get(b"driver\0") {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("no `driver` symbol in {so_path}: {e}");
                        return ExitCode::from(4);
                    }
                }
            };
            let which = args.get(3).expect("missing which argument").clone();
            let other = cstring_buf(&hex_decode(args.get(4).map(|s| s.as_str()).unwrap_or(".")));
            let p = other.as_ptr() as *const c_char;
            let bogus = 1usize as *const c_char;
            unsafe {
                match which.as_str() {
                    "s1" => f(bogus, p),
                    "s2" => f(p, bogus),
                    "both" => f(bogus, bogus),
                    other => panic!("bad which: {other}"),
                }
            }
            std::process::exit(0);
        }
        other => {
            eprintln!("unknown mode: {other}");
            ExitCode::from(2)
        }
    }
}
