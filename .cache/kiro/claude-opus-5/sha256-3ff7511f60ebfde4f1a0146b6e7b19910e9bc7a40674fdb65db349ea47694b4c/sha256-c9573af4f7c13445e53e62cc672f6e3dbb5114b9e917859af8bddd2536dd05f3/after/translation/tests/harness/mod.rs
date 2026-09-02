//! Shared harness: loads BOTH shared objects via `libloading` and exposes the
//! `pow43` export of each.
//!
//! Nothing here calls the Rust implementation directly — every Rust-side call
//! goes through `dlopen`/`dlsym` on `libpow43_lib.so`, exactly as an external C
//! consumer would, so the `#[no_mangle] extern "C"` wrapper is under test too.

// Each integration-test binary compiles this whole module but uses only the
// part it needs, so unused-item warnings here are expected and meaningless.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;

pub type Pow43Fn = unsafe extern "C" fn(std::ffi::c_int) -> f32;

pub struct Pair {
    _c_lib: Library,
    _rs_lib: Library,
    c: Pow43Fn,
    rs: Pow43Fn,
    pub c_path: PathBuf,
    pub rs_path: PathBuf,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|s| s == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    found.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // The integration test binary lives in target/<profile>/deps/, so the
    // cdylib sits one directory up. Fall back to the well-known locations.
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            if let Some(profile_dir) = deps.parent() {
                candidates.push(profile_dir.join("libpow43_lib.so"));
            }
        }
    }
    let root = workspace_root().join("translation/target");
    candidates.push(root.join("release/libpow43_lib.so"));
    candidates.push(root.join("debug/libpow43_lib.so"));

    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "no Rust cdylib found; looked in {:?}. Build it with `cargo build --release`.",
        candidates
    );
}

impl Pair {
    pub fn load() -> Pair {
        let c_path = find_c_so();
        let rs_path = find_rust_so();
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rs_lib = Library::new(&rs_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rs_path.display()));

            let c_sym: Symbol<Pow43Fn> = c_lib
                .get(b"pow43\0")
                .expect("C .so does not export `pow43`");
            let rs_sym: Symbol<Pow43Fn> = rs_lib
                .get(b"pow43\0")
                .expect("Rust .so does not export `pow43` (missing #[no_mangle]?)");

            let c = *c_sym;
            let rs = *rs_sym;
            Pair {
                _c_lib: c_lib,
                _rs_lib: rs_lib,
                c,
                rs,
                c_path,
                rs_path,
            }
        }
    }

    #[inline]
    pub fn c(&self, x: i32) -> f32 {
        unsafe { (self.c)(x) }
    }

    #[inline]
    pub fn rs(&self, x: i32) -> f32 {
        unsafe { (self.rs)(x) }
    }

    /// Bit-exact differential assertion. Compares `to_bits()` so that `-0.0`
    /// vs `+0.0` and distinct NaN payloads are *not* treated as equal.
    #[inline]
    #[track_caller]
    pub fn assert_same(&self, x: i32, row: &str) {
        let a = self.c(x);
        let b = self.rs(x);
        if a.to_bits() != b.to_bits() {
            panic!(
                "[{row}] divergence at x = {x}:\n  C    = {a:?} (bits 0x{:08x})\n  Rust = {b:?} (bits 0x{:08x})",
                a.to_bits(),
                b.to_bits()
            );
        }
    }

    /// Compare over an iterator of inputs, reporting the first divergence.
    pub fn assert_same_all<I: IntoIterator<Item = i32>>(&self, xs: I, row: &str) -> usize {
        let mut n = 0usize;
        for x in xs {
            self.assert_same(x, row);
            n += 1;
        }
        assert!(n > 0, "[{row}] test drove zero inputs");
        n
    }
}

/// The defined-behaviour domain, derived in ERRORS.md:
/// every `x` for which the C code's table index stays within `[0, 144]`.
pub const DOMAIN_LO: i32 = -16;
pub const DOMAIN_HI: i32 = 8223;

/// Reimplementation of the C code's index computation, used only to *classify*
/// inputs in the tests (never to produce an expected value).
pub fn c_table_index(x: i32) -> i32 {
    if x < 129 {
        return 16i32.wrapping_add(x);
    }
    let mut x = x;
    if x < 1024 {
        x = x.wrapping_shl(3);
    }
    let sign = x.wrapping_mul(2) & 64;
    16i32.wrapping_add(x.wrapping_add(sign) >> 6)
}

pub fn in_bounds(x: i32) -> bool {
    let i = c_table_index(x);
    (0..=144).contains(&i)
}

/// Deterministic SplitMix64 — fixed seed, reproducible randomized inputs.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
}

pub const SEED: u64 = 0x5EED_1234;

/// How many randomized inputs each property-style row drives.
pub const N_RANDOM: usize = 20_000;
