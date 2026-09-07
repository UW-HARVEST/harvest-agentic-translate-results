#![allow(dead_code)] // each integration-test crate uses a different subset

//! Shared harness: loads BOTH the C `.so` and the Rust `.so` with `libloading`
//! and calls `dataentry` through the dynamic-symbol boundary in both cases.
//!
//! The Rust function is NEVER called directly — only via its exported
//! `#[no_mangle]` symbol in `libdataentry_lib.so`, exactly as an external C
//! consumer would.

use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

pub type DataEntryFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

pub struct Libs {
    pub c: Library,
    pub rs: Library,
    pub c_path: PathBuf,
    pub rs_path: PathBuf,
}

fn first_so_in(dir: &Path) -> Option<PathBuf> {
    let mut hits: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    hits.sort();
    hits.into_iter().next()
}

fn c_so_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let build_dir = manifest.parent().unwrap().join("c_src").join("build");
    first_so_in(&build_dir).unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build \
             && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    })
}

fn rust_so_path() -> PathBuf {
    // current_exe is <target>/<profile>/deps/<testbin>; the cdylib lives in
    // <target>/<profile>/.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("profile dir")
        .to_path_buf();

    let candidates = [
        profile_dir.join("libdataentry_lib.so"),
        profile_dir.join("dataentry_lib.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }

    // `cargo test` builds the lib target as an rlib and does NOT emit the
    // cdylib, so on a clean tree the `.so` is missing. Build it for the profile
    // this test binary was compiled with. (Cargo's build lock is already
    // released by the time test binaries run.)
    let release = profile_dir
        .file_name()
        .map(|n| n == "release")
        .unwrap_or(false);
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(&cargo);
    cmd.arg("build")
        .arg("--lib")
        .current_dir(env!("CARGO_MANIFEST_DIR"));
    if release {
        cmd.arg("--release");
    }
    let status = cmd.status();
    match status {
        Ok(s) if s.success() => {}
        other => panic!(
            "cdylib missing and `{cargo} build --lib{}` failed ({other:?}). \
             Build it manually: cd translation && cargo build{}",
            if release { " --release" } else { "" },
            if release { " --release" } else { "" }
        ),
    }

    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib still not found; looked for {:?} in {}",
        candidates,
        profile_dir.display()
    );
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rs_path = rust_so_path();
        // SAFETY: both objects are plain C-ABI libraries with no initializers
        // that run arbitrary code beyond their own static setup.
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
        let rs = unsafe { Library::new(&rs_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", rs_path.display()));
        Libs {
            c,
            rs,
            c_path,
            rs_path,
        }
    })
}

pub fn c_fn() -> Symbol<'static, DataEntryFn> {
    unsafe { libs().c.get(b"dataentry\0") }.expect("C .so does not export `dataentry`")
}

pub fn rs_fn() -> Symbol<'static, DataEntryFn> {
    unsafe { libs().rs.get(b"dataentry\0") }.expect("Rust .so does not export `dataentry`")
}

/// Calls both exports with the same arguments and asserts bit-identical results.
pub fn assert_same(row: &str, mode: c_int, p1: c_int, p2: c_int, p3: c_int) {
    let c = c_fn();
    let r = rs_fn();
    let cv = unsafe { c(mode, p1, p2, p3) };
    let rv = unsafe { r(mode, p1, p2, p3) };
    assert_eq!(
        cv, rv,
        "[{row}] dataentry({mode}, {p1}, {p2}, {p3}): C returned {cv}, Rust returned {rv}"
    );
}

/// Deterministic xorshift64* PRNG so every "randomized" run is reproducible.
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
    /// Full-range `i32`, including `INT_MIN`/`INT_MAX`.
    pub fn i32(&mut self) -> c_int {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Uniform in `[lo, hi]` inclusive; `lo <= hi` required.
    pub fn range(&mut self, lo: i64, hi: i64) -> c_int {
        let span = (hi - lo + 1) as u64;
        (lo + (self.next_u64() % span) as i64) as c_int
    }
}
