// Integration tests comparing C and Rust implementations through FFI.
// Loads both libdriver.so (C) and our compiled cdylib via libloading,
// captures stdout for each call, and compares byte-for-byte.

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::io::Read;
use std::os::raw::{c_char, c_int};
use std::os::unix::io::FromRawFd;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // The integration test runs after the cdylib is built (we ensure that
    // out-of-band by running `cargo build --release` first), but cargo also
    // builds the cdylib for the test target. Look in both release and debug
    // build directories.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest.join("target/release/libdriver.so"),
        manifest.join("target/debug/libdriver.so"),
    ];
    for c in candidates.iter() {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "Could not locate libdriver.so under target/. Tried: {:?}",
        candidates
    );
}

/// Capture everything that gets written to stdout (file descriptor 1) by the
/// closure, including writes performed by C `printf`.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // Make sure the libc-side stdout buffer is empty before redirecting.
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }

    let saved = unsafe { libc::dup(1) };
    assert!(saved >= 0, "dup(1) failed");

    let mut fds = [0 as libc::c_int; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe failed");
    let (read_end, write_end) = (fds[0], fds[1]);

    // Redirect stdout to the pipe's write end.
    let rc = unsafe { libc::dup2(write_end, 1) };
    assert!(rc >= 0, "dup2 failed");
    unsafe {
        libc::close(write_end);
    }

    // Drain the pipe in a background thread so a large write doesn't block.
    let mut reader = unsafe { std::fs::File::from_raw_fd(read_end) };
    let handle = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = reader.read_to_end(&mut buf);
        buf
    });

    f();

    // Flush libc stdout before restoring fd.
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }

    // Restore original stdout. This also closes the pipe's writer (fd 1
    // was the only remaining ref), which lets the reader thread finish.
    let rc = unsafe { libc::dup2(saved, 1) };
    assert!(rc >= 0, "restore dup2 failed");
    unsafe {
        libc::close(saved);
    }

    handle.join().expect("reader thread panicked")
}

fn load_libs() -> (Library, Library) {
    let c_path = c_lib_path();
    let r_path = rust_lib_path();
    assert!(c_path.exists(), "C .so missing at {:?}", c_path);
    assert!(r_path.exists(), "Rust .so missing at {:?}", r_path);
    let c_lib = unsafe { Library::new(&c_path) }.expect("load C .so");
    let r_lib = unsafe { Library::new(&r_path) }.expect("load Rust .so");
    (c_lib, r_lib)
}

#[test]
fn print_line_matches() {
    let (c_lib, r_lib) = load_libs();
    let inputs: &[Option<&str>] = &[
        Some(""),
        Some("hello"),
        Some("a longer line with spaces and 123"),
        Some("multi\nline\ttext"),
        None,
    ];

    for inp in inputs {
        let cstr = inp.map(|s| CString::new(s).unwrap());
        let ptr: *const c_char = match &cstr {
            Some(c) => c.as_ptr(),
            None => std::ptr::null(),
        };

        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(*const c_char)> =
                c_lib.get(b"printLine\0").unwrap();
            f(ptr);
        });
        let r_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(*const c_char)> =
                r_lib.get(b"printLine\0").unwrap();
            f(ptr);
        });
        assert_eq!(c_out, r_out, "printLine output mismatch for {:?}", inp);
    }
}

#[test]
fn print_int_line_matches() {
    let (c_lib, r_lib) = load_libs();
    let inputs: &[c_int] = &[0, 1, -1, 42, -42, i32::MAX, i32::MIN, 1234567];

    for &n in inputs {
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                c_lib.get(b"printIntLine\0").unwrap();
            f(n);
        });
        let r_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                r_lib.get(b"printIntLine\0").unwrap();
            f(n);
        });
        assert_eq!(c_out, r_out, "printIntLine output mismatch for {}", n);
    }
}

#[test]
fn bad_matches() {
    let (c_lib, r_lib) = load_libs();
    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = c_lib.get(b"bad\0").unwrap();
        f();
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = r_lib.get(b"bad\0").unwrap();
        f();
    });
    assert_eq!(c_out, r_out, "bad() output mismatch");
}

#[test]
fn good_matches() {
    let (c_lib, r_lib) = load_libs();
    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = c_lib.get(b"good\0").unwrap();
        f();
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = r_lib.get(b"good\0").unwrap();
        f();
    });
    assert_eq!(c_out, r_out, "good() output mismatch");
}

#[test]
fn driver_matches() {
    let (c_lib, r_lib) = load_libs();
    for use_good in &[0 as c_int, 1, 2, -1] {
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                c_lib.get(b"driver\0").unwrap();
            f(*use_good);
        });
        let r_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                r_lib.get(b"driver\0").unwrap();
            f(*use_good);
        });
        assert_eq!(c_out, r_out, "driver({}) output mismatch", use_good);
    }
}

#[test]
fn rust_so_exports_all_c_symbols() {
    use std::process::Command;
    let c_path = c_lib_path();
    let r_path = rust_lib_path();
    let out_c = Command::new("nm")
        .args(["-D", "--defined-only", c_path.to_str().unwrap()])
        .output()
        .expect("run nm on C .so");
    let out_r = Command::new("nm")
        .args(["-D", "--defined-only", r_path.to_str().unwrap()])
        .output()
        .expect("run nm on Rust .so");
    assert!(out_c.status.success());
    assert!(out_r.status.success());

    fn parse(out: &[u8]) -> std::collections::BTreeSet<String> {
        let s = std::str::from_utf8(out).unwrap();
        let mut set = std::collections::BTreeSet::new();
        for line in s.lines() {
            // Format: "<addr> <type> <name>"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }
            let kind = parts[parts.len() - 2];
            // Only take real defined functions/data, skip weak/init/fini glue.
            if matches!(kind, "T" | "B" | "D" | "R") {
                let name = parts[parts.len() - 1];
                if !matches!(
                    name,
                    "_init" | "_fini" | "__bss_start" | "_edata" | "_end"
                ) {
                    set.insert(name.to_string());
                }
            }
        }
        set
    }

    let c_syms = parse(&out_c.stdout);
    let r_syms = parse(&out_r.stdout);
    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so missing exports present in C .so: {:?}",
        missing
    );
}
