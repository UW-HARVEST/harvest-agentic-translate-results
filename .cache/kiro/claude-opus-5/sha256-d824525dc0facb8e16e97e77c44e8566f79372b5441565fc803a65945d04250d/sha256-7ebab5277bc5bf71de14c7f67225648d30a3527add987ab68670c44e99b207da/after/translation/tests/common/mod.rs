//! Shared differential-testing harness.
//!
//! Both the original C shared library (`c_src/build/libdriver.so`) and the Rust
//! translation (`target/<profile>/libdriver.so`) are loaded with `libloading`,
//! so *every* call — including the Rust one — crosses a real FFI boundary and
//! therefore exercises the `#[no_mangle]` export wrappers.

#![allow(dead_code)]

use std::ffi::{CString, c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits we need for byte-exact capture of the C `stdio` streams
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fclose(stream: *mut c_void) -> c_int;
    fn feof(stream: *mut c_void) -> c_int;
    fn ferror(stream: *mut c_void) -> c_int;
}

/// `fflush(NULL)` — flush *all* open output streams.
fn flush_all() {
    unsafe { fflush(std::ptr::null_mut()) };
}

/// Close a `FILE*` handed back across the FFI boundary.
pub fn close_file(fp: *mut c_void) {
    if !fp.is_null() {
        unsafe { fclose(fp) };
    }
}

/// Observable `FILE*` state, so we compare more than just null-ness.
#[derive(Debug, PartialEq, Eq)]
pub struct FileState {
    pub is_null: bool,
    pub eof: c_int,
    pub error: c_int,
}

pub fn file_state(fp: *mut c_void) -> FileState {
    if fp.is_null() {
        return FileState { is_null: true, eof: 0, error: 0 };
    }
    FileState {
        is_null: false,
        eof: unsafe { feof(fp) }.signum(),
        error: unsafe { ferror(fp) }.signum(),
    }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("translation/ has a parent")
        .join("c_src/build/libdriver.so")
}

/// `target/<profile>/libdriver.so`, derived from the test binary's own location
/// (`target/<profile>/deps/<test>-<hash>`).
///
/// `cargo test` does not produce a `.so` for a `crate-type = ["cdylib"]` library
/// (it only builds an `rmeta`), so the shared object is built here — **always**,
/// not just when it is absent, otherwise a stale artifact from an earlier build
/// would silently be tested instead of the current sources.
///
/// `DRIVER_RUST_SO` overrides the path (and skips the build); extra cargo flags
/// such as a feature selection come from `DRIVER_CARGO_FLAGS`.
pub fn rust_so_path() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(build_rust_so).clone()
}

fn build_rust_so() -> PathBuf {
    if let Some(p) = std::env::var_os("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }

    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>/deps/<test>")
        .to_path_buf();
    let so = profile_dir.join("libdriver.so");

    let profile = profile_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("debug")
        .to_string();

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut cmd = std::process::Command::new(cargo);
    cmd.current_dir(manifest_dir()).arg("build");
    if profile != "debug" {
        cmd.arg("--profile").arg(&profile);
    }
    for flag in std::env::var("DRIVER_CARGO_FLAGS")
        .unwrap_or_default()
        .split_whitespace()
    {
        cmd.arg(flag);
    }
    let out = cmd.output().expect("spawn cargo build for the cdylib");
    assert!(
        out.status.success(),
        "`cargo build` for the cdylib failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The build must have actually refreshed the artifact we are about to load.
    let built_at = std::fs::metadata(&so)
        .unwrap_or_else(|e| panic!("{} not produced by cargo build: {e}", so.display()))
        .modified()
        .expect("mtime");
    let src_at = std::fs::metadata(manifest_dir().join("src/lib.rs"))
        .and_then(|m| m.modified())
        .expect("mtime of src/lib.rs");
    assert!(
        built_at >= src_at,
        "{} is older than src/lib.rs — a stale library would be tested",
        so.display()
    );

    so
}

fn load(path: PathBuf) -> Library {
    assert!(
        path.exists(),
        "shared library not found: {}\n\
         build it first (cmake for the C side, `cargo build` for Rust)",
        path.display()
    );
    unsafe { Library::new(&path) }.unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()))
}

/// The original C implementation.
pub fn c_lib() -> &'static Library {
    static L: OnceLock<Library> = OnceLock::new();
    L.get_or_init(|| load(c_so_path()))
}

/// The Rust translation, loaded exactly like any other external consumer.
pub fn rust_lib() -> &'static Library {
    static L: OnceLock<Library> = OnceLock::new();
    L.get_or_init(|| load(rust_so_path()))
}

pub fn sym<T>(lib: &'static Library, name: &str) -> Symbol<'static, T> {
    unsafe { lib.get(name.as_bytes()) }
        .unwrap_or_else(|e| panic!("missing exported symbol `{name}`: {e}"))
}

// ---------------------------------------------------------------------------
// Signatures of the public API (see c_src/include/goto.h and c_src/src/goto.c)
// ---------------------------------------------------------------------------

pub type ForwardGotoExample = unsafe extern "C" fn(c_int) -> c_int;
pub type OpenWithCleanup = unsafe extern "C" fn(*const c_char) -> *mut c_void;
pub type Driver = unsafe extern "C" fn(c_int, *const c_char) -> c_int;

// ---------------------------------------------------------------------------
// stdout / stderr capture
// ---------------------------------------------------------------------------

/// Serialises the process-global fd redirection performed by [`capture`].
fn capture_lock() -> &'static Mutex<()> {
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(()))
}

/// Everything an implementation observably produced for one call.
pub struct Observed<T> {
    pub ret: T,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl<T> Observed<T> {
    pub fn describe(&self) -> String {
        format!(
            "stdout={:?}\n  stderr={:?}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
        )
    }
}

/// Run `f` with fds 1 and 2 redirected to fresh temporary files and return its
/// result together with the raw bytes each stream received.
pub fn capture<T>(f: impl FnOnce() -> T) -> Observed<T> {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let dir = std::env::temp_dir();
    let unique = format!(
        "{}-{:?}-{}",
        std::process::id(),
        std::thread::current().id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let out_path = dir.join(format!("goto-diff-out-{unique}"));
    let err_path = dir.join(format!("goto-diff-err-{unique}"));

    // Nothing of ours should still be sitting in a stdio buffer.
    flush_all();

    let saved_out = unsafe { dup(1) };
    let saved_err = unsafe { dup(2) };
    assert!(saved_out >= 0 && saved_err >= 0, "dup failed");

    let out_file = std::fs::File::create(&out_path).expect("create stdout capture file");
    let err_file = std::fs::File::create(&err_path).expect("create stderr capture file");
    let (out_fd, err_fd) = {
        use std::os::fd::AsRawFd;
        (out_file.as_raw_fd(), err_file.as_raw_fd())
    };
    assert!(unsafe { dup2(out_fd, 1) } >= 0, "dup2 stdout failed");
    assert!(unsafe { dup2(err_fd, 2) } >= 0, "dup2 stderr failed");

    let ret = f();

    // Push the callee's buffered output into the capture files before restoring.
    flush_all();

    assert!(unsafe { dup2(saved_out, 1) } >= 0, "restore stdout failed");
    assert!(unsafe { dup2(saved_err, 2) } >= 0, "restore stderr failed");
    unsafe {
        close(saved_out);
        close(saved_err);
    }
    drop(out_file);
    drop(err_file);

    let stdout = std::fs::read(&out_path).expect("read stdout capture");
    let stderr = std::fs::read(&err_path).expect("read stderr capture");
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(&err_path);

    Observed { ret, stdout, stderr }
}

static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

// ---------------------------------------------------------------------------
// Comparison helper
// ---------------------------------------------------------------------------

/// Collects mismatches so a single test reports every failing input at once.
#[derive(Default)]
pub struct Diffs(Vec<String>);

impl Diffs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn compare<T: PartialEq + std::fmt::Debug>(
        &mut self,
        case: &str,
        c: &Observed<T>,
        rust: &Observed<T>,
    ) {
        if c.ret != rust.ret {
            self.0.push(format!(
                "[{case}] return value differs:\n  C    = {:?}\n  Rust = {:?}",
                c.ret, rust.ret
            ));
        }
        if c.stdout != rust.stdout {
            self.0.push(format!(
                "[{case}] stdout differs:\n  C    = {:?}\n  Rust = {:?}",
                String::from_utf8_lossy(&c.stdout),
                String::from_utf8_lossy(&rust.stdout),
            ));
        }
        if c.stderr != rust.stderr {
            self.0.push(format!(
                "[{case}] stderr differs:\n  C    = {:?}\n  Rust = {:?}",
                String::from_utf8_lossy(&c.stderr),
                String::from_utf8_lossy(&rust.stderr),
            ));
        }
    }

    pub fn assert_empty(self) {
        if !self.0.is_empty() {
            panic!(
                "{} C/Rust mismatch(es):\n\n{}\n",
                self.0.len(),
                self.0.join("\n\n")
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

pub fn cstr(s: &str) -> CString {
    CString::new(s).expect("no interior NUL")
}

/// A temporary directory that cleans itself up.
pub struct TmpDir(PathBuf);

impl TmpDir {
    pub fn new(tag: &str) -> Self {
        let p = std::env::temp_dir().join(format!(
            "goto-diff-{tag}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&p).expect("create temp dir");
        TmpDir(p)
    }

    pub fn path(&self) -> &std::path::Path {
        &self.0
    }

    /// Write `bytes` to `name` inside the directory and return its path.
    pub fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let p = self.0.join(name);
        std::fs::write(&p, bytes).expect("write fixture");
        p
    }
}

impl Drop for TmpDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
