//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and exposes them behind identical call signatures.
//!
//! The Rust side is deliberately NEVER called as a Rust function. Every call
//! goes through `dlsym` on the built `cdylib`, exactly as an external C caller
//! would, so the `#[no_mangle] extern "C"` export wrapper is under test too.

// Each test binary includes this module separately and uses a different subset
// of it, so per-binary "never used" warnings are expected and not meaningful.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;

/// Signature as declared in `c_src/include/lib.h`.
pub type Half2FloatFn = unsafe extern "C" fn(u16) -> f32;

/// Deliberately mis-declared: a caller that claims the callee takes a 32-bit
/// argument. Used to probe the out-of-range / dirty-high-bits FFI boundary
/// (row B5 of `ERRORS.md`).
pub type Half2FloatFnU32 = unsafe extern "C" fn(u32) -> f32;

/// Deliberately mis-declared with a 64-bit argument (row B5).
pub type Half2FloatFnU64 = unsafe extern "C" fn(u64) -> f32;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Locate the C shared library built by `c_src/CMakeLists.txt`.
///
/// The CMake project name is derived from the *parent directory's* name, so the
/// file name is not fixed. Glob for any `lib*.so` in `c_src/build`.
fn c_library_path() -> PathBuf {
    let build_dir = workspace_root().join("c_src").join("build");
    assert!(
        build_dir.is_dir(),
        "C build directory {} does not exist. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build_dir.display()
    );

    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .expect("c_src/build must be readable")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let name = match p.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => return false,
            };
            name.starts_with("lib") && name.ends_with(".so")
        })
        .collect();
    candidates.sort();

    match candidates.len() {
        0 => panic!(
            "no lib*.so found in {} — build the C library first",
            build_dir.display()
        ),
        _ => candidates.remove(0),
    }
}

/// Locate the Rust `cdylib`. Prefers `release` (which is what `SYMBOLS.md`
/// documents) and falls back to `debug`, so the tests work whichever profile
/// was built most recently.
fn rust_library_path() -> PathBuf {
    // Explicit override so CI / the verification script can pin a specific
    // profile instead of relying on mtime.
    if let Some(p) = std::env::var_os("HALF2FLOAT_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(
            p.is_file(),
            "HALF2FLOAT_RUST_SO points at {} which is not a file",
            p.display()
        );
        return p;
    }

    let target = workspace_root().join("translation").join("target");
    let name = "libhalf2float_lib.so";

    let release = target.join("release").join(name);
    let debug = target.join("debug").join(name);

    // Prefer whichever exists; if both exist prefer the newer one so a fresh
    // `cargo build` is always the thing under test.
    match (release.is_file(), debug.is_file()) {
        (true, true) => {
            let m = |p: &PathBuf| {
                std::fs::metadata(p)
                    .and_then(|md| md.modified())
                    .ok()
            };
            if m(&release) >= m(&debug) { release } else { debug }
        }
        (true, false) => release,
        (false, true) => debug,
        (false, false) => panic!(
            "Rust cdylib {} not found in {}/{{release,debug}} — \
             build it with `cargo build --release`",
            name,
            target.display()
        ),
    }
}

/// Both libraries, kept loaded for the lifetime of a test.
pub struct Pair {
    // Field order matters for drop order: symbols borrow from the libraries,
    // but we re-`get` per accessor instead of storing symbols, so plain
    // ownership is fine.
    c_lib: Library,
    rust_lib: Library,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

impl Pair {
    pub fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();

        // SAFETY: both paths point at shared objects we just built from the
        // sources in this repository; loading them runs their initialisers.
        let c_lib = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("failed to dlopen C lib {}: {e}", c_path.display()));
        let rust_lib = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("failed to dlopen Rust lib {}: {e}", rust_path.display()));

        Self { c_lib, rust_lib, c_path, rust_path }
    }

    /// `half2float` from the C `.so`, correctly declared.
    pub fn c_half2float(&self) -> Symbol<'_, Half2FloatFn> {
        unsafe { self.c_lib.get(b"half2float\0") }
            .expect("C .so must export `half2float`")
    }

    /// `half2float` from the Rust `.so`, correctly declared.
    pub fn rust_half2float(&self) -> Symbol<'_, Half2FloatFn> {
        unsafe { self.rust_lib.get(b"half2float\0") }
            .expect("Rust .so must export `half2float`")
    }

    pub fn c_half2float_u32(&self) -> Symbol<'_, Half2FloatFnU32> {
        unsafe { self.c_lib.get(b"half2float\0") }.expect("C .so must export `half2float`")
    }

    pub fn rust_half2float_u32(&self) -> Symbol<'_, Half2FloatFnU32> {
        unsafe { self.rust_lib.get(b"half2float\0") }.expect("Rust .so must export `half2float`")
    }

    pub fn c_half2float_u64(&self) -> Symbol<'_, Half2FloatFnU64> {
        unsafe { self.c_lib.get(b"half2float\0") }.expect("C .so must export `half2float`")
    }

    pub fn rust_half2float_u64(&self) -> Symbol<'_, Half2FloatFnU64> {
        unsafe { self.rust_lib.get(b"half2float\0") }.expect("Rust .so must export `half2float`")
    }
}

/// Compare two `f32`s by their raw bits.
///
/// Using `==` would be wrong in two ways that matter for this library:
///   * `+0.0 == -0.0` is true, but the C returns genuinely distinct bit
///     patterns for `h = 0x0000` and `h = 0x8000`;
///   * `NaN == NaN` is false, and rows 9–12 / 21–23 of `CONFIGS.md` produce
///     NaNs whose payload bits must match exactly.
#[track_caller]
pub fn assert_bits_eq(h: u16, c_val: f32, rust_val: f32, ctx: &str) {
    let cb = c_val.to_bits();
    let rb = rust_val.to_bits();
    assert_eq!(
        cb, rb,
        "divergence for h = {h:#06x} (n = {}, mantissa = {:#05x}) [{ctx}]:\n  \
         C    = {:#010x} ({c_val:e})\n  Rust = {:#010x} ({rust_val:e})",
        h >> 10,
        h & 0x3ff,
        cb,
        rb,
    );
}

/// Deterministic PRNG (SplitMix64) so every randomized row is reproducible
/// from a fixed seed without pulling in a `rand` dependency.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
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

    pub fn next_u16(&mut self) -> u16 {
        (self.next_u64() >> 48) as u16
    }

    /// Uniform-ish value in `0..bound` (bound > 0).
    pub fn below(&mut self, bound: u32) -> u32 {
        self.next_u32() % bound
    }

    /// Inclusive range `lo..=hi`.
    pub fn in_range(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo <= hi);
        lo + self.below(hi - lo + 1)
    }
}

/// Build a half bit pattern from an exponent-field index `n` (0..=63, i.e. the
/// sign bit and the 5 exponent bits) and a 10-bit mantissa field.
pub fn half_from(n: u32, mantissa: u32) -> u16 {
    assert!(n < 64, "n must fit in 6 bits");
    assert!(mantissa < 0x400, "mantissa must fit in 10 bits");
    ((n << 10) | mantissa) as u16
}

/// Number of randomized samples used per `CONFIGS.md` row.
pub const SAMPLES_PER_ROW: usize = 4096;
