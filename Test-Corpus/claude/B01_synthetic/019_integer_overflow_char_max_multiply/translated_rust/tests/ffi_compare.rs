// Integration tests that load BOTH the C shared library and the Rust shared
// library through libloading, invoke the same exported symbol with the same
// inputs in each library, and assert that the captured stdout output is
// byte-identical.
//
// Both libraries call into the libc printf/scanf machinery for output, so
// their stdout streams are routed through the same FILE* stdout. We capture
// stdout by dup2'ing it onto a temp file, calling the function, fflush'ing
// stdout, then restoring the original stdout fd and reading the temp file.

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::raw::{c_char, c_int};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::Mutex;

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
    static stdout: *mut std::ffi::c_void;
}

// Serialize all stdout-redirecting tests so they don't race each other.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    project_root().join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    // Built by `cargo build` (cdylib).
    let mut p = project_root().join("target");
    // Honor CARGO_TARGET_DIR if set.
    if let Ok(t) = std::env::var("CARGO_TARGET_DIR") {
        p = PathBuf::from(t);
    }
    p.push("debug");
    p.push("libtranslated_rust.so");
    p
}

fn load_c() -> Library {
    let path = c_so_path();
    assert!(
        path.exists(),
        "C shared library missing at {:?} – build it first",
        path
    );
    unsafe { Library::new(&path).expect("failed to load C .so") }
}

fn load_rust() -> Library {
    let path = rust_so_path();
    assert!(
        path.exists(),
        "Rust shared library missing at {:?} – run `cargo build` first",
        path
    );
    unsafe { Library::new(&path).expect("failed to load Rust .so") }
}

/// Run `f` with stdout redirected to a tmp file; return the captured bytes.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap();

    // Make a temp file we can dup2 onto fd 1.
    let mut tmp = tempfile();
    let tmp_fd = tmp.as_raw_fd();

    // Save original stdout fd.
    let saved_fd = unsafe { dup(1) };
    assert!(saved_fd >= 0, "dup(1) failed");

    // Make sure both libc and Rust have flushed everything before we redirect.
    let _ = std::io::stdout().flush();
    unsafe {
        fflush(stdout);
    }

    // Redirect stdout to tmp file.
    let r = unsafe { dup2(tmp_fd, 1) };
    assert!(r >= 0, "dup2 to tmp failed");

    // Run the work.
    f();

    // Flush before restoring.
    let _ = std::io::stdout().flush();
    unsafe {
        fflush(stdout);
    }

    // Restore original stdout.
    let r = unsafe { dup2(saved_fd, 1) };
    assert!(r >= 0, "dup2 restore failed");
    unsafe {
        close(saved_fd);
    }

    // Read captured bytes.
    tmp.seek(SeekFrom::Start(0)).unwrap();
    let mut out = Vec::new();
    tmp.read_to_end(&mut out).unwrap();
    out
}

fn tempfile() -> File {
    // Build a unique path in /tmp.
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p = std::env::temp_dir().join(format!("ffi_capture_{}_{}.txt", pid, nanos));
    let f = File::options()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&p)
        .unwrap();
    // Unlink so file is cleaned when fd is closed.
    let _ = std::fs::remove_file(&p);
    f
}

fn run_void_fn(lib: &Library, sym: &str) -> Vec<u8> {
    capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = lib.get(sym.as_bytes()).unwrap();
        f();
    })
}

fn run_print_hex_char(lib: &Library, value: c_char) -> Vec<u8> {
    capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(c_char)> = lib.get(b"printHexCharLine").unwrap();
        f(value);
    })
}

fn run_print_line(lib: &Library, s: Option<&CString>) -> Vec<u8> {
    capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = lib.get(b"printLine").unwrap();
        match s {
            Some(cs) => f(cs.as_ptr()),
            None => f(std::ptr::null()),
        }
    })
}

#[test]
fn print_line_null_pointer() {
    let c = load_c();
    let r = load_rust();
    let c_out = run_print_line(&c, None);
    let r_out = run_print_line(&r, None);
    assert_eq!(c_out, r_out, "printLine(NULL) outputs differ");
}

#[test]
fn print_line_basic_strings() {
    let c = load_c();
    let r = load_rust();
    for s in ["", "hello", "data value is too large to perform arithmetic safely."]
    {
        let cs = CString::new(s).unwrap();
        let c_out = run_print_line(&c, Some(&cs));
        let r_out = run_print_line(&r, Some(&cs));
        assert_eq!(
            c_out, r_out,
            "printLine differs for {:?}: c={:?} rust={:?}",
            s, c_out, r_out
        );
    }
}

#[test]
fn print_hex_char_line_full_range() {
    let c = load_c();
    let r = load_rust();
    // Cover the entire signed-char range.
    for v in i8::MIN..=i8::MAX {
        let arg = v as c_char;
        let c_out = run_print_hex_char(&c, arg);
        let r_out = run_print_hex_char(&r, arg);
        assert_eq!(
            c_out, r_out,
            "printHexCharLine differs for {}: c={:?} rust={:?}",
            v, c_out, r_out
        );
    }
}

#[test]
fn bad_function_matches_c() {
    let c = load_c();
    let r = load_rust();
    let c_out = run_void_fn(&c, "bad");
    let r_out = run_void_fn(&r, "bad");
    assert_eq!(c_out, r_out, "bad() outputs differ");
}

#[test]
fn good_function_matches_c() {
    let c = load_c();
    let r = load_rust();
    let c_out = run_void_fn(&c, "good");
    let r_out = run_void_fn(&r, "good");
    assert_eq!(c_out, r_out, "good() outputs differ");
}

/// Verify each .so exports the symbol set under test. Using libloading::get
/// to confirm presence.
#[test]
fn rust_exports_match_c_public_api() {
    let c = load_c();
    let r = load_rust();
    for sym in ["printLine", "printHexCharLine", "bad", "good", "main"] {
        unsafe {
            let _: Symbol<unsafe extern "C" fn()> = c
                .get(sym.as_bytes())
                .unwrap_or_else(|_| panic!("C .so missing symbol {}", sym));
            let _: Symbol<unsafe extern "C" fn()> = r
                .get(sym.as_bytes())
                .unwrap_or_else(|_| panic!("Rust .so missing symbol {}", sym));
        }
    }
}
