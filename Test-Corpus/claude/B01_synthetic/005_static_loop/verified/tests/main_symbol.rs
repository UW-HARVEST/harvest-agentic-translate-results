// Integration test: compares the `main` symbol exported by the C and Rust .so
// files. We invoke `main(argc, argv)` directly via libloading and capture
// stdout to confirm that the C and Rust libraries produce byte-identical
// output and the same return code.
//
// stdout is redirected at the FD level (dup2 over fd 1) so it captures
// output from libc `printf` calls inside both libraries.

mod common;

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::raw::{c_char, c_int};
use std::os::unix::io::AsRawFd;

type MainFn = unsafe extern "C" fn(argc: c_int, argv: *mut *mut c_char) -> c_int;

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

fn run_main_capture(lib: &Library, args: &[&str]) -> (c_int, Vec<u8>) {
    unsafe {
        let main_fn: Symbol<MainFn> =
            lib.get(b"main\0").expect("library missing main symbol");

        // Build a heap-allocated argv array that lives until after the call.
        let cstrings: Vec<CString> = args
            .iter()
            .map(|a| CString::new(*a).expect("arg has interior NUL"))
            .collect();
        let mut ptrs: Vec<*mut c_char> = cstrings
            .iter()
            .map(|s| s.as_ptr() as *mut c_char)
            .collect();
        // C convention: argv has a trailing NULL, but the C program
        // doesn't read past argc; still, mirror the convention.
        ptrs.push(std::ptr::null_mut());

        // Capture stdout via tmpfile + dup2.
        // 1. Flush any pending Rust/C stdio buffers.
        fflush(std::ptr::null_mut());

        // 2. Save the current fd 1.
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0, "dup(1) failed");

        // 3. Open a tempfile and redirect fd 1 into it.
        let tmp = tempfile_rdwr();
        let tmp_fd = tmp.as_raw_fd();
        let r = dup2(tmp_fd, 1);
        assert!(r >= 0, "dup2 failed");

        // 4. Call main with argc = args.len(), argv = ptrs.
        let argc = args.len() as c_int;
        let argv = ptrs.as_mut_ptr();
        let rc = main_fn(argc, argv);

        // 5. Flush again to drain printf's stdio buffer into the tempfile.
        fflush(std::ptr::null_mut());

        // 6. Restore fd 1.
        let r = dup2(saved_stdout, 1);
        assert!(r >= 0, "dup2 restore failed");
        close(saved_stdout);

        // 7. Read everything that was written.
        let mut tmp = tmp;
        tmp.seek(SeekFrom::Start(0)).expect("seek tmp");
        let mut buf = Vec::new();
        tmp.read_to_end(&mut buf).expect("read tmp");

        (rc, buf)
    }
}

// Create an anonymous read/write tempfile.
fn tempfile_rdwr() -> File {
    // Use the platform's tmpfile equivalent without pulling in a crate.
    // We open a new file in /tmp that auto-removes on close (best effort).
    let path = std::env::temp_dir().join(format!(
        "driver-test-stdout-{}-{}",
        std::process::id(),
        rand_u64()
    ));
    let f = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("open tmp file");
    // Unlink immediately so the file is cleaned up when the FDs close.
    let _ = std::fs::remove_file(&path);
    f
}

fn rand_u64() -> u64 {
    // Simple non-crypto unique-ish value; combines time + counter.
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let c = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    nanos ^ c
}

fn assert_main_outputs_match(args: &[&str], label: &str) {
    // We must use a FRESH process load for each library so the static
    // state inside `static_sum` starts from zero. Cargo runs each #[test]
    // in the same process, so we serialize all sub-cases in one test
    // function and re-load the libraries between sub-cases.
    let c_lib = unsafe { Library::new(common::c_so_path()).expect("load C .so") };
    let r_lib = unsafe { Library::new(common::rust_so_path()).expect("load Rust .so") };

    let (c_rc, c_out) = run_main_capture(&c_lib, args);
    let (r_rc, r_out) = run_main_capture(&r_lib, args);

    assert_eq!(
        c_rc, r_rc,
        "[{label}] return code mismatch: C={c_rc}, Rust={r_rc}"
    );
    assert_eq!(
        c_out, r_out,
        "[{label}] stdout mismatch:\nC ({} bytes): {:?}\nRust ({} bytes): {:?}",
        c_out.len(),
        String::from_utf8_lossy(&c_out),
        r_out.len(),
        String::from_utf8_lossy(&r_out),
    );

    // Drop libraries; this also forces them to be re-loaded next call,
    // resetting each library's static state.
    drop(c_lib);
    drop(r_lib);
}

#[test]
fn main_symbol_matches_c() {
    // Canonical: stride = 3.
    assert_main_outputs_match(&["driver", "3"], "stride=3");

    // Negative stride.
    assert_main_outputs_match(&["driver", "-7"], "stride=-7");

    // Zero stride.
    assert_main_outputs_match(&["driver", "0"], "stride=0");

    // Stride with leading whitespace and sign — strtol skips whitespace.
    assert_main_outputs_match(&["driver", "  +5"], "stride=  +5");

    // Stride with trailing junk — strtol stops at first non-digit; the
    // C program only checks `end == argv[1]`, so it accepts the prefix.
    assert_main_outputs_match(&["driver", "12abc"], "stride=12abc");

    // Wrong argument count: should print error and return 1.
    assert_main_outputs_match(&["driver"], "no-args");
    assert_main_outputs_match(&["driver", "1", "2"], "too-many-args");

    // Non-integer argument: should print integer-error message and return 1.
    assert_main_outputs_match(&["driver", "abc"], "non-integer");
    assert_main_outputs_match(&["driver", ""], "empty-arg");
    assert_main_outputs_match(&["driver", "+"], "just-sign");
}
