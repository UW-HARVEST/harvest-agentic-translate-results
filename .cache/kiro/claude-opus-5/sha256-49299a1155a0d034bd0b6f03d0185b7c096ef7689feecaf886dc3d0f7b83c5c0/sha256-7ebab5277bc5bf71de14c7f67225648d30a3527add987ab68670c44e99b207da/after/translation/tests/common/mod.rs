//! Shared harness: loads the C `.so` and the Rust `.so` side by side and
//! provides stdout capture so that `printf` output can be diffed too.
//!
//! The Rust side is *never* called directly — every call goes through
//! `libloading` against the exported `#[no_mangle]` symbol, exactly like an
//! external C caller would do.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int};
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

unsafe extern "C" {
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

/// Workspace root (the directory that contains `c_src/` and `translation/`).
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Locate the C shared library produced by CMake.
fn c_lib_path() -> PathBuf {    let build = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e} (did you run cmake?)", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

/// Locate the Rust cdylib for the profile the test itself was built with.
///
/// `current_exe()` is `<target>/<profile>/deps/<test>`; the cdylib lives in
/// `<target>/<profile>/`.
///
/// `cargo test` does not build `cdylib`-only targets, so the library is built
/// on demand with a nested `cargo build` (cargo has already released its build
/// lock by the time the test binary runs).
fn rust_lib_path() -> PathBuf {
    static ONCE: std::sync::Once = std::sync::Once::new();

    let exe = std::env::current_exe().expect("current_exe");
    let deps_dir = exe.parent().expect("…/deps/<test>");
    let profile_dir = deps_dir.parent().expect("…/<profile>/deps/<test>");
    let target_dir = profile_dir.parent().expect("<target>/<profile>/…");
    let profile_name = profile_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("debug")
        .to_string();

    let p = profile_dir.join("libmodeselect_lib.so");

    ONCE.call_once(|| {
        let mut cmd = std::process::Command::new(
            std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()),
        );
        cmd.arg("build")
            .arg("--lib")
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .env("CARGO_TARGET_DIR", target_dir);
        match profile_name.as_str() {
            "debug" => {}
            "release" => {
                cmd.arg("--release");
            }
            other => {
                cmd.arg("--profile").arg(other);
            }
        }
        // Mirror the feature selection the test binary itself was built with.
        for extra in feature_args() {
            cmd.arg(extra);
        }
        let out = cmd.output().expect("spawn cargo build --lib");
        if !out.status.success() {
            panic!(
                "nested `cargo build --lib` failed:\n{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
    });

    assert!(
        p.exists(),
        "Rust cdylib not found at {} after `cargo build --lib`",
        p.display()
    );
    p
}

/// Reconstruct `--no-default-features --features …` from the `CARGO_FEATURE_*`
/// environment, so the on-demand cdylib build matches the test build.
///
/// This crate currently declares no `[features]`, so the list is normally
/// empty; the logic is kept so that adding features later needs no test change.
fn feature_args() -> Vec<String> {
    let mut feats: Vec<String> = std::env::vars()
        .filter_map(|(k, _)| k.strip_prefix("CARGO_FEATURE_").map(|s| s.to_lowercase()))
        .collect();
    feats.sort();
    if feats.is_empty() {
        Vec::new()
    } else {
        vec![
            "--no-default-features".to_string(),
            "--features".to_string(),
            feats.join(","),
        ]
    }
}

/// Both implementations, loaded through the dynamic linker.
pub struct Pair {
    pub c: Library,
    pub rs: Library,
}

impl Pair {
    pub fn load() -> Self {
        // RTLD_LOCAL (libloading's default) keeps the two identically-named
        // symbol sets from colliding with each other.
        let c = unsafe { Library::new(c_lib_path()) }.expect("load C .so");
        let rs = unsafe { Library::new(rust_lib_path()) }.expect("load Rust .so");
        Pair { c, rs }
    }

    fn sym<'a, T>(lib: &'a Library, name: &str) -> Symbol<'a, T> {
        unsafe { lib.get(name.as_bytes()) }
            .unwrap_or_else(|e| panic!("missing symbol `{name}`: {e}"))
    }

    // -- one accessor pair per exported function -----------------------------

    pub fn classify_mode(&self) -> (Fn1p, Fn1p) {
        (
            *Self::sym::<Fn1p>(&self.c, "classify_mode"),
            *Self::sym::<Fn1p>(&self.rs, "classify_mode"),
        )
    }

    pub fn apply_multiplier(&self) -> (Fn2i, Fn2i) {
        (
            *Self::sym::<Fn2i>(&self.c, "apply_multiplier"),
            *Self::sym::<Fn2i>(&self.rs, "apply_multiplier"),
        )
    }

    pub fn convert_time_factor(&self) -> (Fn1d, Fn1d) {
        (
            *Self::sym::<Fn1d>(&self.c, "convert_time_factor"),
            *Self::sym::<Fn1d>(&self.rs, "convert_time_factor"),
        )
    }

    pub fn convert_negative_overflow(&self) -> (Fn1d, Fn1d) {
        (
            *Self::sym::<Fn1d>(&self.c, "convert_negative_overflow"),
            *Self::sym::<Fn1d>(&self.rs, "convert_negative_overflow"),
        )
    }

    pub fn get_modified_time(&self) -> (Fn2iT, Fn2iT) {
        (
            *Self::sym::<Fn2iT>(&self.c, "get_modified_time"),
            *Self::sym::<Fn2iT>(&self.rs, "get_modified_time"),
        )
    }

    pub fn hash_time_value(&self) -> (Fn1T, Fn1T) {
        (
            *Self::sym::<Fn1T>(&self.c, "hash_time_value"),
            *Self::sym::<Fn1T>(&self.rs, "hash_time_value"),
        )
    }

    pub fn modeselect(&self) -> (Fn4i, Fn4i) {
        (
            *Self::sym::<Fn4i>(&self.c, "modeselect"),
            *Self::sym::<Fn4i>(&self.rs, "modeselect"),
        )
    }
}

pub type Fn1p = unsafe extern "C" fn(*const c_char) -> c_int;
pub type Fn2i = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type Fn1d = unsafe extern "C" fn(c_double) -> c_int;
pub type Fn2iT = unsafe extern "C" fn(c_int, c_int) -> i64;
pub type Fn1T = unsafe extern "C" fn(i64) -> c_int;
pub type Fn4i = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// Serialises fd-1 redirection: the capture below rewires a process-wide file
/// descriptor, so two threads must never be inside it at the same time.
static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Run `f` with fd 1 redirected into a temp file; return its result plus the
/// captured bytes.
///
/// `fflush(NULL)` is issued on both sides of the redirect because the C and the
/// Rust library share the process-wide libc `stdout` buffer.
pub fn capture_stdout<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let mut path = std::env::temp_dir();
    path.push(format!(
        "modeselect-capture-{}-{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));

    let file = std::fs::File::create(&path).expect("create capture file");
    let fd = file.as_raw_fd();

    let result;
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(fd, 1) >= 0, "dup2 failed");

        result = f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restore dup2 failed");
        close(saved);
    }
    drop(file);

    let mut bytes = Vec::new();
    std::fs::File::open(&path)
        .expect("reopen capture file")
        .read_to_end(&mut bytes)
        .expect("read capture file");
    let _ = std::fs::remove_file(&path);

    (result, bytes)
}

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Public accessors for the two library paths (used by the symbol-parity test).
pub fn c_lib_path_pub() -> PathBuf {
    c_lib_path()
}

pub fn rust_lib_path_pub() -> PathBuf {
    rust_lib_path()
}

/// A spread of `int` values that pokes at boundaries, signs and small magnitudes.
pub fn interesting_ints() -> Vec<c_int> {
    let mut v = vec![
        0,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        4,
        -4,
        5,
        -5,
        7,
        8,
        11,
        16,
        23,
        24,
        25,
        99,
        100,
        127,
        128,
        255,
        256,
        1000,
        0xA0,
        0xDEAD,
        0xBEEF,
        65535,
        65536,
        1 << 20,
        86400,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        -86400,
        -100000,
        123456789,
        -123456789,
    ];
    v.dedup();
    v
}

/// A spread of `double` values, including out-of-`int`-range and non-finite ones.
pub fn interesting_doubles() -> Vec<c_double> {
    vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        1e-13,
        -1e-13,
        1e-12,
        2.5e-12,
        -2.5e-12,
        1e-9,
        -1e-9,
        1e-6,
        1e-3,
        1.0e-4,
        2.147483647e-3,
        2.147483648e-3,
        -2.147483648e-3,
        -2.147483649e-3,
        1.0,
        1e3,
        1e8,
        -1e8,
        1e15,
        -1e15,
        1e300,
        -1e300,
        f64::MAX,
        f64::MIN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::EPSILON,
        1.0 / 3.0,
        -1.0 / 3.0,
        2147483647.0,
        -2147483648.0,
        2147483648.0,
        -2147483649.0,
    ]
}
