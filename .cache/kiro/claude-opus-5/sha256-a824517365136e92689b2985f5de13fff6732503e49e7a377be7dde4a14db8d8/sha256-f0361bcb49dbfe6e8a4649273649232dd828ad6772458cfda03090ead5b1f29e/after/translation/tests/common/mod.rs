//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls the exported
//! `memchra2` symbol on each. The Rust implementation is *never* called
//! directly — it is always reached through `libmemchra2_lib.so`, so the
//! `#[no_mangle] extern "C"` wrapper is under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::process::Command;

pub type Memchra2Signed = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;
pub type Memchra2Unsigned = unsafe extern "C" fn(u32, u32, u32, u32) -> u32;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    let build_dir = workspace_root().join("c_src").join("build");
    if !build_dir.join("CMakeCache.txt").exists() {
        std::fs::create_dir_all(&build_dir).expect("create c_src/build");
        let st = Command::new("cmake")
            .arg("..")
            .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
            .current_dir(&build_dir)
            .status()
            .expect("run cmake configure");
        assert!(st.success(), "cmake configure failed");
        let st = Command::new("cmake")
            .arg("--build")
            .arg(".")
            .current_dir(&build_dir)
            .status()
            .expect("run cmake build");
        assert!(st.success(), "cmake build failed");
    }
    let mut found = None;
    for entry in std::fs::read_dir(&build_dir).expect("read c_src/build") {
        let p = entry.expect("dir entry").path();
        if p.extension().map(|e| e == "so").unwrap_or(false) {
            found = Some(p);
            break;
        }
    }
    found.unwrap_or_else(|| panic!("no .so found in {}", build_dir.display()))
}

fn rust_so_path() -> PathBuf {
    // Allow pointing the harness at an alternative build of the cdylib (e.g.
    // the debug profile, which has overflow checks enabled) so the same suite
    // can be run against it.
    if let Ok(p) = std::env::var("MEMCHRA2_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "MEMCHRA2_RUST_SO does not exist: {}", p.display());
        return p;
    }
    // Build the cdylib explicitly: `cargo test` builds the test binaries but a
    // cdylib-only crate is not linked into them, so make sure the artifact is
    // present and fresh. Guarded by a Once so parallel tests don't contend on
    // the cargo build-directory lock.
    static BUILT: std::sync::Once = std::sync::Once::new();
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    BUILT.call_once(|| {
        let st = Command::new(env!("CARGO"))
            .args(["build", "--release", "--lib"])
            .current_dir(&manifest)
            .status()
            .expect("cargo build --release --lib");
        assert!(st.success(), "building the Rust cdylib failed");
    });
    let p = manifest
        .join("target")
        .join("release")
        .join("libmemchra2_lib.so");
    assert!(p.exists(), "missing Rust cdylib at {}", p.display());
    p
}

/// Both libraries plus the resolved `memchra2` addresses.
pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    c_signed: Memchra2Signed,
    rust_signed: Memchra2Signed,
    c_unsigned: Memchra2Unsigned,
    rust_unsigned: Memchra2Unsigned,
}

impl Pair {
    pub fn load() -> Pair {
        unsafe {
            let c_lib = Library::new(c_so_path()).expect("load C .so");
            let rust_lib = Library::new(rust_so_path()).expect("load Rust .so");

            let cs: Symbol<Memchra2Signed> =
                c_lib.get(b"memchra2\0").expect("C memchra2 (signed view)");
            let rs: Symbol<Memchra2Signed> = rust_lib
                .get(b"memchra2\0")
                .expect("Rust memchra2 (signed view)");
            let cu: Symbol<Memchra2Unsigned> =
                c_lib.get(b"memchra2\0").expect("C memchra2 (unsigned view)");
            let ru: Symbol<Memchra2Unsigned> = rust_lib
                .get(b"memchra2\0")
                .expect("Rust memchra2 (unsigned view)");

            let c_signed = *cs;
            let rust_signed = *rs;
            let c_unsigned = *cu;
            let rust_unsigned = *ru;

            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_signed,
                rust_signed,
                c_unsigned,
                rust_unsigned,
            }
        }
    }

    pub fn c(&self, a: i32, b: i32, c: i32, d: i32) -> i32 {
        unsafe { (self.c_signed)(a, b, c, d) }
    }

    pub fn rust(&self, a: i32, b: i32, c: i32, d: i32) -> i32 {
        unsafe { (self.rust_signed)(a, b, c, d) }
    }

    pub fn c_u(&self, a: u32, b: u32, c: u32, d: u32) -> u32 {
        unsafe { (self.c_unsigned)(a, b, c, d) }
    }

    pub fn rust_u(&self, a: u32, b: u32, c: u32, d: u32) -> u32 {
        unsafe { (self.rust_unsigned)(a, b, c, d) }
    }

    /// Raw code address of the C `memchra2` (harness self-check).
    pub fn c_fn_addr(&self) -> usize {
        self.c_signed as usize
    }

    /// Raw code address of the Rust `memchra2` (harness self-check).
    pub fn rust_fn_addr(&self) -> usize {
        self.rust_signed as usize
    }

    /// Assert byte-identical results for one tuple.
    pub fn assert_eq_at(&self, label: &str, a: i32, b: i32, c: i32, d: i32) {
        let got_c = self.c(a, b, c, d);
        let got_r = self.rust(a, b, c, d);
        assert_eq!(
            got_c.to_le_bytes(),
            got_r.to_le_bytes(),
            "[{label}] memchra2({a}, {b}, {c}, {d}): C = {got_c} (0x{got_c:08x}), \
             Rust = {got_r} (0x{got_r:08x})"
        );
    }

    /// Run a whole randomized row: `n` tuples produced by `gen`.
    pub fn run_row<F>(&self, label: &str, n: usize, mut gen: F)
    where
        F: FnMut(&mut Rng) -> (i32, i32, i32, i32),
    {
        let mut rng = Rng::new(FIXED_SEED ^ fnv(label.as_bytes()));
        for _ in 0..n {
            let (a, b, c, d) = gen(&mut rng);
            self.assert_eq_at(label, a, b, c, d);
        }
    }
}

pub const FIXED_SEED: u64 = 0x5EED_1234_ABCD_9876;

fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// Deterministic SplitMix64 — reproducible across runs and platforms.
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

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in `[lo, hi]` inclusive, over the full i32 range safely.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64) as u64 + 1;
        let v = self.next_u64() % span;
        (lo as i64 + v as i64) as i32
    }

    /// Uniform in `[lo, hi]` inclusive over u32 bit patterns.
    pub fn range_u32(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u32
    }

    /// A magnitude in `[lo, hi]`, with the sign taken from `neg`.
    pub fn signed_mag(&mut self, neg: bool, lo: i32, hi: i32) -> i32 {
        let m = self.range_i32(lo, hi);
        if neg {
            m.wrapping_neg()
        } else {
            m
        }
    }
}

/// Bit pattern of `1.0f32` — lower edge of the `(int)f >= 1` sub-window.
pub const BITS_1_0: u32 = 0x3F80_0000;
/// Bit pattern of `1000.0f32` — first value rejected by `f < 1000.0f`.
pub const BITS_1000_0: u32 = 0x447A_0000;
