//! Shared differential-test harness.
//!
//! Both shared objects are loaded with `libloading` and driven **only** through
//! their exported symbols, so the `#[no_mangle]` wrappers are part of what is
//! under test. Both are opened `RTLD_LOCAL` so that the C `.so`'s internal
//! `spectral_contrast@plt` call cannot accidentally bind to the Rust `.so`'s
//! definition (or vice versa).

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::{Path, PathBuf};

use libloading::os::unix::{Library as UnixLibrary, RTLD_LOCAL, RTLD_NOW};
use libloading::Library;

pub type MatchFn = unsafe extern "C" fn(*mut f64, *mut f64, c_int, f64) -> c_int;
pub type SpectralFn = unsafe extern "C" fn(*mut f32, *mut f32, c_int) -> f64;

pub struct Lib {
    pub name: &'static str,
    _lib: Library,
    pub r#match: MatchFn,
    pub spectral_contrast: SpectralFn,
}

impl Lib {
    fn open(name: &'static str, path: &Path) -> Lib {
        let lib: Library = unsafe {
            UnixLibrary::open(Some(path), RTLD_NOW | RTLD_LOCAL)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()))
        }
        .into();
        unsafe {
            let m = *lib
                .get::<MatchFn>(b"match\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol `match`: {e}"));
            let s = *lib
                .get::<SpectralFn>(b"spectral_contrast\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol `spectral_contrast`: {e}"));
            Lib {
                name,
                _lib: lib,
                r#match: m,
                spectral_contrast: s,
            }
        }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

/// `c_src/build/lib<project>.so`. The CMake project name is derived from the
/// name of the directory containing `c_src`, so it is discovered by glob rather
/// than hard-coded.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}); build the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|e| e == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {}, found {found:?}",
        build.display()
    );
    found.pop().unwrap()
}

/// `translation/target/{release,debug}/libunderhanded_c_nuke_lib.so`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let name = "libunderhanded_c_nuke_lib.so";
    for profile in ["release", "debug"] {
        let p = target.join(profile).join(name);
        if p.exists() {
            return p;
        }
    }
    panic!(
        "{name} not found under {}; run `cargo build --release` first",
        target.display()
    );
}

pub fn c_lib() -> Lib {
    Lib::open("C", &c_so_path())
}

pub fn rust_lib() -> Lib {
    Lib::open("Rust", &rust_so_path())
}

/// Both libraries, ready for differential calls.
pub struct Pair {
    pub c: Lib,
    pub rust: Lib,
}

pub fn pair() -> Pair {
    Pair {
        c: c_lib(),
        rust: rust_lib(),
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xoshiro256** seeded through SplitMix64). Self-contained so
// the test suite has no dependency beyond `libloading`.
// ---------------------------------------------------------------------------

pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        let mut z = seed;
        let mut next = || {
            z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut x = z;
            x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            x ^ (x >> 31)
        };
        Rng {
            s: [next(), next(), next(), next()],
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let r = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        r
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform in `[-1, 1)`.
    pub fn signed(&mut self) -> f64 {
        self.unit() * 2.0 - 1.0
    }

    /// Uniform in `[lo, hi)`.
    pub fn range_f64(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.unit() * (hi - lo)
    }

    /// Uniform in `[lo, hi]`.
    pub fn range_usize(&mut self, lo: usize, hi: usize) -> usize {
        lo + (self.next_u64() % (hi - lo + 1) as u64) as usize
    }

    /// A `f64` with a random sign, a random exponent within `±exp_bits` and a
    /// random mantissa. Covers subnormals and overflow-to-`inf` when scaled.
    pub fn wide(&mut self, max_abs_exp: i32) -> f64 {
        let e = (self.next_u64() % (2 * max_abs_exp as u64 + 1)) as i32 - max_abs_exp;
        let m = self.signed();
        m * (2f64).powi(e)
    }

    /// Fully random 64 bits reinterpreted as `f64` (includes NaN/inf/subnormal).
    pub fn raw_f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }

    /// Fully random 32 bits reinterpreted as `f32`.
    pub fn raw_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers.
// ---------------------------------------------------------------------------

pub fn bits64(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

pub fn bits32(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

#[track_caller]
pub fn assert_f64_bits_eq(ctx: &str, c: f64, rust: f64) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{ctx}: return value differs\n  C    = {c:?} (0x{:016x})\n  Rust = {rust:?} (0x{:016x})",
        c.to_bits(),
        rust.to_bits()
    );
}

#[track_caller]
pub fn assert_slice32_bits_eq(ctx: &str, what: &str, c: &[f32], rust: &[f32]) {
    assert_eq!(c.len(), rust.len(), "{ctx}: {what} length differs");
    for i in 0..c.len() {
        assert_eq!(
            c[i].to_bits(),
            rust[i].to_bits(),
            "{ctx}: {what}[{i}] differs\n  C    = {:?} (0x{:08x})\n  Rust = {:?} (0x{:08x})",
            c[i],
            c[i].to_bits(),
            rust[i],
            rust[i].to_bits()
        );
    }
}

#[track_caller]
pub fn assert_slice64_bits_eq(ctx: &str, what: &str, c: &[f64], rust: &[f64]) {
    assert_eq!(c.len(), rust.len(), "{ctx}: {what} length differs");
    for i in 0..c.len() {
        assert_eq!(
            c[i].to_bits(),
            rust[i].to_bits(),
            "{ctx}: {what}[{i}] differs\n  C    = {:?} (0x{:016x})\n  Rust = {:?} (0x{:016x})",
            c[i],
            c[i].to_bits(),
            rust[i],
            rust[i].to_bits()
        );
    }
}

// ---------------------------------------------------------------------------
// Differential drivers.
// ---------------------------------------------------------------------------

/// Call `spectral_contrast` in both `.so`s on identical copies of `a`/`b` and
/// assert the return value *and* both in-place-normalised buffers match.
#[track_caller]
pub fn diff_spectral(p: &Pair, ctx: &str, a: &[f32], b: &[f32], length: c_int) {
    let (mut ca, mut cb) = (a.to_vec(), b.to_vec());
    let (mut ra, mut rb) = (a.to_vec(), b.to_vec());
    let cr = unsafe { (p.c.spectral_contrast)(ca.as_mut_ptr(), cb.as_mut_ptr(), length) };
    let rr = unsafe { (p.rust.spectral_contrast)(ra.as_mut_ptr(), rb.as_mut_ptr(), length) };
    assert_f64_bits_eq(ctx, cr, rr);
    assert_slice32_bits_eq(ctx, "a (normalised in place)", &ca, &ra);
    assert_slice32_bits_eq(ctx, "b (normalised in place)", &cb, &rb);
}

/// Same, but with `a` and `b` aliased to one buffer, as the C permits.
#[track_caller]
pub fn diff_spectral_aliased(p: &Pair, ctx: &str, a: &[f32], length: c_int) {
    let mut cv = a.to_vec();
    let mut rv = a.to_vec();
    let cp = cv.as_mut_ptr();
    let rp = rv.as_mut_ptr();
    let cr = unsafe { (p.c.spectral_contrast)(cp, cp, length) };
    let rr = unsafe { (p.rust.spectral_contrast)(rp, rp, length) };
    assert_f64_bits_eq(ctx, cr, rr);
    assert_slice32_bits_eq(ctx, "v (aliased, normalised)", &cv, &rv);
}

/// Call `match` in both `.so`s and assert the `int` result matches. Also assert
/// neither implementation mutates its input buffers (the C reads them only).
#[track_caller]
pub fn diff_match(p: &Pair, ctx: &str, test: &[f64], reference: &[f64], bins: c_int, threshold: f64) {
    let (mut ct, mut cr) = (test.to_vec(), reference.to_vec());
    let (mut rt, mut rr) = (test.to_vec(), reference.to_vec());
    let cv = unsafe { (p.c.r#match)(ct.as_mut_ptr(), cr.as_mut_ptr(), bins, threshold) };
    let rv = unsafe { (p.rust.r#match)(rt.as_mut_ptr(), rr.as_mut_ptr(), bins, threshold) };
    assert_eq!(
        cv, rv,
        "{ctx}: match() differs: C = {cv}, Rust = {rv} (bins={bins}, threshold={threshold:?} \
         0x{:016x})",
        threshold.to_bits()
    );
    assert_slice64_bits_eq(ctx, "test buffer", &ct, &rt);
    assert_slice64_bits_eq(ctx, "reference buffer", &cr, &rr);
    // Neither side may write through the caller's pointers.
    assert_slice64_bits_eq(ctx, "test buffer (unmodified)", test, &ct);
    assert_slice64_bits_eq(ctx, "reference buffer (unmodified)", reference, &cr);
}

/// `match` with `test` and `reference` aliased to the same buffer.
#[track_caller]
pub fn diff_match_aliased(p: &Pair, ctx: &str, v: &[f64], bins: c_int, threshold: f64) {
    let mut cv = v.to_vec();
    let mut rv = v.to_vec();
    let cp = cv.as_mut_ptr();
    let rp = rv.as_mut_ptr();
    let c = unsafe { (p.c.r#match)(cp, cp, bins, threshold) };
    let r = unsafe { (p.rust.r#match)(rp, rp, bins, threshold) };
    assert_eq!(c, r, "{ctx}: match() differs (aliased): C = {c}, Rust = {r}");
    assert_slice64_bits_eq(ctx, "v", &cv, &rv);
}

/// Number of randomized iterations per `CONFIGS.md` row.
pub fn iters() -> usize {
    std::env::var("DIFF_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(200)
}

// ---------------------------------------------------------------------------
// Input generators (one per value-shape axis in CONFIGS.md).
// ---------------------------------------------------------------------------

pub fn gen_unit_f32(rng: &mut Rng, n: usize) -> Vec<f32> {
    (0..n).map(|_| rng.unit() as f32).collect()
}

pub fn gen_signed_f32(rng: &mut Rng, n: usize) -> Vec<f32> {
    (0..n).map(|_| rng.signed() as f32).collect()
}

pub fn gen_wide_f32(rng: &mut Rng, n: usize) -> Vec<f32> {
    (0..n).map(|_| rng.wide(40) as f32).collect()
}

pub fn gen_raw_f32(rng: &mut Rng, n: usize) -> Vec<f32> {
    (0..n).map(|_| rng.raw_f32()).collect()
}

pub fn gen_subnormal_f32(rng: &mut Rng, n: usize) -> Vec<f32> {
    (0..n)
        .map(|_| f32::from_bits((rng.next_u32() & 0x807F_FFFF) | 1))
        .collect()
}

/// Uniform values with `NaN` / `±inf` sprinkled in at ~1/4 density.
pub fn gen_specials_f32(rng: &mut Rng, n: usize) -> Vec<f32> {
    (0..n)
        .map(|_| match rng.next_u64() % 8 {
            0 => f32::NAN,
            1 => -f32::NAN,
            2 => f32::from_bits(0x7FC0_1234), // NaN, distinct payload
            3 => f32::INFINITY,
            4 => f32::NEG_INFINITY,
            _ => rng.signed() as f32,
        })
        .collect()
}

pub fn gen_unit_f64(rng: &mut Rng, n: usize) -> Vec<f64> {
    (0..n).map(|_| rng.unit()).collect()
}

pub fn gen_signed_f64(rng: &mut Rng, n: usize) -> Vec<f64> {
    (0..n).map(|_| rng.signed()).collect()
}

pub fn gen_wide_f64(rng: &mut Rng, n: usize) -> Vec<f64> {
    (0..n).map(|_| rng.wide(300)).collect()
}

pub fn gen_raw_f64(rng: &mut Rng, n: usize) -> Vec<f64> {
    (0..n).map(|_| rng.raw_f64()).collect()
}

pub fn gen_subnormal_f64(rng: &mut Rng, n: usize) -> Vec<f64> {
    (0..n)
        .map(|_| f64::from_bits((rng.next_u64() & 0x800F_FFFF_FFFF_FFFF) | 1))
        .collect()
}

pub fn gen_specials_f64(rng: &mut Rng, n: usize) -> Vec<f64> {
    (0..n)
        .map(|_| match rng.next_u64() % 8 {
            0 => f64::NAN,
            1 => -f64::NAN,
            2 => f64::from_bits(0x7FF8_0000_1234_5678),
            3 => f64::INFINITY,
            4 => f64::NEG_INFINITY,
            _ => rng.signed(),
        })
        .collect()
}

pub fn gen_ramp_f64(rng: &mut Rng, n: usize) -> Vec<f64> {
    let base = rng.signed();
    let step = rng.signed();
    (0..n).map(|i| base + step * i as f64).collect()
}

pub fn gen_dc_f64(rng: &mut Rng, n: usize) -> Vec<f64> {
    let c = rng.signed();
    vec![c; n]
}

/// The `threshold` values the `comisd` gates treat specially.
pub const SPECIAL_THRESHOLDS: &[f64] = &[
    f64::NEG_INFINITY,
    -1e300,
    -1.0,
    -0.0,
    0.0,
    1e-300,
    0.5,
    1.0 - f64::EPSILON,
    1.0,
    1.0 + f64::EPSILON,
    1e300,
    f64::INFINITY,
    f64::NAN,
    -f64::NAN,
];

// --------------------------------------------------------------------------
// Seam helpers: reproduce `match.c`'s `preprocess` and the `double` -> `float`
// reinterpretation so tests can *construct* inputs for the low-level
// `spectral_contrast` export. Used to build inputs only, never to check outputs.
// --------------------------------------------------------------------------

/// A local re-implementation of `c_src/src/match.c`'s `preprocess`, used only to
/// *construct* seam inputs for row 35 (not to check outputs).
pub fn preprocess_ref(source: &[f64]) -> Vec<f64> {
    const N_SMOOTH: usize = 16;
    let mut v = source.to_vec();
    let len = v.len();
    let smoothen = |v: &mut Vec<f64>| {
        for i in 0..len {
            let mut sum = 0.0f64;
            let mut j = 0;
            while j < N_SMOOTH && i + j < len {
                sum += v[i + j];
                j += 1;
            }
            v[i] = sum / N_SMOOTH as f64;
        }
    };
    smoothen(&mut v);
    if len > 0 {
        for i in 0..len - 1 {
            v[i] = v[i + 1] - v[i];
        }
        v[len - 1] = 0.0;
    }
    smoothen(&mut v);
    v
}

/// The first `n` `f32` lanes of an `f64` buffer, i.e. what `spectral_contrast`
/// sees when `match` hands it a `double *`.
pub fn as_f32_lanes(v: &mut [f64], n: usize) -> Vec<f32> {
    assert!(n <= v.len(), "n f32 lanes must fit in n f64 slots");
    let lanes = unsafe { std::slice::from_raw_parts(v.as_ptr() as *const f32, n) };
    lanes.to_vec()
}
