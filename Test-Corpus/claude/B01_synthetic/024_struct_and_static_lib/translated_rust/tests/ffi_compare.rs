// Integration tests that load both the C and Rust shared libraries via
// libloading and compare their outputs byte-for-byte.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("libdriver.so");
    p
}

fn rust_lib_path() -> PathBuf {
    // CARGO_TARGET_TMPDIR-aware path — fall back to known target/debug location.
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    // Build/test profile may differ; check both.
    let candidates = ["debug", "release"];
    for prof in &candidates {
        let mut q = p.clone();
        q.push(prof);
        q.push("libdriver.so");
        if q.exists() {
            return q;
        }
    }
    p.push("debug");
    p.push("libdriver.so");
    p
}

/// Redirect libc stdout (fd 1) to a temp file while running `f`.
/// Returns the captured bytes.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // libc's stdout buffers, so we must flush before swapping fds.
    unsafe {
        let s = libc_stdout();
        libc_fflush(s);
    }

    let tmp = tempfile();
    let saved_fd = unsafe { libc_dup(1) };
    assert!(saved_fd >= 0, "dup failed");
    let new_fd = tmp.as_raw_fd();
    let r = unsafe { libc_dup2(new_fd, 1) };
    assert!(r >= 0, "dup2 failed");

    f();

    unsafe {
        let s = libc_stdout();
        libc_fflush(s);
    }

    let r = unsafe { libc_dup2(saved_fd, 1) };
    assert!(r >= 0, "dup2 restore failed");
    unsafe {
        libc_close(saved_fd);
    }

    let mut file = tmp;
    file.seek(SeekFrom::Start(0)).expect("seek");
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).expect("read");
    buf
}

fn tempfile() -> File {
    // Use tmpfile(3) so the file is automatically cleaned up.
    unsafe {
        let fp = libc_tmpfile();
        assert!(!fp.is_null(), "tmpfile failed");
        let fd = libc_fileno(fp);
        assert!(fd >= 0);
        // Duplicate fd into a Rust-owned File to manage lifetime cleanly.
        let dup_fd = libc_dup(fd);
        assert!(dup_fd >= 0);
        libc_fclose(fp);
        use std::os::unix::io::FromRawFd;
        File::from_raw_fd(dup_fd)
    }
}

// libc bindings — kept local to avoid pulling in the libc crate.
#[link(name = "c")]
unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn tmpfile() -> *mut std::ffi::c_void;
    fn fileno(stream: *mut std::ffi::c_void) -> c_int;
    fn fclose(stream: *mut std::ffi::c_void) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

// stdout symbol — Linux glibc exposes `stdout` as a global pointer to FILE.
unsafe extern "C" {
    static stdout: *mut std::ffi::c_void;
}

unsafe fn libc_dup(fd: c_int) -> c_int {
    unsafe { dup(fd) }
}
unsafe fn libc_dup2(a: c_int, b: c_int) -> c_int {
    unsafe { dup2(a, b) }
}
unsafe fn libc_close(fd: c_int) -> c_int {
    unsafe { close(fd) }
}
unsafe fn libc_tmpfile() -> *mut std::ffi::c_void {
    unsafe { tmpfile() }
}
unsafe fn libc_fileno(s: *mut std::ffi::c_void) -> c_int {
    unsafe { fileno(s) }
}
unsafe fn libc_fclose(s: *mut std::ffi::c_void) -> c_int {
    unsafe { fclose(s) }
}
unsafe fn libc_fflush(s: *mut std::ffi::c_void) -> c_int {
    unsafe { fflush(s) }
}
unsafe fn libc_stdout() -> *mut std::ffi::c_void {
    unsafe { stdout }
}

fn load_c() -> Library {
    let path = c_lib_path();
    assert!(path.exists(), "C .so not built at {:?}", path);
    unsafe { Library::new(path).expect("load C lib") }
}

fn load_rust() -> Library {
    let path = rust_lib_path();
    assert!(path.exists(), "Rust .so not built at {:?}", path);
    unsafe { Library::new(path).expect("load Rust lib") }
}

/// Call `driver(x)` in the given library, capturing its stdout.
fn call_driver(lib: &Library, x: c_int) -> Vec<u8> {
    capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int)> =
            lib.get(b"driver").expect("driver symbol");
        f(x);
    })
}

/// Call `run(x)` in the given library, capturing its stdout.
fn call_run(lib: &Library, x: c_int) -> Vec<u8> {
    capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int)> =
            lib.get(b"run").expect("run symbol");
        f(x);
    })
}

fn assert_outputs_match(label: &str, c_out: &[u8], r_out: &[u8]) {
    if c_out != r_out {
        eprintln!("=== {label} ===");
        eprintln!("--- C output ({} bytes) ---", c_out.len());
        eprintln!("{}", String::from_utf8_lossy(c_out));
        eprintln!("--- Rust output ({} bytes) ---", r_out.len());
        eprintln!("{}", String::from_utf8_lossy(r_out));
        panic!("Outputs differ for {}", label);
    }
}

#[test]
fn driver_matches_c_for_various_inputs() {
    // Each library has its own internal state; loading a fresh copy resets it.
    // libloading caches handles per path within a process, so we ensure each
    // test loads the library fresh and drops the handle when it goes out of
    // scope.
    let inputs = [0i32, 1, 2, 3, 7, -1, -5, 100, -100, i32::MAX, i32::MIN];

    for &x in &inputs {
        let c = load_c();
        let r = load_rust();
        let c_out = call_driver(&c, x);
        let r_out = call_driver(&r, x);
        assert_outputs_match(&format!("driver({})", x), &c_out, &r_out);
        drop(c);
        drop(r);
    }
}

#[test]
fn run_matches_c_for_various_inputs() {
    let inputs = [0i32, 1, 2, 3, 7, -1, -5, 100, -100, i32::MAX, i32::MIN];

    for &x in &inputs {
        let c = load_c();
        let r = load_rust();
        let c_out = call_run(&c, x);
        let r_out = call_run(&r, x);
        assert_outputs_match(&format!("run({})", x), &c_out, &r_out);
        drop(c);
        drop(r);
    }
}

#[test]
fn run_state_persists_across_calls() {
    // Verify state mutation matches across multiple calls within one library load.
    let c = load_c();
    let r = load_rust();
    for x in [1, 2, 3, 4, 5] {
        let c_out = call_run(&c, x);
        let r_out = call_run(&r, x);
        assert_outputs_match(&format!("run({}) sequential", x), &c_out, &r_out);
    }
}

#[test]
fn driver_then_run_matches() {
    // Mix calls to driver and run; both implementations should mutate
    // the_house identically.
    let c = load_c();
    let r = load_rust();
    let calls: &[(&str, i32)] = &[
        ("driver", 1),
        ("run", 2),
        ("driver", -3),
        ("run", 0),
        ("driver", 10),
    ];
    for (which, x) in calls {
        let (c_out, r_out) = if *which == "driver" {
            (call_driver(&c, *x), call_driver(&r, *x))
        } else {
            (call_run(&c, *x), call_run(&r, *x))
        };
        assert_outputs_match(&format!("{}({})", which, x), &c_out, &r_out);
    }
}
