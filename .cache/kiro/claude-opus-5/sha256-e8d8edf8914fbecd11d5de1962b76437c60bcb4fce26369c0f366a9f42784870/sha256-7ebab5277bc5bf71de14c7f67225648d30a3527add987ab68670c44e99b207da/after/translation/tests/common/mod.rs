//! Shared harness: loads the C and Rust shared objects side by side and
//! captures anything they write to stdout so results can be diffed exactly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::OnceLock;

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn free(p: *mut c_void);
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn find_so(dir: &PathBuf) -> PathBuf {
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no .so found in {}", dir.display()))
}

/// Loads both libraries once for the whole test binary.
pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let root = workspace_root();

        let c_build = root.join("c_src").join("build");
        let c_path = find_so(&c_build);

        // The Rust cdylib lives next to the test binary's build output.
        // `cargo test` does not necessarily refresh the cdylib, so verify it is
        // newer than the sources — otherwise the whole suite would silently
        // compare against a stale library.
        let rust_path = {
            let mut exe = std::env::current_exe().expect("current_exe");
            exe.pop(); // deps/
            if exe.file_name().map(|n| n == "deps").unwrap_or(false) {
                exe.pop();
            }
            let p = exe.join("libcharinbuf_lib.so");
            if !p.exists() {
                panic!(
                    "Rust cdylib not found at {} — run `cargo build` first",
                    p.display()
                );
            }
            let so_mtime = std::fs::metadata(&p)
                .and_then(|m| m.modified())
                .expect("cdylib mtime");
            let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
            for entry in std::fs::read_dir(&src_dir).expect("read src/").flatten() {
                let sp = entry.path();
                if let Ok(m) = std::fs::metadata(&sp).and_then(|m| m.modified()) {
                    assert!(
                        m <= so_mtime,
                        "{} is newer than {} — the cdylib is stale. \
                         Run `cargo build` (or ./run_tests.sh) before `cargo test`.",
                        sp.display(),
                        p.display()
                    );
                }
            }
            p
        };


        unsafe {
            let c = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("load {}: {e}", c_path.display()));
            let rust = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("load {}: {e}", rust_path.display()));
            Libs { c, rust }
        }
    })
}

/// Resolve `name` from both libraries, returning `(c_sym, rust_sym)`.
pub fn sym<T>(name: &str) -> (Symbol<'static, T>, Symbol<'static, T>) {
    let l = libs();
    let bytes = name.as_bytes();
    unsafe {
        let c: Symbol<T> = l
            .c
            .get(&[bytes, b"\0"].concat())
            .unwrap_or_else(|e| panic!("C symbol `{name}` missing: {e}"));
        let r: Symbol<T> = l
            .rust
            .get(&[bytes, b"\0"].concat())
            .unwrap_or_else(|e| panic!("Rust symbol `{name}` missing: {e}"));
        (c, r)
    }
}

/// Runs `f` with fd 1 redirected into a temp file and returns
/// `(return value, captured bytes)`.
///
/// `fflush(NULL)` is issued before and after so that anything sitting in the
/// process-wide stdio buffer is attributed to the right side of the diff.
pub fn capture<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    unsafe {
        fflush(std::ptr::null_mut());
    }

    let mut tmp = std::env::temp_dir();
    tmp.push(format!(
        "charinbuf_cap_{}_{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&tmp)
        .expect("open capture file");

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    unsafe {
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto stdout failed");
    }

    let ret = f();

    unsafe {
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }

    let mut file = file;
    file.seek(SeekFrom::Start(0)).expect("seek");
    let mut out = Vec::new();
    file.read_to_end(&mut out).expect("read capture");
    drop(file);
    let _ = std::fs::remove_file(&tmp);

    (ret, out)
}

pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// Read a NUL-terminated C string as bytes (excluding the terminator).
pub unsafe fn cstr_bytes(p: *const c_char) -> Vec<u8> {
    let mut v = Vec::new();
    let mut i = 0isize;
    loop {
        let ch = unsafe { *p.offset(i) };
        if ch == 0 {
            break;
        }
        v.push(ch as u8);
        i += 1;
    }
    v
}

pub unsafe fn cfree(p: *mut c_void) {
    if !p.is_null() {
        unsafe { free(p) }
    }
}

/// stdout redirection and the libraries' file-scope counters are both
/// process-wide, so every test body must hold this lock.
pub fn lock() -> std::sync::MutexGuard<'static, ()> {
    static M: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    match M.get_or_init(|| std::sync::Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}
