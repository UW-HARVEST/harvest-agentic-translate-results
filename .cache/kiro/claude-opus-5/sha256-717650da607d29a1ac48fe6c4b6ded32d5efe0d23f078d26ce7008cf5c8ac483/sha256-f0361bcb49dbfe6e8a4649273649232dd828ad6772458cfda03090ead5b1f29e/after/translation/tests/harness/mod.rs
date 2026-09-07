//! Shared harness: loads BOTH the C `.so` and the Rust `.so` with `libloading`
//! and exposes each library's `float2half` export.
//!
//! The Rust function is NEVER called directly — it is always reached through
//! the `cdylib`'s `#[no_mangle] extern "C"` symbol, exactly as an external C
//! caller would, so the export wrapper and its ABI are under test too.

use libloading::{Library, Symbol};
use std::path::PathBuf;

pub type Float2Half = unsafe extern "C" fn(f32) -> u16;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    let build = repo_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one C .so in {}, found {found:?}",
        build.display()
    );
    found.pop().unwrap()
}

fn rust_so_path() -> PathBuf {
    // Explicit override lets the Phase D driver pin exactly which cdylib build
    // is under test (debug and release differ: the dev profile enables
    // `debug_assertions` + `overflow-checks`, so it can panic where the C
    // silently wraps — that is a distinct code path worth exercising).
    if let Some(p) = std::env::var_os("FLOAT2HALF_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "FLOAT2HALF_RUST_SO={} is not a file", p.display());
        return p;
    }

    // Otherwise: require the cdylib built with the SAME profile as this test
    // binary, so a missing build cannot silently fall back to the other
    // profile and hide it from verification.
    let target = repo_root().join("translation").join("target");
    let name = "libfloat2half_lib.so";
    let want = if cfg!(debug_assertions) { "debug" } else { "release" };
    let cand = target.join(want).join(name);
    assert!(
        cand.is_file(),
        "{} not found. Run `cargo build{}` so the {want}-profile cdylib exists \
         (refusing to fall back to the other profile).",
        cand.display(),
        if want == "release" { " --release" } else { "" }
    );
    cand
}

/// Both libraries, kept alive for the duration of a test.
pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: Float2Half,
    pub rust: Float2Half,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

impl Pair {
    pub fn load() -> Pair {
        let c_path = c_so_path();
        let rust_path = rust_so_path();

        // SAFETY: loading a plain C/Rust cdylib with no initialisers that run
        // arbitrary code; both are built from this repository.
        let c_lib = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
        let rust_lib = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));

        let c = unsafe {
            let s: Symbol<Float2Half> = c_lib
                .get(b"float2half\0")
                .expect("C .so does not export `float2half`");
            *s
        };
        let rust = unsafe {
            let s: Symbol<Float2Half> = rust_lib
                .get(b"float2half\0")
                .expect("Rust .so does not export `float2half`");
            *s
        };

        Pair {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c,
            rust,
            c_path,
            rust_path,
        }
    }

    /// Call both exports with the same raw bit pattern and return `(c, rust)`.
    #[inline]
    pub fn both_bits(&self, bits: u32) -> (u16, u16) {
        let x = f32::from_bits(bits);
        // SAFETY: both symbols have the checked `extern "C" fn(f32) -> u16` ABI.
        unsafe { ((self.c)(x), (self.rust)(x)) }
    }

    /// Assert byte-identical results for one raw bit pattern.
    #[inline]
    #[track_caller]
    pub fn assert_bits(&self, bits: u32, what: &str) {
        let (c, r) = self.both_bits(bits);
        assert_eq!(
            c, r,
            "DIVERGENCE ({what}): input bits 0x{bits:08X} (f32 {:e}) \
             -> C 0x{c:04X} != Rust 0x{r:04X}",
            f32::from_bits(bits)
        );
    }

    /// Assert byte-identical results for an `f32` value.
    #[inline]
    #[track_caller]
    pub fn assert_val(&self, x: f32, what: &str) {
        self.assert_bits(x.to_bits(), what);
    }
}

/// Compose a raw `f32` bit pattern from its three fields, exactly as the C
/// code decomposes it (`j = (n >> 23) & 0x1ff` is `(sign << 8) | exponent`).
#[inline]
pub fn bits_from(j: u32, mantissa: u32) -> u32 {
    debug_assert!(j < 512);
    debug_assert!(mantissa <= 0x007f_ffff);
    (j << 23) | (mantissa & 0x007f_ffff)
}

/// Deterministic SplitMix64 — fixed seed, reproducible across runs/platforms.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `0..n` (n > 0).
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

/// The mantissa boundary shapes every row is probed with.
pub const MANTISSA_EDGES: [u32; 9] = [
    0x00_0000, 0x00_0001, 0x00_0002, 0x00_0003, 0x3F_FFFF, 0x40_0000, 0x7F_FFFD, 0x7F_FFFE,
    0x7F_FFFF,
];

/// The maximal contiguous runs of equal `m__shift[j]`, extracted mechanically
/// from `c_src/src/lib.c`. `(j_lo, j_hi, shift, label)`.
pub const RUNS: [(u32, u32, u8, &str); 28] = [
    (0, 102, 0x18, "+ flush-to-zero (zero, subnormals, tiny)"),
    (103, 103, 0x17, "+ half-subnormal step 1"),
    (104, 104, 0x16, "+ half-subnormal step 2"),
    (105, 105, 0x15, "+ half-subnormal step 3"),
    (106, 106, 0x14, "+ half-subnormal step 4"),
    (107, 107, 0x13, "+ half-subnormal step 5"),
    (108, 108, 0x12, "+ half-subnormal step 6"),
    (109, 109, 0x11, "+ half-subnormal step 7"),
    (110, 110, 0x10, "+ half-subnormal step 8"),
    (111, 111, 0x0f, "+ half-subnormal step 9"),
    (112, 112, 0x0e, "+ half-subnormal step 10"),
    (113, 142, 0x0d, "+ normal halves"),
    (143, 254, 0x18, "+ overflow saturate to +Inf"),
    (255, 255, 0x0d, "+Inf / +NaN"),
    (256, 358, 0x18, "- flush-to-zero (-0, subnormals, tiny)"),
    (359, 359, 0x17, "- half-subnormal step 1"),
    (360, 360, 0x16, "- half-subnormal step 2"),
    (361, 361, 0x15, "- half-subnormal step 3"),
    (362, 362, 0x14, "- half-subnormal step 4"),
    (363, 363, 0x13, "- half-subnormal step 5"),
    (364, 364, 0x12, "- half-subnormal step 6"),
    (365, 365, 0x11, "- half-subnormal step 7"),
    (366, 366, 0x10, "- half-subnormal step 8"),
    (367, 367, 0x0f, "- half-subnormal step 9"),
    (368, 368, 0x0e, "- half-subnormal step 10"),
    (369, 398, 0x0d, "- normal halves"),
    (399, 510, 0x18, "- overflow saturate to -Inf"),
    (511, 511, 0x0d, "-Inf / -NaN"),
];
pub const REAL_RUNS: usize = 28;
