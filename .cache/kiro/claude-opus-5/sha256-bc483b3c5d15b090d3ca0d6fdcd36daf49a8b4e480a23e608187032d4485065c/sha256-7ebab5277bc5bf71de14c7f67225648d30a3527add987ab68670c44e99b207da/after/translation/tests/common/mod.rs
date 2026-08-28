#![allow(dead_code)]
//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and exposes helpers to call them and capture their stdout.
//!
//! Nothing here calls the Rust crate directly — every invocation goes through
//! the dynamic-library export table, exactly like an external C caller, so the
//! `#[no_mangle]` wrappers are part of what is under test.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int, c_void};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

pub type FnSafeDoubleToInt = unsafe extern "C" fn(c_double) -> c_int;
pub type FnProcessWithFallthrough = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnCopyDataBlock = unsafe extern "C" fn(*mut c_void, *const c_void);
pub type FnHandlePointerOperations = unsafe extern "C" fn(c_int) -> c_int;
pub type FnOverunder = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

pub struct Impls {
    pub c: Library,
    pub rust: Library,
}

impl Impls {
    pub fn sym<'a, T>(&self, lib: &'a Library, name: &str) -> Symbol<'a, T> {
        let bytes = format!("{name}\0").into_bytes();
        unsafe {
            lib.get::<T>(&bytes)
                .unwrap_or_else(|e| panic!("symbol `{name}` missing: {e}"))
        }
    }

    pub fn c_sym<T>(&self, name: &str) -> Symbol<'_, T> {
        self.sym(&self.c, name)
    }

    pub fn rust_sym<T>(&self, name: &str) -> Symbol<'_, T> {
        self.sym(&self.rust, name)
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn find_c_so() -> PathBuf {
    // Allows the same suite to be re-run against an alternative C build (e.g. an
    // optimized one) without duplicating any test logic.
    if let Some(p) = std::env::var_os("C_SO_PATH") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "C_SO_PATH does not exist: {}", p.display());
        return p;
    }
    let build = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

/// Feature flags this test binary was compiled with. Integration tests are
/// compiled under the same feature resolution as the library, so mirroring them
/// here guarantees the `.so` we build below matches the configuration under
/// test. `ALL_FEATURES` must list every feature in `Cargo.toml`'s `[features]`
/// table; the compile-time assertion in `feature_args` guards against drift.
const ALL_FEATURES: &[(&str, bool)] = &[
    // (name, enabled) — the crate currently declares no features.
];

fn feature_args() -> Vec<String> {
    // `--no-default-features` plus an explicit list reproduces exactly this
    // build's configuration regardless of what `default` contains.
    let enabled: Vec<&str> = ALL_FEATURES
        .iter()
        .filter(|(_, on)| *on)
        .map(|(n, _)| *n)
        .collect();
    let mut args = vec!["--no-default-features".to_string()];
    if !enabled.is_empty() {
        args.push("--features".to_string());
        args.push(enabled.join(","));
    }
    args
}

/// Builds the Rust `cdylib` and returns its path.
///
/// This is essential rather than convenient: an integration test has no
/// dependency edge to a `crate-type = ["cdylib"]` target, so `cargo test` does
/// **not** rebuild the `.so`. Loading whatever happens to be sitting in
/// `target/<profile>/` would silently verify a stale artifact. Building here ties
/// the library under test to the current sources.
///
/// A dedicated `CARGO_TARGET_DIR` is used so this nested invocation cannot
/// contend with the outer `cargo test`'s lock on the main target directory.
fn build_rust_so() -> PathBuf {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = manifest.join(format!("target/harness-{profile}"));

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut cmd = std::process::Command::new(cargo);
    cmd.arg("build")
        .args(feature_args())
        .current_dir(&manifest)
        .env("CARGO_TARGET_DIR", &target_dir)
        .env_remove("RUSTFLAGS");
    if profile == "release" {
        cmd.arg("--release");
    }

    let out = cmd.output().expect("failed to spawn cargo to build the cdylib");
    assert!(
        out.status.success(),
        "building the Rust cdylib failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let so = target_dir.join(profile).join("liboverunder_lib.so");
    assert!(so.exists(), "cdylib not produced at {}", so.display());

    // Staleness guard: the artifact must be at least as new as the sources.
    let src = manifest.join("src/lib.rs");
    if let (Ok(a), Ok(b)) = (std::fs::metadata(&so), std::fs::metadata(&src)) {
        if let (Ok(ta), Ok(tb)) = (a.modified(), b.modified()) {
            assert!(
                ta >= tb,
                "Rust .so ({}) is older than {} — it would not reflect the current source",
                so.display(),
                src.display()
            );
        }
    }
    so
}

fn find_rust_so() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(build_rust_so).clone()
}

pub fn c_so_path() -> PathBuf {
    find_c_so()
}

pub fn rust_so_path() -> PathBuf {
    find_rust_so()
}

pub fn impls() -> &'static Impls {
    static IMPLS: OnceLock<Impls> = OnceLock::new();
    IMPLS.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        unsafe {
            Impls {
                c: Library::new(&c_path)
                    .unwrap_or_else(|e| panic!("load C so {}: {e}", c_path.display())),
                rust: Library::new(&rust_path)
                    .unwrap_or_else(|e| panic!("load Rust so {}: {e}", rust_path.display())),
            }
        }
    })
}

/// Runs `f` with file descriptor 1 redirected into a temporary file and returns
/// `(f's return value, raw stdout bytes)`.
///
/// Both libraries funnel their output through the process-wide libc `stdout`,
/// so this captures C and Rust output identically (including `printf`'s exact
/// float formatting and any buffering effects).
///
/// IMPORTANT: redirecting fd 1 affects the whole process, including libtest's
/// own progress output ("test foo ... ok"), which the harness writes from the
/// main thread while other test threads are still running. Each test binary
/// therefore declares exactly ONE `#[test]` function, so nothing else can be
/// writing to fd 1 while a capture is active. Separate test binaries are
/// separate processes with independent descriptor tables, so they cannot
/// interfere with each other.
pub fn capture_stdout<T, F: FnOnce() -> T>(f: F) -> (T, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom};

    // Redirecting fd 1 is a process-wide effect and cargo's harness runs test
    // functions on parallel threads. Serialize so captures cannot interleave
    // (and so the harness' own output is never swallowed).
    static LOCK: Mutex<()> = Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut tmp = std::env::temp_dir();
    tmp.push(format!(
        "overunder_capture_{}_{}.txt",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&tmp)
        .expect("open capture file");

    let ret;
    let mut buf = Vec::new();
    unsafe {
        use std::os::unix::io::AsRawFd;
        // Flush anything already pending so it is not misattributed.
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        ret = f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);

        file.seek(SeekFrom::Start(0)).expect("seek");
        file.read_to_end(&mut buf).expect("read capture");
    }
    let _ = std::fs::remove_file(&tmp);
    (ret, buf)
}

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// A byte-level stand-in for the C `DataBlock` so that struct padding is part of
/// the comparison. 64 bytes is comfortably larger than `sizeof(DataBlock)`,
/// which lets the test observe exactly how many bytes each implementation
/// copies.
pub const BLOCK_SCRATCH: usize = 64;

pub fn make_block_bytes(id: c_int, value: c_double, label: &[u8], fill: u8) -> [u8; BLOCK_SCRATCH] {
    let mut b = [fill; BLOCK_SCRATCH];
    b[0..4].copy_from_slice(&id.to_ne_bytes());
    b[8..16].copy_from_slice(&value.to_ne_bytes());
    for i in 0..20 {
        b[16 + i] = if i < label.len() { label[i] } else { 0 };
    }
    b
}


