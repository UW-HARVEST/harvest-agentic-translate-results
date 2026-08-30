//! Shared harness for differential testing of the C `libdriver.so` against the
//! Rust `libdriver.so`.
//!
//! Both libraries are loaded through `libloading` (i.e. `dlopen` with
//! `RTLD_LOCAL`), so the Rust code is only ever reached through its
//! `#[no_mangle]` C ABI exports, exactly as an external caller would.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

const STDOUT_FD: c_int = 1;

/// `target/<profile>/libdriver.so` — derived from the test binary's own path
/// (`target/<profile>/deps/<test>-<hash>`), so it follows `--release` etc.
///
/// `cargo test` only builds the *test* targets, so the `cdylib` may not exist
/// yet on a clean tree; in that case it is built on demand. This keeps a plain
/// `cargo test` self-sufficient.
pub fn rust_so_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let exe = std::env::current_exe().expect("current_exe");
        let profile_dir = exe
            .parent() // deps/
            .and_then(Path::parent) // target/<profile>/
            .expect("test binary layout")
            .to_path_buf();
        let candidate = profile_dir.join("libdriver.so");
        if !candidate.is_file() {
            build_cdylib(&profile_dir);
        }
        assert!(
            candidate.is_file(),
            "Rust cdylib not found at {}. Build it with `cargo build` (add \
             `--release` for the release profile) and re-run the tests.",
            candidate.display()
        );
        candidate
    })
    .clone()
}

/// Runs `cargo build --lib` for the profile implied by `profile_dir`.
fn build_cdylib(profile_dir: &Path) {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut cmd = std::process::Command::new(cargo);
    cmd.arg("build")
        .arg("--lib")
        .current_dir(env!("CARGO_MANIFEST_DIR"));
    if profile_dir.file_name().and_then(|s| s.to_str()) == Some("release") {
        cmd.arg("--release");
    }
    // Inherit the caller's feature selection so the cdylib matches the tests.
    for flag in feature_flags() {
        cmd.arg(flag);
    }
    match cmd.status() {
        Ok(s) if s.success() => {}
        other => eprintln!("note: on-demand `cargo build --lib` did not succeed: {other:?}"),
    }
}

/// Feature flags recorded in `tests/feature_flags.txt`, if present.
///
/// The crate currently declares no `[features]`, so this is empty in practice;
/// the hook exists so a future feature matrix can be threaded through to the
/// on-demand cdylib build.
fn feature_flags() -> Vec<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("feature_flags.txt");
    match fs::read_to_string(path) {
        Ok(s) => s.split_whitespace().map(str::to_string).collect(),
        Err(_) => Vec::new(),
    }
}

/// `../c_src/build/libdriver.so`, produced by the CMake build.
pub fn c_so_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest
        .parent()
        .expect("workspace root")
        .join("c_src")
        .join("build")
        .join("libdriver.so");
    assert!(
        candidate.is_file(),
        "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        candidate.display()
    );
    candidate
}

fn load(path: &Path) -> &'static Library {
    // Leaked on purpose: the libraries stay resident for the whole test binary,
    // which keeps every `Symbol` we hand out valid.
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
    Box::leak(Box::new(lib))
}

pub fn c_lib() -> &'static Library {
    static LIB: OnceLock<&'static Library> = OnceLock::new();
    LIB.get_or_init(|| load(&c_so_path()))
}

pub fn rust_lib() -> &'static Library {
    static LIB: OnceLock<&'static Library> = OnceLock::new();
    LIB.get_or_init(|| load(&rust_so_path()))
}

/// Both implementations, tagged for assertion messages.
pub struct Impls {
    pub c: &'static Library,
    pub rust: &'static Library,
}

pub fn impls() -> Impls {
    Impls {
        c: c_lib(),
        rust: rust_lib(),
    }
}

// ---------------------------------------------------------------------------
// Symbol lookup
// ---------------------------------------------------------------------------

pub type FnPrintLine = unsafe extern "C" fn(*const c_char);
pub type FnVoid = unsafe extern "C" fn();
pub type FnDriver = unsafe extern "C" fn(c_int);

pub fn sym<T>(lib: &'static Library, name: &str) -> Symbol<'static, T> {
    unsafe { lib.get::<T>(name.as_bytes()) }
        .unwrap_or_else(|e| panic!("missing exported symbol `{name}`: {e}"))
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// fd 1 is process-global, so captures must not overlap.
fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Runs `f` with file descriptor 1 redirected to a temporary file and returns
/// every byte it wrote.
///
/// `fflush(NULL)` is issued on both sides of the redirect so that the C
/// library's `stdout` buffer is drained into the capture file and nothing
/// leaks across calls.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let _guard = capture_lock();

    let path = std::env::temp_dir().join(format!(
        "driver-capture-{}-{}.bin",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let file = fs::File::create(&path).expect("create capture file");

    // Drain anything already pending so it is not attributed to `f`.
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let saved = unsafe { dup(STDOUT_FD) };
    assert!(saved >= 0, "dup(1) failed");

    let redirected = unsafe { dup2(as_raw_fd(&file), STDOUT_FD) };
    assert!(redirected >= 0, "dup2 onto stdout failed");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, STDOUT_FD) } >= 0, "restore stdout failed");
    unsafe { close(saved) };
    drop(file);

    let mut buf = Vec::new();
    fs::File::open(&path)
        .expect("reopen capture file")
        .read_to_end(&mut buf)
        .expect("read capture file");
    let _ = fs::remove_file(&path);

    match result {
        Ok(()) => buf,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

fn as_raw_fd(file: &fs::File) -> c_int {
    use std::os::unix::io::AsRawFd;
    file.as_raw_fd()
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

pub fn show(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &b in bytes {
        match b {
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            b'\\' => out.push_str("\\\\"),
            0x20..=0x7e => out.push(b as char),
            _ => out.push_str(&format!("\\x{b:02x}")),
        }
    }
    out
}

/// Byte-for-byte comparison of two captured outputs.
pub fn assert_same_output(label: &str, c_out: &[u8], rust_out: &[u8]) {
    assert_eq!(
        c_out,
        rust_out,
        "\n{label}: stdout mismatch\n  C    ({} bytes): \"{}\"\n  Rust ({} bytes): \"{}\"\n",
        c_out.len(),
        show(c_out),
        rust_out.len(),
        show(rust_out),
    );
}

/// Collects every mismatch so a whole run reports all of them at once.
///
/// All stdout-capturing comparisons live inside a *single* `#[test]`: file
/// descriptor 1 is process-global, and libtest's own progress output would
/// otherwise be written into a capture window by a concurrently finishing test.
#[derive(Default)]
pub struct Report {
    checks: usize,
    failures: Vec<String>,
}

impl Report {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check(&mut self, label: &str, c_out: &[u8], rust_out: &[u8]) {
        self.checks += 1;
        if c_out != rust_out {
            self.failures.push(format!(
                "{label}\n    C    ({:>7} bytes): \"{}\"\n    Rust ({:>7} bytes): \"{}\"",
                c_out.len(),
                show(c_out),
                rust_out.len(),
                show(rust_out),
            ));
        }
    }

    pub fn checks(&self) -> usize {
        self.checks
    }

    pub fn finish(self) {
        if !self.failures.is_empty() {
            let shown: Vec<&String> = self.failures.iter().take(25).collect();
            panic!(
                "{} of {} differential checks mismatched:\n  {}{}",
                self.failures.len(),
                self.checks,
                shown
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join("\n  "),
                if self.failures.len() > shown.len() {
                    format!("\n  ... and {} more", self.failures.len() - shown.len())
                } else {
                    String::new()
                }
            );
        }
        eprintln!("{} differential checks passed", self.checks);
    }
}
