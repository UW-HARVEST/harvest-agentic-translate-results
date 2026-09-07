//! Shared differential-test harness.
//!
//! BOTH libraries are loaded as shared objects through `libloading` and called
//! only through their exported C ABI symbols. The Rust crate is never linked
//! directly, so the `#[no_mangle] extern "C"` wrapper is under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

/// Mirror of the C `cn_rnd_t`.
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

pub type NextDoubleFn = unsafe extern "C" fn(*mut CnRnd) -> f64;

/// One loaded implementation.
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    next_double: NextDoubleFn,
}

impl Impl {
    /// One step: returns the raw bits of the produced `double` plus the
    /// post-call state. Comparing raw bits (not `f64` equality) is what makes
    /// this byte-for-byte: it distinguishes `+0.0`/`-0.0` and every NaN payload.
    pub fn step(&self, rnd: &mut CnRnd) -> u64 {
        let r = unsafe { (self.next_double)(rnd as *mut CnRnd) };
        r.to_bits()
    }

    pub fn step_f(&self, rnd: &mut CnRnd) -> f64 {
        unsafe { (self.next_double)(rnd as *mut CnRnd) }
    }

    /// One step through a deliberately misaligned pointer.
    pub unsafe fn step_raw(&self, p: *mut CnRnd) -> u64 {
        unsafe { (self.next_double)(p) }.to_bits()
    }

    /// Run `n` steps from `seed`; return every raw output plus the final state.
    pub fn run(&self, seed: CnRnd, n: usize) -> (Vec<u64>, CnRnd) {
        let mut s = seed;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            out.push(self.step(&mut s));
        }
        (out, s)
    }
}

fn newest_existing(cands: &[PathBuf]) -> Option<PathBuf> {
    cands
        .iter()
        .filter(|p| p.exists())
        .max_by_key(|p| {
            std::fs::metadata(p)
                .and_then(|m| m.modified())
                .ok()
        })
        .cloned()
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    manifest_dir().parent().expect("manifest has a parent").to_path_buf()
}

/// Locate the C `.so` produced by CMake. The library name is derived from the
/// parent directory name in `c_src/CMakeLists.txt`, so it is discovered by glob
/// rather than hard-coded.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    collect_so(&build, &mut found, 0);
    // Prefer a plain `lib*.so` at the top of the build dir.
    found.sort();
    found
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no C .so under {}; build it with cmake first", build.display()))
}

fn collect_so(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 3 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_so(&p, out, depth + 1);
        } else if p.extension().map(|x| x == "so").unwrap_or(false) {
            out.push(p);
        }
    }
}

/// Locate the Rust cdylib. Never links the crate: loads the built `.so`.
///
/// The `.so` for the profile the TEST was built with is chosen deliberately
/// (not "whichever file is newest"): `debug_assertions` changes observable
/// behaviour at the FFI boundary (e.g. alignment / overflow checks), so a debug
/// test run must exercise the debug `.so` and a `--release` run the release one.
/// Picking by mtime silently hid a real divergence once already.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let t = manifest_dir().join("target");
    let name = "libnext_double_lib.so";
    let want = profile_dir();
    let primary = t.join(&want).join(name);
    if primary.exists() {
        return primary;
    }
    newest_existing(&[t.join("debug").join(name), t.join("release").join(name)]).unwrap_or_else(
        || {
            panic!(
                "no Rust cdylib found under {} (wanted the `{want}` profile)",
                t.display()
            )
        },
    )
}

/// The cargo profile directory this test binary was built into (`debug` /
/// `release`), read off `current_exe()` (`target/<profile>/deps/<test>`).
///
/// `cfg!(debug_assertions)` must NOT be used for this: `[profile.dev]` in
/// `Cargo.toml` turns debug-assertions off (for C parity), and the test profile
/// inherits that, so the cfg would report `release` for a debug build.
pub fn profile_dir() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| {
            // .../target/<profile>/deps/<exe>  ->  <profile>
            p.parent()
                .filter(|d| d.file_name().map(|n| n == "deps").unwrap_or(false))
                .and_then(|d| d.parent())
                .or_else(|| p.parent())
                .and_then(|d| d.file_name())
                .map(|n| n.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "debug".to_string())
}

fn load(name: &'static str, path: PathBuf) -> Impl {
    let lib = unsafe { Library::new(&path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", path.display(), name));
    let next_double: NextDoubleFn = unsafe {
        let s: Symbol<NextDoubleFn> = lib
            .get(b"next_double\0")
            .unwrap_or_else(|e| panic!("{name}: symbol `next_double` missing from {}: {e}", path.display()));
        *s
    };
    Impl { name, path, _lib: lib, next_double }
}

/// The pair under comparison: (C reference, Rust translation).
pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

pub fn pair() -> Pair {
    Pair {
        c: load("C", c_so_path()),
        rs: load("Rust", rust_so_path()),
    }
}

impl Pair {
    /// Core differential assertion for a single seed: identical raw output bits
    /// AND identical mutated state, for `n` consecutive steps.
    pub fn assert_same(&self, seed: CnRnd, n: usize, ctx: &str) {
        let mut cs = seed;
        let mut rs = seed;
        for i in 0..n {
            let cv = self.c.step(&mut cs);
            let rv = self.rs.step(&mut rs);
            assert_eq!(
                cv, rv,
                "{ctx}: return bits differ at step {i} (seed = {:#018x},{:#018x}): \
                 C = {cv:#018x} ({}), Rust = {rv:#018x} ({})",
                seed.state[0], seed.state[1],
                f64::from_bits(cv), f64::from_bits(rv)
            );
            assert_eq!(
                cs, rs,
                "{ctx}: post-call state differs at step {i} (seed = {:#018x},{:#018x}): \
                 C = {:#018x},{:#018x}, Rust = {:#018x},{:#018x}",
                seed.state[0], seed.state[1],
                cs.state[0], cs.state[1], rs.state[0], rs.state[1]
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG for property-style inputs (fixed seed => reproducible).
// ---------------------------------------------------------------------------

pub const FIXED_SEED: u64 = 0x2545_F491_4F6C_DD1D;

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
    pub fn seed_pair(&mut self) -> CnRnd {
        CnRnd::new(self.next_u64(), self.next_u64())
    }
}

pub fn rng() -> SplitMix64 {
    SplitMix64::new(FIXED_SEED)
}

// ---------------------------------------------------------------------------
// Test-local model of the C arithmetic, used ONLY to *construct* seeds that
// land on a wanted code path (e.g. mantissa == 0). It is never used as the
// expected value -- the C .so is always the ground truth.
// ---------------------------------------------------------------------------

/// `x ^= x << 23; x ^= x >> 17; x ^= y ^ (y >> 26);` and `return x + y`.
pub fn model_step(s: CnRnd) -> (u64, CnRnd) {
    let mut x = s.state[0];
    let y = s.state[1];
    x ^= x << 23;
    x ^= x >> 17;
    x ^= y ^ (y >> 26);
    (x.wrapping_add(y), CnRnd::new(y, x))
}

fn inv_xor_lshift(b: u64, s: u32) -> u64 {
    let mut a = b;
    for _ in 0..5 {
        a = b ^ (a << s);
    }
    a
}

fn inv_xor_rshift(b: u64, s: u32) -> u64 {
    let mut a = b;
    for _ in 0..5 {
        a = b ^ (a >> s);
    }
    a
}

/// Build a seed whose FIRST step produces exactly `value` as the raw
/// `x + y` sum, for a caller-chosen `y`.
///
/// `x_final = g(x0) ^ y ^ (y >> 26)` with `g(x) = (x ^ (x<<23))` then
/// `^= >>17`; `g` is invertible, so `x0 = g^-1(x_final ^ y ^ (y>>26))`.
pub fn seed_for_value(value: u64, y: u64) -> CnRnd {
    let x_final = value.wrapping_sub(y);
    let t = x_final ^ y ^ (y >> 26);
    let x0 = inv_xor_lshift(inv_xor_rshift(t, 17), 23);
    CnRnd::new(x0, y)
}
