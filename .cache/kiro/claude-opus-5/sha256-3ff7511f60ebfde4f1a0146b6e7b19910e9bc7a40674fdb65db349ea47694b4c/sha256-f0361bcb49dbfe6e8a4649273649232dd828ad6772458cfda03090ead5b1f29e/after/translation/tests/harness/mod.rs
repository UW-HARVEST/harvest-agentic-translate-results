//! Shared harness: loads BOTH the C `.so` and the Rust `.so` with `libloading`
//! and exposes `pow43` from each strictly through its exported symbol.
//!
//! Nothing here calls a Rust function directly — the Rust side is reached only
//! via `dlsym("pow43")` on `libpow43_lib.so`, exactly as an external C consumer
//! would, so the `#[no_mangle] extern "C"` wrapper is under test too.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub type Pow43 = unsafe extern "C" fn(c_int) -> f32;

pub struct Libs {
    _c: libloading::Library,
    _rust: libloading::Library,
    pub c_pow43: Pow43,
    pub rust_pow43: Pow43,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<parent-dir-name>.so` — the CMake project name is derived
/// from the *parent* directory name, so glob for it instead of hardcoding.
pub fn find_c_so() -> PathBuf {
    let build = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                found.push(p);
            }
        }
    }
    found.sort();
    assert!(
        !found.is_empty(),
        "no C shared library found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    found.remove(0)
}

/// The Rust `cdylib`. Prefer the profile dir the running test binary lives in
/// (`target/<profile>/`), fall back to the other profile.
pub fn find_rust_so() -> PathBuf {
    const NAME: &str = "libpow43_lib.so";

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        // target/<profile>/deps/<test-bin> -> target/<profile>
        if let Some(profile_dir) = exe.parent().and_then(Path::parent) {
            candidates.push(profile_dir.join(NAME));
        }
    }
    let target = manifest_dir().join("target");
    candidates.push(target.join("debug").join(NAME));
    candidates.push(target.join("release").join(NAME));

    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib {NAME} not found. Looked in: {:?}. Build it with `cargo build` / `cargo build --release`.",
        candidates
    );
}

fn load() -> Libs {
    let c_path = find_c_so();
    let rust_path = find_rust_so();

    unsafe {
        let c = libloading::Library::new(&c_path)
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
        let rust = libloading::Library::new(&rust_path)
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));

        let c_sym: libloading::Symbol<Pow43> = c
            .get(b"pow43\0")
            .unwrap_or_else(|e| panic!("dlsym pow43 in C .so: {e}"));
        let r_sym: libloading::Symbol<Pow43> = rust
            .get(b"pow43\0")
            .unwrap_or_else(|e| panic!("dlsym pow43 in Rust .so: {e}"));

        let c_pow43 = *c_sym;
        let rust_pow43 = *r_sym;

        Libs { _c: c, _rust: rust, c_pow43, rust_pow43, c_path, rust_path }
    }
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(load)
}

/// Call both exported `pow43` symbols with the same argument.
pub fn both(x: c_int) -> (f32, f32) {
    let l = libs();
    unsafe { ((l.c_pow43)(x), (l.rust_pow43)(x)) }
}

/// Bit-exact comparison (so `-0.0 != 0.0` and NaN payloads are compared too).
pub fn assert_bit_eq(x: c_int, row: &str) {
    let (c, r) = both(x);
    assert_eq!(
        c.to_bits(),
        r.to_bits(),
        "[{row}] pow43({x}): C = {c:?} (bits {:#010x}) but Rust = {r:?} (bits {:#010x})",
        c.to_bits(),
        r.to_bits()
    );
}

pub fn assert_all_bit_eq<I: IntoIterator<Item = c_int>>(xs: I, row: &str) -> usize {
    let mut n = 0usize;
    for x in xs {
        assert_bit_eq(x, row);
        n += 1;
    }
    assert!(n > 0, "[{row}] no inputs were exercised");
    n
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

pub const SEED: u64 = 0x2545_F491_4F6C_DD1D;

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        debug_assert!(lo <= hi);
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
}

/// Randomized inputs in `[lo, hi]` that satisfy `pred`, with a fixed seed.
pub fn sample_where(
    seed_tweak: u64,
    lo: i32,
    hi: i32,
    count: usize,
    pred: impl Fn(i32) -> bool,
) -> Vec<c_int> {
    let mut rng = Rng::new(SEED ^ seed_tweak);
    let mut out = Vec::with_capacity(count);
    let mut guard = 0u64;
    while out.len() < count {
        guard += 1;
        assert!(guard < 100_000_000, "predicate too sparse in [{lo}, {hi}]");
        let v = rng.range(lo as i64, hi as i64) as i32;
        if pred(v) {
            out.push(v);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Domain constants, derived from the C source (see ERRORS.md).
// ---------------------------------------------------------------------------

/// `g_pow43[16 + x]` with `x >= -16` ⇒ smallest defined input.
pub const DOMAIN_MIN: i32 = -16;
/// Largest `x` whose computed table index is still `<= 144`.
pub const DOMAIN_MAX: i32 = 8223;
/// `if (x < 129)` — the early-return branch boundary.
pub const BRANCH_A: i32 = 129;
/// `if (x < 1024)` — the `mult`/shift branch boundary.
pub const BRANCH_B: i32 = 1024;
