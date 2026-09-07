//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `cdylib` with `libloading` and calls
//! `pow43` through the FFI boundary in both, so the `#[no_mangle]` export
//! wrapper is exercised exactly as an external consumer would.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

pub type Pow43Fn = unsafe extern "C" fn(i32) -> f32;

pub struct Libs {
    _c: Library,
    _rust: Library,
    pub c_pow43: Pow43Fn,
    pub rust_pow43: Pow43Fn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Directory that holds the freshly built artifacts for this test run
/// (`target/debug` or `target/release`, whichever cargo used).
fn artifact_dir() -> PathBuf {
    // current_exe is <target>/<profile>/deps/<testname>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    profile.to_path_buf()
}

const RUST_SO_NAMES: [&str; 3] = ["libpow43_lib.so", "libpow43_lib.dylib", "pow43_lib.dll"];

fn probe_rust_so(dir: &Path) -> Option<PathBuf> {
    RUST_SO_NAMES
        .iter()
        .map(|n| dir.join(n))
        .find(|p| p.is_file())
}

/// Locate the Rust cdylib produced from `src/lib.rs` (`[lib] name = "pow43_lib"`).
///
/// `cargo test` does **not** emit the `cdylib` artifact (integration tests do
/// not link against it — we `dlopen` it instead), so if it is missing we build
/// it on demand for the *same profile* the test binary was built with. That
/// keeps a bare `cargo test` working without a separate `cargo build` step, in
/// both `debug` and `release`.
fn find_rust_so() -> PathBuf {
    let dir = artifact_dir();
    if let Some(p) = probe_rust_so(&dir) {
        return p;
    }

    let profile = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("debug")
        .to_string();
    let mut cmd = std::process::Command::new(std::env::var("CARGO").unwrap_or("cargo".into()));
    cmd.arg("build").arg("--offline").arg("--lib");
    if profile == "release" {
        cmd.arg("--release");
    }
    cmd.current_dir(manifest_dir());
    // Avoid inheriting the running test's cargo environment, which would make
    // the nested invocation try to reuse the parent's build plan.
    for k in [
        "RUSTC_WORKSPACE_WRAPPER",
        "CARGO_MAKEFLAGS",
        "RUSTC_WRAPPER",
        "CARGO_PRIMARY_PACKAGE",
    ] {
        cmd.env_remove(k);
    }
    let out = cmd.output().expect("spawn `cargo build --lib` for the cdylib");
    assert!(
        out.status.success(),
        "failed to build the Rust cdylib for profile `{profile}`:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    probe_rust_so(&dir).unwrap_or_else(|| {
        panic!(
            "Rust cdylib still not found in {} after `cargo build --lib` \
             (contents: {:?}). Build it manually with `cargo build --release`.",
            dir.display(),
            std::fs::read_dir(&dir).map(|d| d
                .filter_map(|e| e.ok())
                .map(|e| e.file_name())
                .collect::<Vec<_>>())
        )
    })
}

/// Locate the C shared library built by `c_src/CMakeLists.txt`.
///
/// The CMake project name is derived from the *parent directory name* of
/// `c_src`, so the file name is not fixed; scan the build tree for any
/// `lib*.so` that exports `pow43`.
fn find_c_so() -> PathBuf {
    // Allow the matrix runner (`run_all.sh`) to point at a C library built
    // with different optimisation flags, without touching c_src/.
    if let Ok(p) = std::env::var("POW43_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "POW43_C_SO={} is not a file", p.display());
        return p;
    }
    let c_root = manifest_dir()
        .parent()
        .expect("workspace root")
        .join("c_src");
    let mut candidates = Vec::new();
    collect_shared_libs(&c_root, &mut candidates, 0);
    candidates.sort();
    for cand in &candidates {
        // Only accept a library that actually exports `pow43`.
        if unsafe {
            Library::new(cand)
                .ok()
                .map(|l| l.get::<Pow43Fn>(b"pow43\0").is_ok())
                .unwrap_or(false)
        } {
            return cand.clone();
        }
    }
    panic!(
        "no C shared library exporting `pow43` found under {} (candidates: {:?}).\n\
         Build it with:\n  cd c_src && mkdir -p build && cd build && \\\n\
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        c_root.display(),
        candidates
    );
}

fn collect_shared_libs(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.filter_map(|e| e.ok()) {
        let p = e.path();
        if p.is_dir() {
            collect_shared_libs(&p, out, depth + 1);
        } else if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if name.ends_with(".so") || name.ends_with(".dylib") || name.ends_with(".dll") {
                out.push(p);
            }
        }
    }
}

impl Libs {
    pub fn load() -> Self {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        unsafe {
            let c = Library::new(&c_path).expect("dlopen C library");
            let rust = Library::new(&rust_path).expect("dlopen Rust cdylib");
            let c_sym: Symbol<Pow43Fn> = c.get(b"pow43\0").expect("C pow43");
            let rust_sym: Symbol<Pow43Fn> = rust.get(b"pow43\0").expect("Rust pow43");
            let c_pow43 = *c_sym;
            let rust_pow43 = *rust_sym;
            drop(c_sym);
            drop(rust_sym);
            Libs {
                _c: c,
                _rust: rust,
                c_pow43,
                rust_pow43,
                c_path,
                rust_path,
            }
        }
    }

    #[inline]
    pub fn c(&self, x: i32) -> f32 {
        unsafe { (self.c_pow43)(x) }
    }

    #[inline]
    pub fn rust(&self, x: i32) -> f32 {
        unsafe { (self.rust_pow43)(x) }
    }

    /// Compare bit-for-bit (so `-0.0` vs `+0.0` and NaN payloads are caught).
    #[inline]
    pub fn assert_same(&self, x: i32, ctx: &str) {
        let c = self.c(x);
        let r = self.rust(x);
        assert_eq!(
            c.to_bits(),
            r.to_bits(),
            "{ctx}: pow43({x}) diverged: C = {c:?} ({:#010x}) vs Rust = {r:?} ({:#010x})",
            c.to_bits(),
            r.to_bits()
        );
    }
}

/// Deterministic xorshift64* PRNG so every "randomized" row is reproducible.
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `lo..=hi` (inclusive), works across the full i32 range.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64) as u64 + 1;
        let v = self.next_u64() % span;
        (lo as i64 + v as i64) as i32
    }
}
