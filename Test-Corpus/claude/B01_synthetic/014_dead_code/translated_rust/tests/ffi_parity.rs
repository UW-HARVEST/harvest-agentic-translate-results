// Integration tests that load both the reference C shared library and the
// Rust-translated cdylib through `libloading` and verify byte-identical
// stdout output for every exported symbol.
//
// We never invoke the Rust functions through their Rust ABI — we always
// reach them through the dynamic library so the `#[no_mangle]` exports are
// exercised exactly the way an external C caller would use them.

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::io::{Read, Write};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::sync::Mutex;

// stdout-capturing utilities are not thread-safe (they manipulate process-wide
// fd 1), so we serialize all tests behind a single mutex.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn project_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at translated_rust/
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_lib_path() -> PathBuf {
    // `cargo test` builds in target/debug/ by default. The cdylib lives next
    // to the test binary as libdriver.so on Linux.
    let mut p = project_root();
    p.push("target");
    p.push("debug");
    p.push("libdriver.so");
    p
}

fn c_lib_path() -> PathBuf {
    let mut p = project_root();
    p.push("c_src");
    p.push("build");
    p.push("libdriver_c.so");
    p
}

/// Run `f`, capturing whatever it writes to fd 1 (stdout) and returning the
/// captured bytes. Flushes Rust's stdout buffer and uses `dup2` to redirect
/// the underlying file descriptor so writes from C `printf` are also caught.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd};

    // Make sure both the Rust side and any libc buffering is drained before
    // we swap descriptors out from under them.
    let _ = std::io::stdout().flush();
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }

    let mut fds = [0 as libc::c_int; 2];
    let r = unsafe { libc::pipe(fds.as_mut_ptr()) };
    assert_eq!(r, 0, "pipe() failed");
    let read_fd = fds[0];
    let write_fd = fds[1];

    let stdout_fd = std::io::stdout().as_raw_fd();
    let saved_fd = unsafe { libc::dup(stdout_fd) };
    assert!(saved_fd >= 0, "dup() failed");

    let r = unsafe { libc::dup2(write_fd, stdout_fd) };
    assert!(r >= 0, "dup2() failed");
    unsafe {
        libc::close(write_fd);
    }

    f();

    // Drain everything the function wrote.
    let _ = std::io::stdout().flush();
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }

    // Restore original stdout.
    let r = unsafe { libc::dup2(saved_fd, stdout_fd) };
    assert!(r >= 0, "dup2(restore) failed");
    unsafe {
        libc::close(saved_fd);
    }

    let mut reader = unsafe { std::fs::File::from_raw_fd(read_fd) };
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).expect("read pipe");
    let _ = reader.into_raw_fd(); // closed by the File drop normally; consume

    buf
}

fn load_c() -> Library {
    unsafe { Library::new(c_lib_path()) }.expect("loading libdriver_c.so")
}
fn load_rust() -> Library {
    unsafe { Library::new(rust_lib_path()) }.expect("loading libdriver.so")
}

type FnVoid = unsafe extern "C" fn();
type FnPrintLine = unsafe extern "C" fn(*const c_char);
type FnMain = unsafe extern "C" fn(c_int, *const *const c_char) -> c_int;

#[test]
fn print_line_null_pointer_matches() {
    let _g = STDOUT_LOCK.lock().unwrap();
    let c = load_c();
    let r = load_rust();

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<FnPrintLine> = c.get(b"printLine").unwrap();
        f(std::ptr::null());
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<FnPrintLine> = r.get(b"printLine").unwrap();
        f(std::ptr::null());
    });
    assert_eq!(c_out, r_out, "printLine(NULL) outputs differ");
    assert!(c_out.is_empty(), "expected empty output for NULL");
}

#[test]
fn print_line_basic_strings_match() {
    let _g = STDOUT_LOCK.lock().unwrap();
    let c = load_c();
    let r = load_rust();

    let cases: &[&[u8]] = &[
        b"",
        b"hello",
        b"a\tb",
        b"line with spaces",
        b"unicode \xe2\x9c\x94 check",
        b"a really long string ----------------------------------------",
    ];

    for case in cases {
        let cstr = CString::new(*case).expect("CString from input");
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<FnPrintLine> = c.get(b"printLine").unwrap();
            f(cstr.as_ptr());
        });
        let r_out = capture_stdout(|| unsafe {
            let f: Symbol<FnPrintLine> = r.get(b"printLine").unwrap();
            f(cstr.as_ptr());
        });
        assert_eq!(
            c_out, r_out,
            "printLine differs for input {:?}",
            String::from_utf8_lossy(case)
        );
    }
}

#[test]
fn good_matches() {
    let _g = STDOUT_LOCK.lock().unwrap();
    let c = load_c();
    let r = load_rust();

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<FnVoid> = c.get(b"good").unwrap();
        f();
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<FnVoid> = r.get(b"good").unwrap();
        f();
    });
    assert_eq!(c_out, r_out, "good() outputs differ");
    assert_eq!(c_out, b"good()\nhelperGood()\n");
}

#[test]
fn bad_matches() {
    let _g = STDOUT_LOCK.lock().unwrap();
    let c = load_c();
    let r = load_rust();

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<FnVoid> = c.get(b"bad").unwrap();
        f();
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<FnVoid> = r.get(b"bad").unwrap();
        f();
    });
    assert_eq!(c_out, r_out, "bad() outputs differ");
    assert_eq!(c_out, b"bad()\n");
}

#[test]
fn main_matches() {
    let _g = STDOUT_LOCK.lock().unwrap();
    let c = load_c();
    let r = load_rust();

    // Match how the executable is normally invoked: argc == 1, argv[0] is
    // a program name, argv[1] is NULL.
    let prog = CString::new("driver").unwrap();
    let argv = [prog.as_ptr(), std::ptr::null()];

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<FnMain> = c.get(b"main").unwrap();
        let rc = f(1, argv.as_ptr());
        assert_eq!(rc, 0, "C main returned non-zero");
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<FnMain> = r.get(b"main").unwrap();
        let rc = f(1, argv.as_ptr());
        assert_eq!(rc, 0, "Rust main returned non-zero");
    });
    assert_eq!(c_out, r_out, "main() outputs differ");
    let expected: &[u8] = b"Calling good()...\ngood()\nhelperGood()\nFinished good()\nCalling bad()...\nbad()\nFinished bad()\n";
    assert_eq!(c_out, expected);
}
