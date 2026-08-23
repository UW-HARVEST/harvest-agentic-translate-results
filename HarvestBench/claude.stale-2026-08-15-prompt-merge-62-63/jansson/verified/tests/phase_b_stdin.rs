//! Phase B — CONFIGS.md rows 126 and 128: the `<stdin>` source branch.
//!
//! `load.c:973-974` stamps the error source as `"<stdin>"` (instead of
//! `"<stream>"`) when `json_loadf` is handed `stdin`, and `load.c:1008` does the
//! same when `json_loadfd` is handed `STDIN_FILENO`. That branch is invisible to
//! every other test, because nothing else passes fd 0.
//!
//! Reading real stdin under `cargo test` would block, so we temporarily point
//! fd 0 at a temp file with `dup2`, run the comparison, then restore fd 0. Both
//! libraries are driven through the same redirected fd, one after the other.

mod common;

use common::*;
use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::io::Write;
use std::os::unix::io::AsRawFd;

/// Redirecting fd 0 is PROCESS-GLOBAL state. libtest runs `#[test]` fns on
/// parallel threads, so these tests must not overlap with each other.
static FD0_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn fd0_guard() -> std::sync::MutexGuard<'static, ()> {
    FD0_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
}

type FnLoadf = unsafe extern "C" fn(*mut c_void, usize, *mut json_error_t) -> *mut json_t;
type FnLoadfd = unsafe extern "C" fn(c_int, usize, *mut json_error_t) -> *mut json_t;

/// Write `content` to a temp file and return it (kept alive by the caller).
fn temp_with(content: &[u8], tag: &str) -> (std::fs::File, std::path::PathBuf) {
    let p = std::env::temp_dir().join(format!("jansson_stdin_{}_{}.json", tag, std::process::id()));
    let mut f = std::fs::File::create(&p).expect("create temp");
    f.write_all(content).expect("write temp");
    f.sync_all().ok();
    drop(f);
    let f = std::fs::File::open(&p).expect("reopen temp");
    (f, p)
}

/// Redirect fd 0 to `file`, run `f`, then restore fd 0.
unsafe fn with_stdin_as<T>(file: &std::fs::File, f: impl FnOnce() -> T) -> T {
    let saved = dup(0);
    assert!(saved >= 0, "dup(0) failed");
    lseek(file.as_raw_fd(), 0, 0 /* SEEK_SET */);
    assert!(dup2(file.as_raw_fd(), 0) >= 0, "dup2 onto fd 0 failed");
    let out = f();
    dup2(saved, 0);
    close(saved);
    out
}

#[test]
fn row128_json_loadfd_stdin_source() {
    let _guard = fd0_guard();
    let l = libs();
    let doc = br#"{"from":"stdin","n":[1,2,3]}"#;

    let run = |lib: &Library| unsafe {
        let (file, path) = temp_with(doc, "fd");
        let out = with_stdin_as(&file, || {
            let f: Symbol<FnLoadfd> = sym(lib, "json_loadfd");
            let mut err = json_error_t::new();
            // fd 0 == STDIN_FILENO -> the `<stdin>` source branch
            let j = f(0, 0, &mut err);
            let dump = if j.is_null() { None } else { dumps_to_string(lib, j, JSON_SORT_KEYS) };
            if !j.is_null() {
                decref(lib, j);
            }
            (dump, err.snapshot())
        });
        drop(file);
        let _ = std::fs::remove_file(&path);
        out
    };

    let cv = run(&l.c);
    let rv = run(&l.r);
    assert_eq!(cv, rv, "row128: json_loadfd(STDIN_FILENO) diverged");
    // Pin the actual C behavior so this cannot pass vacuously.
    assert_eq!(cv.1.source, "<stdin>", "C must report source `<stdin>` for fd 0");
    assert_eq!(cv.0.as_deref(), Some(r#"{"from": "stdin", "n": [1, 2, 3]}"#));
}

#[test]
fn row126_json_loadf_stdin_source() {
    let _guard = fd0_guard();
    let l = libs();
    let doc = br#"[10,20,{"k":true}]"#;

    let run = |lib: &Library| unsafe {
        let (file, path) = temp_with(doc, "f");
        let out = with_stdin_as(&file, || {
            // Each library has its OWN libc `stdin` FILE* only if it links its
            // own libc copy; in practice both resolve the same glibc `stdin`.
            // Fetch it fresh per library and rewind so the two runs see the same
            // bytes.
            let stdin_ptr: *mut c_void = {
                extern "C" {
                    static mut stdin: *mut c_void;
                }
                let p = std::ptr::addr_of!(stdin).read();
                // Clear any buffered state / EOF flag from a previous read.
                extern "C" {
                    fn clearerr(f: *mut c_void);
                    fn fseek(f: *mut c_void, off: i64, whence: c_int) -> c_int;
                }
                clearerr(p);
                fseek(p, 0, 0);
                p
            };
            let f: Symbol<FnLoadf> = sym(lib, "json_loadf");
            let mut err = json_error_t::new();
            let j = f(stdin_ptr, 0, &mut err);
            let dump = if j.is_null() { None } else { dumps_to_string(lib, j, 0) };
            if !j.is_null() {
                decref(lib, j);
            }
            (dump, err.snapshot())
        });
        drop(file);
        let _ = std::fs::remove_file(&path);
        out
    };

    let cv = run(&l.c);
    let rv = run(&l.r);
    assert_eq!(cv, rv, "row126: json_loadf(stdin) diverged");
    // The whole point of the row: the source must be `<stdin>`, not `<stream>`.
    assert_eq!(cv.1.source, "<stdin>", "C must report source `<stdin>` for the stdin FILE*");
}

#[test]
fn row125_127_json_loadf_and_loadfd_regular_file_source() {
    let _guard = fd0_guard();
    // Contrast case: a NON-stdin stream/fd must report `<stream>`, proving the
    // `<stdin>` result above is a real branch and not the default.
    let doc = br#"{"regular":1}"#;
    let (file, path) = temp_with(doc, "regular");
    let fd = file.as_raw_fd();

    diff("rows125/127 regular file source", move |lib: &Library| unsafe {
        lseek(fd, 0, 0);
        let f: Symbol<FnLoadfd> = sym(lib, "json_loadfd");
        let mut err = json_error_t::new();
        let j = f(fd, 0, &mut err);
        let dump = if j.is_null() { None } else { dumps_to_string(lib, j, 0) };
        if !j.is_null() {
            decref(lib, j);
        }
        (dump, err.snapshot())
    });

    let l = libs();
    unsafe {
        lseek(fd, 0, 0);
        let f: Symbol<FnLoadfd> = sym(&l.c, "json_loadfd");
        let mut err = json_error_t::new();
        let j = f(fd, 0, &mut err);
        assert_eq!(err.source_str(), "<stream>", "a regular fd must report `<stream>`");
        if !j.is_null() {
            decref(&l.c, j);
        }
    }
    drop(file);
    let _ = std::fs::remove_file(&path);
}
