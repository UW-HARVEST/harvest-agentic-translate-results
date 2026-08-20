//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! invoked purely through their exported C ABI symbol `dataentry`. The Rust
//! function is never called directly, so the `#[no_mangle] extern "C"` wrapper
//! is part of what gets tested.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

pub type DataEntryFn = unsafe extern "C" fn(
    std::ffi::c_int,
    std::ffi::c_int,
    std::ffi::c_int,
    std::ffi::c_int,
) -> std::ffi::c_int;

pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    c_fn: DataEntryFn,
    rust_fn: DataEntryFn,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `<manifest>/c_src/build/libtranslated_rust.so`, built by CMake.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("c_src").join("build");
    // The CMake project name is derived from the parent directory name, so the
    // library file name is not fixed. Accept any lib*.so in the build dir.
    if let Ok(entries) = std::fs::read_dir(&build) {
        let mut candidates: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                    .unwrap_or(false)
            })
            .collect();
        candidates.sort();
        if let Some(p) = candidates.pop() {
            return p;
        }
    }
    build.join("libtranslated_rust.so")
}

/// The cdylib produced for the current cargo profile, e.g.
/// `target/debug/libdataentry_lib.so`. Located relative to the test binary
/// (`target/<profile>/deps/<test>-<hash>`).
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let deps_dir = exe.parent().expect("deps dir");
    let candidates = [
        deps_dir.join("libdataentry_lib.so"),
        deps_dir
            .parent()
            .unwrap_or(Path::new("."))
            .join("libdataentry_lib.so"),
    ];
    for c in candidates.iter() {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "could not locate libdataentry_lib.so near {} — the cdylib is not produced by \
         `cargo test`, run `cargo build [--release] [--features ...]` for the same profile \
         first (or set RUST_SO_PATH)",
        deps_dir.display()
    );
}

impl Pair {
    pub fn load() -> Pair {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        assert!(
            c_path.exists(),
            "C shared library not found at {} — build it with cmake first",
            c_path.display()
        );
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));
            let c_sym: Symbol<DataEntryFn> = c_lib
                .get(b"dataentry\0")
                .expect("C .so does not export `dataentry`");
            let rust_sym: Symbol<DataEntryFn> = rust_lib
                .get(b"dataentry\0")
                .expect("Rust .so does not export `dataentry`");
            let c_fn = *c_sym;
            let rust_fn = *rust_sym;
            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_fn,
                rust_fn,
            }
        }
    }

    pub fn call_c(&self, mode: i32, p1: i32, p2: i32, p3: i32) -> i32 {
        unsafe { (self.c_fn)(mode, p1, p2, p3) }
    }

    pub fn call_rust(&self, mode: i32, p1: i32, p2: i32, p3: i32) -> i32 {
        unsafe { (self.rust_fn)(mode, p1, p2, p3) }
    }

    /// Differential assertion: byte-identical (i.e. bit-identical `int`) result.
    #[track_caller]
    pub fn assert_same(&self, ctx: &str, mode: i32, p1: i32, p2: i32, p3: i32) -> i32 {
        let c = self.call_c(mode, p1, p2, p3);
        let r = self.call_rust(mode, p1, p2, p3);
        assert_eq!(
            c, r,
            "[{ctx}] divergence for dataentry({mode}, {p1}, {p2}, {p3}): C={c} Rust={r} \
             (C bytes {:02x?}, Rust bytes {:02x?})",
            c.to_ne_bytes(),
            r.to_ne_bytes()
        );
        c
    }

    /// Differential assertion that additionally pins the expected C value.
    #[track_caller]
    pub fn assert_same_and_eq(
        &self,
        ctx: &str,
        mode: i32,
        p1: i32,
        p2: i32,
        p3: i32,
        expected: i32,
    ) {
        let c = self.assert_same(ctx, mode, p1, p2, p3);
        assert_eq!(
            c, expected,
            "[{ctx}] C returned {c} but the table expects {expected} for \
             dataentry({mode}, {p1}, {p2}, {p3})"
        );
    }
}

/// Deterministic xorshift64* PRNG, fixed seed for reproducibility.
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_ABCD_9876;

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 1 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// Mixed generator: small values, boundary values and full-range values.
    pub fn mixed_i32(&mut self) -> i32 {
        match self.next_u64() % 8 {
            0 => 0,
            1 => 1,
            2 => -1,
            3 => i32::MAX,
            4 => i32::MIN,
            5 => self.range(-16, 16),
            6 => self.range(-100_000, 100_000),
            _ => self.next_i32(),
        }
    }
}
