//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and calls `next_double` only through the dynamic exports. The Rust function
//! is never called directly, so the `#[no_mangle] extern "C"` wrapper and the
//! `#[repr(C)]` struct layout are part of what is under test.

// Each test binary (`phase_b`, `phase_c`, `phase_d`) links the whole harness but
// uses a different subset of it, so unused-item warnings here are expected.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// Mirror of the C `cn_rnd_t`. Declared independently in the test so the test
/// does not inherit the crate's own layout assumptions.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct CnRnd {
    pub state: [u64; 2],
}

impl CnRnd {
    pub fn new(x: u64, y: u64) -> Self {
        CnRnd { state: [x, y] }
    }
}

pub type NextDouble = unsafe extern "C" fn(*mut CnRnd) -> f64;

/// One loaded implementation. `_lib` must outlive `f`, so it is kept alive here.
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: libloading::Library,
    f: NextDouble,
}

impl Impl {
    /// Advance `rnd` once through the dynamic symbol, returning the raw bits of
    /// the `double` so comparison is exact (no float equality, NaN-safe).
    pub fn next_bits(&self, rnd: &mut CnRnd) -> u64 {
        unsafe { (self.f)(rnd as *mut CnRnd) }.to_bits()
    }

    pub fn next(&self, rnd: &mut CnRnd) -> f64 {
        unsafe { (self.f)(rnd as *mut CnRnd) }
    }

    /// Raw call on an arbitrary (possibly invalid) pointer.
    pub unsafe fn call_raw(&self, p: *mut CnRnd) -> f64 {
        unsafe { (self.f)(p) }
    }
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// The CMake target (and therefore the `.so` file name) is derived from the
/// parent directory name, so glob for any `lib*.so` in `c_src/build`.
fn find_c_so() -> PathBuf {
    let build = repo_root().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    let entries = std::fs::read_dir(&build).unwrap_or_else(|e| {
        panic!(
            "cannot read {} ({e}). Build the C library first:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    });
    for e in entries.flatten() {
        let p = e.path();
        let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
        if name.starts_with("lib") && name.ends_with(".so") {
            found.push(p);
        }
    }
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {}, found {:?}",
        build.display(),
        found
    );
    found.pop().unwrap()
}

/// The Rust cdylib. `cargo test` puts test binaries in `target/<profile>/deps`,
/// so walk up to the profile dir and look for the cdylib there.
fn find_rust_so() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("test exe has a parent").to_path_buf();
    if dir.file_name().map(|s| s == "deps").unwrap_or(false) {
        dir.pop();
    }
    // Same-profile build first, then the sibling profile, then any profile dir.
    let mut tried = Vec::new();
    let mut candidates = vec![dir.join("libnext_double_lib.so")];
    if let Some(target_dir) = dir.parent() {
        for prof in ["debug", "release"] {
            candidates.push(target_dir.join(prof).join("libnext_double_lib.so"));
        }
    }
    for cand in &candidates {
        if cand.exists() {
            return cand.clone();
        }
        tried.push(cand.display().to_string());
    }

    // `cargo test` does not build a cdylib-only lib target, so build it on
    // demand into the same profile directory the tests are running from.
    let profile = dir.file_name().unwrap_or_default().to_string_lossy().to_string();
    let mut cmd = std::process::Command::new(std::env::var("CARGO").unwrap_or("cargo".into()));
    cmd.arg("build").arg("--offline").current_dir(env!("CARGO_MANIFEST_DIR"));
    if profile == "release" {
        cmd.arg("--release");
    }
    let out = cmd.output();
    if let Ok(out) = out {
        for cand in &candidates {
            if cand.exists() {
                return cand.clone();
            }
        }
        panic!(
            "Rust cdylib not found after `cargo build` (tried: {tried:?}).\ncargo stderr:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    panic!(
        "Rust cdylib not found (tried: {tried:?}). Build it first:\n  cd translation && cargo build"
    );
}

fn load(name: &'static str, path: PathBuf) -> Impl {
    let lib = unsafe { libloading::Library::new(&path) }
        .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
    let f: NextDouble = unsafe {
        let sym: libloading::Symbol<NextDouble> = lib
            .get(b"next_double\0")
            .unwrap_or_else(|e| panic!("{name}: symbol `next_double` not exported: {e}"));
        *sym
    };
    Impl { name, path, _lib: lib, f }
}

/// Both implementations, C first.
pub fn both() -> (Impl, Impl) {
    (load("C", find_c_so()), load("Rust", find_rust_so()))
}

pub fn c_so_path() -> PathBuf {
    find_c_so()
}

pub fn rust_so_path() -> PathBuf {
    find_rust_so()
}

/// True when the running test binary was built with debug assertions, which is
/// also true of the same-profile cdylib. Debug builds enable Rust's `ub_checks`
/// (null / alignment / etc.), which deliberately convert UB into a controlled
/// abort — behaviour the C, having no such checks, does not share.
pub fn tests_have_debug_assertions() -> bool {
    cfg!(debug_assertions)
}

/// The **release** cdylib — the actual shipped artifact, built without
/// `ub_checks`, and therefore the right thing to compare against the C `.so`
/// for undefined-behaviour cases such as a null pointer. Built on demand.
pub fn rust_release_so_path() -> PathBuf {
    let target_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release");
    let candidate = target_dir.join("libnext_double_lib.so");
    if candidate.exists() {
        return candidate;
    }
    let out = std::process::Command::new(std::env::var("CARGO").unwrap_or("cargo".into()))
        .args(["build", "--release", "--offline"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output();
    match out {
        Ok(out) if candidate.exists() => {
            let _ = out;
            candidate
        }
        Ok(out) => panic!(
            "release cdylib still missing at {}\ncargo stderr:\n{}",
            candidate.display(),
            String::from_utf8_lossy(&out.stderr)
        ),
        Err(e) => panic!("could not run cargo to build the release cdylib: {e}"),
    }
}

// ---------------------------------------------------------------------------
// Fixed-seed SplitMix64: reproducible randomized inputs with no extra crates.
// Deliberately a *different* algorithm from the library under test, so the test
// data generator cannot accidentally mask a bug in xorshift128+.
// ---------------------------------------------------------------------------
pub struct SplitMix64(pub u64);

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        SplitMix64(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_nonzero(&mut self) -> u64 {
        loop {
            let v = self.next_u64();
            if v != 0 {
                return v;
            }
        }
    }
}

/// Core differential assertion: run `calls` iterations from the same start
/// state on both libraries and require the returned `double` bit patterns AND
/// the resulting 128-bit state to agree at every step.
#[track_caller]
pub fn assert_same_run(c: &Impl, r: &Impl, start: CnRnd, calls: usize, ctx: &str) {
    let mut cs = start;
    let mut rs = start;
    for i in 0..calls {
        let cb = c.next_bits(&mut cs);
        let rb = r.next_bits(&mut rs);
        assert_eq!(
            cb, rb,
            "{ctx}: return value diverged at call {i} (start=[{:#018x}, {:#018x}])\n  \
             C   = {:#018x} ({})\n  Rust= {:#018x} ({})",
            start.state[0],
            start.state[1],
            cb,
            f64::from_bits(cb),
            rb,
            f64::from_bits(rb)
        );
        assert_eq!(
            cs, rs,
            "{ctx}: state diverged after call {i} (start=[{:#018x}, {:#018x}])",
            start.state[0], start.state[1]
        );
        // The C code always produces a value in [0, 1); mirror-check it here so
        // a systematic bit-level agreement on a *wrong* encoding still trips.
        let v = f64::from_bits(cb);
        assert!(
            (0.0..1.0).contains(&v),
            "{ctx}: C returned {v} outside [0,1) at call {i}"
        );
    }
    let _ = (c.name, r.name);
}
