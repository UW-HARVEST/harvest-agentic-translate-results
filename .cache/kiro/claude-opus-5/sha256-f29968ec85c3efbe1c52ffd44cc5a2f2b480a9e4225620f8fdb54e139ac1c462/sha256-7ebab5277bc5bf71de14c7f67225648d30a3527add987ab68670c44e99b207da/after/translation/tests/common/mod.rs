#![allow(dead_code)]
//! Shared helpers: locate the two shared libraries and capture stdout at the
//! file-descriptor level so that output produced by `printf` inside either
//! library can be compared byte-for-byte.

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn strncmp(a: *const c_char, b: *const c_char, n: usize) -> c_int;
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation dir has a parent")
        .to_path_buf()
}

/// Path to the C shared library built from `c_src`.
/// Override with `C_LIB_PATH` to compare against a differently-compiled C
/// build (e.g. an optimized one) without touching `c_src`.
pub fn c_lib_path() -> PathBuf {
    if let Some(p) = std::env::var_os("C_LIB_PATH") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "C_LIB_PATH does not exist: {}", p.display());
        return p;
    }
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no .so found in {}; build the C library first (cmake .. && cmake --build .)",
            build.display()
        )
    })
}

/// Path to the Rust `cdylib` under test.
///
/// The crate declares `crate-type = ["cdylib"]` only, so `cargo test` does not
/// refresh `libcleanup_lib.so` as a side effect of building the integration
/// tests — a stale artifact would silently be tested instead of the current
/// sources. Build it explicitly (into its own target directory to avoid
/// contending for the outer build lock) and return the fresh path.
pub fn rust_lib_path() -> PathBuf {
    static PATH: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    PATH.get_or_init(build_rust_cdylib).clone()
}

fn build_rust_cdylib() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = manifest_dir.join("target").join("so-under-test");
    let release = is_release_build();

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut cmd = std::process::Command::new(cargo);
    cmd.current_dir(&manifest_dir)
        .arg("build")
        .arg("--lib")
        .arg("--manifest-path")
        .arg(manifest_dir.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target_dir);
    if release {
        cmd.arg("--release");
    }
    // Forward the feature selection this test binary was compiled with so the
    // .so under test matches the configuration being exercised.
    cmd.arg("--no-default-features");
    let features = enabled_features();
    if !features.is_empty() {
        cmd.arg("--features").arg(features.join(","));
    }
    // Avoid inheriting cargo's per-invocation environment, which otherwise
    // confuses the nested build.
    for var in [
        "RUSTC_WORKSPACE_WRAPPER",
        "CARGO_MAKEFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "LD_LIBRARY_PATH",
    ] {
        cmd.env_remove(var);
    }

    let out = cmd.output().expect("run nested cargo build for cdylib");
    assert!(
        out.status.success(),
        "building the Rust cdylib failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let path = target_dir
        .join(if release { "release" } else { "debug" })
        .join("libcleanup_lib.so");
    assert!(
        path.is_file(),
        "expected cdylib at {} after build",
        path.display()
    );
    path
}

fn is_release_build() -> bool {
    // The test binary lives in <target>/<profile>/deps/<name>.
    std::env::current_exe()
        .ok()
        .and_then(|p| {
            p.parent()
                .and_then(|d| d.parent())
                .and_then(|d| d.file_name())
                .map(|s| s.to_string_lossy().into_owned())
        })
        .map(|profile| profile != "debug")
        .unwrap_or(false)
}

/// Features active in this test binary, mirrored from `cargo`'s
/// `CARGO_FEATURE_*` compile-time environment. The crate currently declares no
/// `[features]`, so this is normally empty; it keeps the harness correct if
/// feature flags are added later.
fn enabled_features() -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for (k, _) in std::env::vars() {
        if let Some(rest) = k.strip_prefix("CARGO_FEATURE_") {
            v.push(rest.to_ascii_lowercase().replace('_', "-"));
        }
    }
    v.sort();
    v
}

/// Runs `f` with file descriptor 1 redirected into a temporary file and
/// returns everything written to it (including output from C `printf`).
pub fn capture_stdout<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::fd::AsRawFd;

    let mut tmp = tempfile();

    // Flush anything Rust/libc has buffered before swapping the fd.
    let _ = std::io::Write::flush(&mut std::io::stdout());
    unsafe { fflush(std::ptr::null_mut()) };

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(tmp.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let result = f();

    unsafe { fflush(std::ptr::null_mut()) };
    let _ = std::io::Write::flush(&mut std::io::stdout());

    assert!(unsafe { dup2(saved, 1) } >= 0, "restore dup2 failed");
    unsafe { close(saved) };

    let mut buf = Vec::new();
    tmp.seek(SeekFrom::Start(0)).expect("seek");
    tmp.read_to_end(&mut buf).expect("read captured stdout");
    (result, buf)
}

fn tempfile() -> std::fs::File {
    let mut path = std::env::temp_dir();
    let unique = format!(
        "c2rust-capture-{}-{:?}-{}.tmp",
        std::process::id(),
        std::thread::current().id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    path.push(unique);
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create temp file");
    // Unlink immediately; the open handle keeps it alive.
    let _ = std::fs::remove_file(&path);
    file
}

/// Renders a byte buffer for assertion messages.
pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}
