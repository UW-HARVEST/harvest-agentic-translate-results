//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as *shared libraries* through `libloading`
//! and called only through their exported `float2half` symbol. The Rust
//! implementation is deliberately **never** called directly as a Rust function,
//! so the `#[no_mangle] extern "C"` wrapper and the cdylib's C ABI are part of
//! what is under test.

// This module is compiled separately into each integration-test binary, and no
// single binary uses every helper, so per-binary dead-code warnings are noise.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

pub type Float2Half = unsafe extern "C" fn(f32) -> u16;

/// Deterministic xorshift64* PRNG so every "randomized" row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub const SEED: u64 = 0x2545_F491_4F6C_DD1D;

    pub fn new() -> Self {
        Rng(Self::SEED)
    }

    pub fn with_seed(seed: u64) -> Self {
        // Guard against the zero state, which is a fixed point of xorshift.
        Rng(if seed == 0 { Self::SEED } else { seed })
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

    /// Uniform in `0..bound` (bound > 0).
    pub fn below(&mut self, bound: u32) -> u32 {
        self.next_u32() % bound
    }
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_so(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut found = None;
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some("so") && p.is_file() {
            // Prefer a deterministic choice if several exist.
            match &found {
                None => found = Some(p),
                Some(prev) if p < *prev => found = Some(p),
                _ => {}
            }
        }
    }
    found
}

/// Build (if needed) and locate the C shared library.
fn c_so_path() -> PathBuf {
    let root = repo_root();
    let build = root.join("c_src/build");

    if let Some(p) = find_so(&build) {
        return p;
    }

    std::fs::create_dir_all(&build).expect("create c_src/build");

    let cmake = Command::new("cmake")
        .current_dir(&build)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .output()
        .expect("failed to run `cmake` — is it installed?");
    assert!(
        cmake.status.success(),
        "cmake configure failed:\n{}\n{}",
        String::from_utf8_lossy(&cmake.stdout),
        String::from_utf8_lossy(&cmake.stderr)
    );

    let build_out = Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .output()
        .expect("failed to run `cmake --build`");
    assert!(
        build_out.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&build_out.stdout),
        String::from_utf8_lossy(&build_out.stderr)
    );

    find_so(&build).unwrap_or_else(|| panic!("no .so produced in {}", build.display()))
}

/// Build (if needed) and locate the Rust cdylib.
fn rust_so_path() -> PathBuf {
    let root = repo_root();
    let translation = root.join("translation");

    for profile in ["release", "debug"] {
        let candidate = translation
            .join("target")
            .join(profile)
            .join("libfloat2half_lib.so");
        if candidate.is_file() {
            return candidate;
        }
    }

    // Not built yet: build the cdylib. `cargo` is re-entrant enough here
    // because the cdylib target is independent of the test target.
    let out = Command::new(env!("CARGO"))
        .current_dir(&translation)
        .args(["build", "--release", "--lib"])
        .output()
        .expect("failed to run cargo build");
    assert!(
        out.status.success(),
        "cargo build --release --lib failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let p = translation.join("target/release/libfloat2half_lib.so");
    assert!(p.is_file(), "expected cdylib at {}", p.display());
    p
}

/// A loaded pair of libraries plus the resolved `float2half` symbols.
pub struct Pair {
    // Keep the libraries alive for as long as the function pointers are used.
    _c_lib: libloading::Library,
    _rust_lib: libloading::Library,
    pub c: Float2Half,
    pub rust: Float2Half,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

impl Pair {
    fn load() -> Self {
        let c_path = c_so_path();
        let rust_path = rust_so_path();

        unsafe {
            let c_lib = libloading::Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rust_lib = libloading::Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));

            let c_sym: libloading::Symbol<Float2Half> = c_lib
                .get(b"float2half\0")
                .expect("C .so does not export `float2half`");
            let rust_sym: libloading::Symbol<Float2Half> = rust_lib
                .get(b"float2half\0")
                .expect("Rust .so does not export `float2half`");

            let c = *c_sym;
            let rust = *rust_sym;

            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
                c_path,
                rust_path,
            }
        }
    }

    /// Call both libraries with the given raw bit pattern and assert the
    /// returned `uint16_t`s are identical.
    #[track_caller]
    pub fn check_bits(&self, bits: u32) {
        let x = f32::from_bits(bits);
        let got_c = unsafe { (self.c)(x) };
        let got_rust = unsafe { (self.rust)(x) };
        assert_eq!(
            got_c, got_rust,
            "float2half divergence for input bits 0x{bits:08x} \
             (j=0x{j:03x}={j}, mantissa=0x{m:06x}): C=0x{got_c:04x} Rust=0x{got_rust:04x}",
            j = (bits >> 23) & 0x1ff,
            m = bits & 0x007f_ffff,
        );
    }

    /// Same as [`check_bits`] but taking an `f32` directly.
    #[track_caller]
    pub fn check_f32(&self, x: f32) {
        self.check_bits(x.to_bits());
    }

    /// Call both and return the agreed-upon result, asserting agreement.
    #[track_caller]
    pub fn agreed(&self, bits: u32) -> u16 {
        self.check_bits(bits);
        unsafe { (self.c)(f32::from_bits(bits)) }
    }
}

/// The process-wide loaded pair. Loading once keeps the exhaustive tests fast.
pub fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(Pair::load)
}

/// Build `(j << 23) | mantissa` — a full float bit pattern for bucket `j`.
///
/// Asserts unconditionally (not `debug_assert`): a mantissa with bit 23 set
/// would carry into the exponent field and silently test a different bucket than
/// intended, and release-mode test runs must not be able to hide that.
#[track_caller]
pub fn bits_for(j: u32, mantissa: u32) -> u32 {
    assert!(j < 512, "j must be a 9-bit index, got {j}");
    assert!(
        mantissa <= 0x007f_ffff,
        "mantissa must fit in 23 bits, got 0x{mantissa:08x} (bit 23+ would corrupt j)"
    );
    (j << 23) | mantissa
}

/// The 28 contiguous runs of `m__shift`, as `(start_j, end_j_inclusive, shift)`.
///
/// Transcribed from the C table layout and re-derived at runtime by
/// [`observed_shift`] so a table typo cannot make the tests vacuous.
pub const SHIFT_RUNS: [(u32, u32, u32); 28] = [
    (0, 102, 24),
    (103, 103, 23),
    (104, 104, 22),
    (105, 105, 21),
    (106, 106, 20),
    (107, 107, 19),
    (108, 108, 18),
    (109, 109, 17),
    (110, 110, 16),
    (111, 111, 15),
    (112, 112, 14),
    (113, 142, 13),
    (143, 254, 24),
    (255, 255, 13),
    (256, 358, 24),
    (359, 359, 23),
    (360, 360, 22),
    (361, 361, 21),
    (362, 362, 20),
    (363, 363, 19),
    (364, 364, 18),
    (365, 365, 17),
    (366, 366, 16),
    (367, 367, 15),
    (368, 368, 14),
    (369, 398, 13),
    (399, 510, 24),
    (511, 511, 13),
];

/// Sentinel returned by [`observed_shift`] for every bucket whose shift is 23 or
/// more.
///
/// A mantissa is only 23 bits wide, so `m >> 23` and `m >> 24` are both
/// identically zero for every possible input: shifts of 23 and 24 are
/// *behaviourally indistinguishable* through the public API. `observed_shift`
/// therefore saturates here, and comparisons against the `m__shift` values
/// transcribed from the C source must be made with `.min(OBSERVABLE_SHIFT_CAP)`.
pub const OBSERVABLE_SHIFT_CAP: u32 = 23;

/// Recover `m__shift[j]` from observable behaviour of a loaded library: for the
/// true shift `s`, `f(j<<23 | 1<<s) != f(j<<23)` while every lower bit is
/// discarded.
///
/// Only mantissa bits 0..=22 may be probed — setting bit 23 would carry into the
/// exponent field and silently change `j` — so the largest distinguishable shift
/// is [`OBSERVABLE_SHIFT_CAP`], which is returned for any bucket that discards
/// the whole mantissa.
pub fn observed_shift(f: Float2Half, j: u32) -> u32 {
    let base = unsafe { f(f32::from_bits(bits_for(j, 0))) };
    for s in 0..OBSERVABLE_SHIFT_CAP {
        let probe = unsafe { f(f32::from_bits(bits_for(j, 1 << s))) };
        if probe != base {
            return s;
        }
    }
    OBSERVABLE_SHIFT_CAP
}

/// Recover `m__base[j]` from observable behaviour: it is `f(j << 23)`.
pub fn observed_base(f: Float2Half, j: u32) -> u16 {
    unsafe { f(f32::from_bits(bits_for(j, 0))) }
}
