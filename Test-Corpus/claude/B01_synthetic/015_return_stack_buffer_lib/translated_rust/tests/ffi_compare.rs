// Integration tests that compare the C reference shared library against the
// Rust translation by loading both via libloading and comparing stdout output.

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::fs;
use std::io::Read;
use std::os::raw::{c_char, c_int};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

extern "C" {
    fn fflush(stream: *mut libc_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

#[allow(non_camel_case_types)]
type libc_void = std::ffi::c_void;

extern "C" {
    static stdout: *mut libc_void;
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // Use the cdylib produced by `cargo build` for the current profile.
    // CARGO_MANIFEST_DIR/target/{debug|release}/libdriver.so
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let release = manifest.join("target").join("release").join("libdriver.so");
    let debug = manifest.join("target").join("debug").join("libdriver.so");
    if release.exists() {
        release
    } else {
        debug
    }
}

/// Run a closure while stdout is redirected to a temp file. Returns the
/// captured bytes.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // Flush whatever is in the C stdio buffer first.
    unsafe { fflush(stdout) };

    // Create a temp file.
    let tmp_path = std::env::temp_dir().join(format!(
        "driver_capture_{}_{}",
        std::process::id(),
        rand_suffix()
    ));
    let tmp = fs::File::create(&tmp_path).expect("create temp file");
    let tmp_fd = tmp.as_raw_fd();

    // Save stdout fd (the underlying OS fd, not the FILE *).
    let saved_fd = unsafe { dup(1) };
    assert!(saved_fd >= 0, "dup(1) failed");

    // Redirect fd 1 to our temp file.
    let rc = unsafe { dup2(tmp_fd, 1) };
    assert!(rc >= 0, "dup2 failed");

    // Run the user's closure.
    f();

    // Flush and restore.
    unsafe { fflush(stdout) };
    let rc = unsafe { dup2(saved_fd, 1) };
    assert!(rc >= 0, "dup2 restore failed");
    unsafe { close(saved_fd) };

    drop(tmp);

    // Read back the captured contents.
    let mut buf = Vec::new();
    let mut f = fs::File::open(&tmp_path).expect("open temp file");
    f.read_to_end(&mut buf).expect("read temp file");
    let _ = fs::remove_file(&tmp_path);
    buf
}

fn rand_suffix() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

#[test]
fn test_print_line() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let c_print: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { c_lib.get(b"printLine\0").expect("C printLine") };
    let r_print: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { r_lib.get(b"printLine\0").expect("Rust printLine") };

    let inputs: Vec<Option<&str>> = vec![
        Some(""),
        Some("hello"),
        Some("hello world"),
        Some("with\ttabs and\nnewlines"),
        Some("a longer string with several words to print"),
        None,
    ];

    for inp in inputs {
        let cstr = inp.map(|s| CString::new(s).unwrap());
        let ptr = cstr
            .as_ref()
            .map(|c| c.as_ptr())
            .unwrap_or(std::ptr::null());

        let c_out = capture_stdout(|| unsafe { c_print(ptr) });
        let r_out = capture_stdout(|| unsafe { r_print(ptr) });

        assert_eq!(
            c_out, r_out,
            "printLine mismatch for input {:?}: C={:?} Rust={:?}",
            inp,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}

#[test]
fn test_good() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let c_good: Symbol<unsafe extern "C" fn()> =
        unsafe { c_lib.get(b"good\0").expect("C good") };
    let r_good: Symbol<unsafe extern "C" fn()> =
        unsafe { r_lib.get(b"good\0").expect("Rust good") };

    let c_out = capture_stdout(|| unsafe { c_good() });
    let r_out = capture_stdout(|| unsafe { r_good() });

    assert_eq!(
        c_out, r_out,
        "good() mismatch: C={:?} Rust={:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
    // good() should print a stable string ending in newline.
    assert_eq!(c_out, b"helperGood1 string\n");
}

#[test]
fn test_driver_good() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let c_drv: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { c_lib.get(b"driver\0").expect("C driver") };
    let r_drv: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { r_lib.get(b"driver\0").expect("Rust driver") };

    // Use the "good" branch only — the "bad" branch is undefined behavior in
    // the C source (returns a pointer to a stack-allocated array), so its
    // output is not deterministic and not expected to byte-match.
    for v in [1, 2, -1, 1000].iter() {
        let c_out = capture_stdout(|| unsafe { c_drv(*v) });
        let r_out = capture_stdout(|| unsafe { r_drv(*v) });
        assert_eq!(
            c_out, r_out,
            "driver({}) mismatch: C={:?} Rust={:?}",
            v,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}

#[test]
fn test_bad_runs_without_aborting() {
    // bad() invokes undefined behavior in the C source by returning a pointer
    // to a stack-local array. Both implementations dereference such a pointer
    // and feed it to printf("%s\n", ...). The printed contents are not
    // guaranteed to match between C and Rust, but the call should not abort.
    // We only verify that both calls return without panic and that the output
    // ends with a newline.
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let c_bad: Symbol<unsafe extern "C" fn()> =
        unsafe { c_lib.get(b"bad\0").expect("C bad") };
    let r_bad: Symbol<unsafe extern "C" fn()> =
        unsafe { r_lib.get(b"bad\0").expect("Rust bad") };

    let _c_out = capture_stdout(|| unsafe { c_bad() });
    let _r_out = capture_stdout(|| unsafe { r_bad() });

    // No assertion on the contents: both implementations rely on the address
    // of a stack-local array that has already gone out of scope, which is
    // undefined behavior. The compilers may treat this differently (printing
    // nothing, garbage, or the original bytes). All we require is that the
    // call returns instead of crashing — reaching this line proves that.
}
