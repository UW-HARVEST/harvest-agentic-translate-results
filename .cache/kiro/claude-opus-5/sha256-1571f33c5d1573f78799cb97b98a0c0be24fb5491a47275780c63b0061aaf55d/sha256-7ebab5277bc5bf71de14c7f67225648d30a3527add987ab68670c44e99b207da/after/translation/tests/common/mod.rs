//! Shared helpers: locate and load the C and Rust shared libraries and expose
//! their `crc16` exports through libloading.
//!
//! Both implementations are always reached through `dlopen`/`dlsym` so that the
//! `#[no_mangle]` export wrapper is part of what gets tested.

// Each integration test binary includes this module but only uses part of it.
#![allow(dead_code)]

use std::path::{Path, PathBuf};


use libloading::{Library, Symbol};

/// `tflac_u16 crc16(const tflac_u8 *d, tflac_u32 len, tflac_u16 crc16)`
pub type Crc16Fn = unsafe extern "C" fn(*const u8, u32, u16) -> u16;

/// Workspace root (the directory holding `c_src/` and `translation/`).
fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Directory that holds the freshly built Rust artifacts (`target/<profile>`).
///
/// Derived from the running test binary: `target/<profile>/deps/<test>`.
fn rust_artifact_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    profile.to_path_buf()
}

fn find_so(dir: &Path, hint: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut best: Option<PathBuf> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name()?.to_string_lossy().to_string();
        if !name.starts_with("lib") || !name.ends_with(".so") {
            continue;
        }
        if !hint.is_empty() && !name.contains(hint) {
            continue;
        }
        best = Some(path);
        break;
    }
    best
}

/// Path to the C shared library built from `c_src/`.
pub fn c_library_path() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    if let Some(p) = find_so(&build, "") {
        return p;
    }
    build_c_library(&build);
    if let Some(p) = find_so(&build, "") {
        return p;
    }
    panic!(
        "no C shared library found in {}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
}

/// Configure and build the C library (out-of-tree, into `c_src/build`).
fn build_c_library(build: &Path) {
    use std::process::Command;

    let src = workspace_root().join("c_src");
    if std::fs::create_dir_all(build).is_err() {
        return;
    }
    let configure = Command::new("cmake")
        .arg("-S")
        .arg(&src)
        .arg("-B")
        .arg(build)
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .output();
    match configure {
        Ok(o) if o.status.success() => {}
        Ok(o) => panic!(
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(e) => panic!("failed to spawn cmake: {e}"),
    }
    let out = Command::new("cmake")
        .arg("--build")
        .arg(build)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn cmake --build: {e}"));
    assert!(
        out.status.success(),
        "cmake --build failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Path to the Rust `cdylib` built from `translation/`.
///
/// `cargo test` does not necessarily (re)emit the `cdylib` — integration tests
/// do not link against a cdylib-only lib target, so cargo has no reason to
/// build or refresh it. We therefore always run `cargo build --lib` once per
/// test process before loading, so the `.so` under test is never stale.
pub fn rust_library_path() -> PathBuf {
    static BUILT: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    let dir = rust_artifact_dir();
    BUILT.get_or_init(build_rust_cdylib);

    if let Some(p) = existing_rust_so(&dir) {
        return p;
    }
    panic!(
        "no Rust cdylib (libcrc16_lib.so) found in {} even after `cargo build`",
        dir.display()
    );
}

fn existing_rust_so(dir: &Path) -> Option<PathBuf> {
    let direct = dir.join("libcrc16_lib.so");
    if direct.exists() {
        return Some(direct);
    }
    find_so(dir, "crc16_lib")
}

/// Invoke cargo to produce the cdylib for the profile the test is running under.
///
/// The feature set is taken from `CRC16_TEST_FEATURES` /
/// `CRC16_TEST_NO_DEFAULT_FEATURES` when the driver script sets them, so the
/// `.so` under test always matches the configuration being verified.
fn build_rust_cdylib() {
    use std::process::Command;

    let profile_dir = rust_artifact_dir();
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let profile_name = profile_dir
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "debug".to_string());

    let mut cmd = Command::new(cargo);
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
    cmd.arg("build").arg("--lib");
    if profile_name == "debug" {
        cmd.args(["--profile", "dev"]);
    } else {
        cmd.args(["--profile", &profile_name]);
    }
    if std::env::var_os("CRC16_TEST_NO_DEFAULT_FEATURES").is_some() {
        cmd.arg("--no-default-features");
    }
    if let Ok(feats) = std::env::var("CRC16_TEST_FEATURES")
        && !feats.is_empty()
    {
        cmd.args(["--features", &feats]);
    }
    // Avoid inheriting the outer cargo's env, which can confuse nested builds.
    for key in [
        "RUSTC",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "RUSTDOC",
        "CARGO_MAKEFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
    ] {
        cmd.env_remove(key);
    }

    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn cargo build for the cdylib: {e}"));
    assert!(
        out.status.success(),
        "nested `cargo build --lib` failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A loaded implementation plus its resolved `crc16` symbol address.
pub struct Impl {
    _lib: Library,
    f: Crc16Fn,
}

impl Impl {
    pub fn load(path: &Path) -> Self {
        // SAFETY: loading a locally built library with a C ABI entry point.
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        let f = {
            // SAFETY: symbol type matches the C declaration in c_src/include/lib.h.
            let sym: Symbol<Crc16Fn> = unsafe { lib.get(b"crc16\0") }
                .unwrap_or_else(|e| panic!("no `crc16` symbol in {}: {e}", path.display()));
            *sym
        };
        Impl { _lib: lib, f }
    }

    /// Call `crc16` through the FFI boundary.
    ///
    /// # Safety
    /// `d` must point to `len` readable bytes (or `len` must be 0).
    pub unsafe fn crc16(&self, d: *const u8, len: u32, crc: u16) -> u16 {
        unsafe { (self.f)(d, len, crc) }
    }

    /// Convenience wrapper for a slice.
    pub fn crc16_slice(&self, data: &[u8], crc: u16) -> u16 {
        // SAFETY: pointer/length pair comes from a live slice.
        unsafe { self.crc16(data.as_ptr(), data.len() as u32, crc) }
    }
}

/// Both implementations, loaded and ready to compare.
pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

pub fn load_pair() -> Pair {
    Pair {
        c: Impl::load(&c_library_path()),
        rust: Impl::load(&rust_library_path()),
    }
}

/// Deterministic pseudo-random byte generator (xorshift64*), so failures are
/// reproducible without pulling in a rand dependency.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }

    pub fn next_u16(&mut self) -> u16 {
        (self.next_u64() >> 32) as u16
    }

    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next_u8()).collect()
    }
}
