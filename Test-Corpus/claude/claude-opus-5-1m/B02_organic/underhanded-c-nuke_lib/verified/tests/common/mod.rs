//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as *shared objects* through `libloading` and
//! called only through their exported C symbols -- the Rust functions are never
//! called directly, so the `#[no_mangle] extern "C"` wrappers are under test
//! too.
//!
//! `libloading::Library::new` uses `RTLD_LOCAL`, so the two libraries cannot
//! interpose each other's symbols: the C `match` always calls the C
//! `spectral_contrast` and the Rust `match` always calls the Rust one.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type MatchFn = unsafe extern "C" fn(*mut f64, *mut f64, i32, f64) -> i32;
pub type ScFn = unsafe extern "C" fn(*mut f32, *mut f32, i32) -> f64;

pub struct Impl {
    pub name: &'static str,
    pub r#match: MatchFn,
    pub spectral_contrast: ScFn,
    _lib: Library,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    let s: Symbol<T> = unsafe { lib.get(name) }
        .unwrap_or_else(|e| panic!("symbol {:?} not found: {e}", String::from_utf8_lossy(name)));
    *s
}

fn load(name: &'static str, path: &PathBuf) -> Impl {
    assert!(path.is_file(), "shared object not found: {}", path.display());
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
    let r#match = unsafe { sym::<MatchFn>(&lib, b"match\0") };
    let spectral_contrast = unsafe { sym::<ScFn>(&lib, b"spectral_contrast\0") };
    Impl { name, r#match, spectral_contrast, _lib: lib }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` -- derived from this test binary's own location
/// (`target/<profile>/deps/<test>-<hash>`), so the `.so` that belongs to the
/// profile currently under test is the one that gets loaded.
fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

/// Build the C shared library if it is not there yet, exactly as the task
/// describes.
fn ensure_c_so() -> PathBuf {
    let so = c_so_path();
    let c_src = manifest_dir().join("c_src");
    if so.is_file() {
        assert_fresh(&so, &c_src, "cd c_src/build && cmake --build .");
        return so;
    }
    let build = c_src.join("build");
    std::fs::create_dir_all(&build).expect("mkdir c_src/build");
    let cfg = std::process::Command::new("cmake")
        .current_dir(&build)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .output()
        .expect("run cmake");
    assert!(cfg.status.success(), "cmake configure failed:\n{}", String::from_utf8_lossy(&cfg.stderr));
    let bld = std::process::Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .output()
        .expect("run cmake --build");
    assert!(bld.status.success(), "cmake --build failed:\n{}", String::from_utf8_lossy(&bld.stderr));
    assert!(so.is_file(), "C .so still missing after build: {}", so.display());
    assert_fresh(&so, &c_src, "cd c_src/build && cmake --build .");
    so
}

fn newest_mtime(dir: &std::path::Path) -> std::time::SystemTime {
    let mut newest = std::time::SystemTime::UNIX_EPOCH;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().map_or(false, |x| x == "rs" || x == "c" || x == "h") {
                if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                    newest = newest.max(m);
                }
            }
        }
    }
    newest
}

/// `cargo test --test <name>` does **not** rebuild a `cdylib`-only library
/// target (the test binary does not link against it), so it is entirely
/// possible to run a differential test against a *stale* `.so` and see it pass.
/// Refuse to do that.
fn assert_fresh(so: &std::path::Path, sources: &std::path::Path, hint: &str) {
    let so_m = so.metadata().and_then(|m| m.modified()).expect("so mtime");
    let src_m = newest_mtime(sources);
    assert!(
        so_m >= src_m,
        "STALE SHARED OBJECT: {} is older than the newest source in {}.\n\
         The differential test would be comparing against an out-of-date build.\n\
         Run `{hint}` first.",
        so.display(),
        sources.display()
    );
}

fn rust_so_path() -> PathBuf {
    let dir = target_profile_dir();
    let p = dir.join("libunderhanded_c_nuke_lib.so");
    assert!(
        p.is_file(),
        "Rust cdylib not found at {} -- run `cargo build` for this profile first",
        p.display()
    );
    let profile_flag = if dir.ends_with("release") { " --release" } else { "" };
    assert_fresh(&p, &manifest_dir().join("src"), &format!("cargo build{profile_flag}"));
    p
}

pub struct Both {
    pub c: Impl,
    pub rs: Impl,
}

static BOTH: OnceLock<Both> = OnceLock::new();

pub fn both() -> &'static Both {
    BOTH.get_or_init(|| Both {
        c: load("C", &ensure_c_so()),
        rs: load("Rust", &rust_so_path()),
    })
}

/// Loads *exactly one* of the two libraries -- used by the interposition guard
/// in `phase_d.rs` to prove that having both open at once does not let one
/// library's `spectral_contrast` satisfy the other library's internal call.
pub fn load_single(which: &str) -> Impl {
    match which {
        "c" => load("C", &ensure_c_so()),
        "rs" => load("Rust", &rust_so_path()),
        other => panic!("unknown implementation {other:?}"),
    }
}

/// A deterministic batch of calls whose combined results are folded into a
/// digest. Any behavioural change anywhere in the library moves the digest.
pub fn batch_digest(imp: &Impl) -> u64 {
    let mut acc: u64 = 0xCBF2_9CE4_8422_2325;
    let mut mix = |v: u64| {
        acc ^= v;
        acc = acc.wrapping_mul(0x100_0000_01B3);
    };

    for bins in [1i32, 2, 3, 5, 16, 17, 33, 64] {
        let mut rng = Rng::new(0xD16E_5700 ^ bins as u64);
        for _ in 0..16 {
            let t = gen_f64_bits(F64Shape::Positive, bins as usize, &mut rng);
            let r = gen_f64_bits(F64Shape::Peaked, bins as usize, &mut rng);
            for &th in &[0.0f64, 0.25, 0.5, 0.9, 1.0, 2.0] {
                let mut tv: Vec<f64> = t.iter().map(|&x| f64::from_bits(x)).collect();
                let mut rv: Vec<f64> = r.iter().map(|&x| f64::from_bits(x)).collect();
                let got = unsafe { (imp.r#match)(tv.as_mut_ptr(), rv.as_mut_ptr(), bins, th) };
                mix(got as u32 as u64);
            }
        }
    }
    for len in [1i32, 2, 3, 16, 17, 64] {
        let mut rng = Rng::new(0xD16E_5701 ^ len as u64);
        for _ in 0..16 {
            let a = gen_f32_bits(F32Shape::Normal, len as usize, &mut rng);
            let b = gen_f32_bits(F32Shape::Normal, len as usize, &mut rng);
            let mut av: Vec<f32> = a.iter().map(|&x| f32::from_bits(x)).collect();
            let mut bv: Vec<f32> = b.iter().map(|&x| f32::from_bits(x)).collect();
            let got = unsafe { (imp.spectral_contrast)(av.as_mut_ptr(), bv.as_mut_ptr(), len) };
            mix(got.to_bits());
            for x in av.iter().chain(bv.iter()) {
                mix(x.to_bits() as u64);
            }
        }
    }
    acc
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) -- fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub const ROOT_SEED: u64 = 0x5EED_1234_ABCD_EF01;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ ROOT_SEED)
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
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 0
    }
    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) * (1.0 / ((1u64 << 53) as f64))
    }
    /// A "well behaved" value with a randomized exponent in
    /// `2^-lo .. 2^hi`, random sign.
    pub fn scaled(&mut self, lo: i32, hi: i32) -> f64 {
        let span = (hi - lo + 1) as u64;
        let e = lo + self.below(span) as i32;
        let v = (self.unit() + 0.5) * libm_exp2(e);
        if self.bool() { v } else { -v }
    }
}

fn libm_exp2(e: i32) -> f64 {
    // exact power of two, no libm needed
    if e >= 0 {
        (1u128 << e.min(126)) as f64
    } else {
        1.0 / ((1u128 << (-e).min(126)) as f64)
    }
}

// ---------------------------------------------------------------------------
// Input shape generators
// ---------------------------------------------------------------------------

/// Shapes for `f32` buffers passed to `spectral_contrast`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum F32Shape {
    /// Random sign/exponent/mantissa within a sane range.
    Normal,
    /// Completely random 32 bits: NaNs, infinities, denormals, +-0 all occur.
    RawBits,
    /// All `+0.0`.
    PosZeros,
    /// All `-0.0`.
    NegZeros,
    /// Random denormals only (bits `1 ..= 0x7F_FFFF`).
    Denormal,
    /// `~3.0e38`: `f32` products overflow to `+inf`.
    Huge,
    /// `~1e-38`: `f32` products underflow.
    Tiny,
    /// Random mixture of `+0.0` and `-0.0`.
    SignedZeros,
    /// Finite values with `+-inf` sprinkled in.
    WithInf,
    /// Finite values with quiet *and* signaling NaN patterns sprinkled in.
    WithNan,
    /// Every element is a NaN with a *distinct* payload. Discriminates which
    /// operand's payload survives `mulss`/`addsd`.
    DistinctNans,
    /// Mostly finite, with distinct-payload NaNs sprinkled in.
    FiniteWithDistinctNans,
}

/// A NaN with a payload derived from `k`, alternating sign and quiet/signaling.
pub fn distinct_nan_f32(k: u32) -> u32 {
    let payload = k.wrapping_mul(2_654_435_761) & 0x003F_FFFF; // keep clear of bit 22
    let payload = if payload == 0 { 1 } else { payload };
    let quiet = if k % 3 == 0 { 0 } else { 0x0040_0000 };
    let sign = if k % 2 == 0 { 0 } else { 0x8000_0000 };
    sign | 0x7F80_0000 | quiet | payload
}

/// A `f64` NaN with a payload derived from `k`.
pub fn distinct_nan_f64(k: u64) -> u64 {
    let payload = k.wrapping_mul(0x9E37_79B9_7F4A_7C15) & 0x0007_FFFF_FFFF_FFFF;
    let payload = if payload == 0 { 1 } else { payload };
    let quiet = if k % 3 == 0 { 0 } else { 0x0008_0000_0000_0000 };
    let sign = if k % 2 == 0 { 0 } else { 1u64 << 63 };
    sign | 0x7FF0_0000_0000_0000 | quiet | payload
}

pub fn gen_f32_bits(shape: F32Shape, n: usize, rng: &mut Rng) -> Vec<u32> {
    (0..n)
        .map(|_| match shape {
            F32Shape::Normal => (rng.scaled(-20, 20) as f32).to_bits(),
            F32Shape::RawBits => rng.next_u32(),
            F32Shape::PosZeros => 0.0f32.to_bits(),
            F32Shape::NegZeros => (-0.0f32).to_bits(),
            F32Shape::Denormal => {
                let m = 1 + (rng.below(0x7F_FFFF) as u32);
                let s = if rng.bool() { 0 } else { 0x8000_0000 };
                s | m
            }
            F32Shape::Huge => {
                let v = 3.0e38f32 * (0.5 + rng.unit() as f32 * 0.5);
                if rng.bool() { v.to_bits() } else { (-v).to_bits() }
            }
            F32Shape::Tiny => {
                let v = 1.0e-38f32 * (0.5 + rng.unit() as f32 * 0.5);
                if rng.bool() { v.to_bits() } else { (-v).to_bits() }
            }
            F32Shape::SignedZeros => {
                if rng.bool() { 0.0f32.to_bits() } else { (-0.0f32).to_bits() }
            }
            F32Shape::WithInf => match rng.below(4) {
                0 => f32::INFINITY.to_bits(),
                1 => f32::NEG_INFINITY.to_bits(),
                _ => (rng.scaled(-10, 10) as f32).to_bits(),
            },
            F32Shape::WithNan => match rng.below(4) {
                0 => 0x7FC0_0001, // quiet NaN, non-default payload
                1 => 0xFFC0_0002, // negative quiet NaN
                2 => 0x7F80_0001, // signaling NaN
                _ => (rng.scaled(-10, 10) as f32).to_bits(),
            },
            F32Shape::DistinctNans => distinct_nan_f32(rng.next_u32()),
            F32Shape::FiniteWithDistinctNans => {
                if rng.below(3) == 0 {
                    distinct_nan_f32(rng.next_u32())
                } else {
                    (rng.scaled(-10, 10) as f32).to_bits()
                }
            }
        })
        .collect()
}

/// Shapes for `f64` buffers passed to `match`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum F64Shape {
    /// Random positive values -- the realistic "energy spectrum" case.
    Positive,
    /// Random signed values.
    Signed,
    /// Completely random 64 bits.
    RawBits,
    /// All `+0.0`.
    Zeros,
    /// `v[i] = i + 1`.
    Ramp,
    /// `v[i] = n - i`.
    RampDown,
    /// Narrow peaks on a small floor.
    Peaked,
    /// Random `f64` denormals.
    Denormal,
    /// `~1e308`: sums overflow to `+inf`.
    Huge,
    /// Finite values with `+-inf` and NaN sprinkled in.
    InfNan,
    /// All negative.
    Negative,
    /// Every element is a NaN with a *distinct* payload -- discriminates the
    /// `addsd` operand order in `total`/`smoothen`.
    DistinctNans,
    /// Mostly finite, with distinct-payload NaNs sprinkled in.
    FiniteWithDistinctNans,
}

pub fn gen_f64_bits(shape: F64Shape, n: usize, rng: &mut Rng) -> Vec<u64> {
    (0..n)
        .map(|i| {
            let v: f64 = match shape {
                F64Shape::Positive => rng.scaled(-20, 20).abs(),
                F64Shape::Signed => rng.scaled(-20, 20),
                F64Shape::RawBits => return rng.next_u64(),
                F64Shape::Zeros => 0.0,
                F64Shape::Ramp => (i + 1) as f64,
                F64Shape::RampDown => (n - i) as f64,
                F64Shape::Peaked => {
                    let centre = n / 3;
                    let d = (i as i64 - centre as i64).abs() as f64;
                    let peak = 1000.0 / (1.0 + d * d);
                    peak + 0.001 * rng.unit()
                }
                F64Shape::Denormal => {
                    let m = 1 + rng.below(0xFFFF);
                    let s = if rng.bool() { 0 } else { 1u64 << 63 };
                    return s | m;
                }
                F64Shape::Huge => {
                    let v = 1.0e308 * (0.5 + rng.unit() * 0.5);
                    if rng.bool() { v } else { -v }
                }
                F64Shape::InfNan => match rng.below(5) {
                    0 => f64::INFINITY,
                    1 => f64::NEG_INFINITY,
                    2 => f64::NAN,
                    _ => rng.scaled(-10, 10),
                },
                F64Shape::Negative => -rng.scaled(-20, 20).abs(),
                F64Shape::DistinctNans => return distinct_nan_f64(rng.next_u64()),
                F64Shape::FiniteWithDistinctNans => {
                    if rng.below(3) == 0 {
                        return distinct_nan_f64(rng.next_u64());
                    }
                    rng.scaled(-10, 10)
                }
            };
            v.to_bits()
        })
        .collect()
}

/// The `threshold` values the C code branches on.
pub const THRESHOLDS: &[f64] = &[
    f64::NEG_INFINITY,
    -1.0e308,
    -1.0,
    -0.0,
    0.0,
    5.0e-324, // smallest denormal
    1.0e-300,
    0.25,
    0.5,
    0.9,
    1.0,
    2.0,
    1.0e308,
    f64::INFINITY,
    f64::NAN,
];

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq, Debug)]
struct ScOutcome {
    ret: u64,
    a: Vec<u32>,
    b: Vec<u32>,
}

fn run_sc(imp: &Impl, a_bits: &[u32], b_bits: &[u32], len_arg: i32) -> ScOutcome {
    let mut a: Vec<f32> = a_bits.iter().map(|&x| f32::from_bits(x)).collect();
    let mut b: Vec<f32> = b_bits.iter().map(|&x| f32::from_bits(x)).collect();
    let ret = unsafe { (imp.spectral_contrast)(a.as_mut_ptr(), b.as_mut_ptr(), len_arg) };
    ScOutcome {
        ret: ret.to_bits(),
        a: a.iter().map(|x| x.to_bits()).collect(),
        b: b.iter().map(|x| x.to_bits()).collect(),
    }
}

/// Call `spectral_contrast` in both libraries and require bit-identical
/// return value *and* bit-identical in-place mutation of both buffers.
pub fn diff_sc(a_bits: &[u32], b_bits: &[u32], len_arg: i32, ctx: &str) {
    let both = both();
    let c = run_sc(&both.c, a_bits, b_bits, len_arg);
    let rs = run_sc(&both.rs, a_bits, b_bits, len_arg);
    if c != rs {
        panic!(
            "spectral_contrast divergence [{ctx}] len={len_arg}\n  \
             a_in ={a_bits:08X?}\n  b_in ={b_bits:08X?}\n  \
             C  ret={:016X} ({})\n  RS ret={:016X} ({})\n  \
             C  a_out={:08X?}\n  RS a_out={:08X?}\n  \
             C  b_out={:08X?}\n  RS b_out={:08X?}",
            c.ret,
            f64::from_bits(c.ret),
            rs.ret,
            f64::from_bits(rs.ret),
            c.a,
            rs.a,
            c.b,
            rs.b
        );
    }
}

/// Same, but `a` and `b` are literally the *same pointer* (aliased call).
pub fn diff_sc_aliased(a_bits: &[u32], len_arg: i32, ctx: &str) {
    let both = both();
    let run = |imp: &Impl| -> (u64, Vec<u32>) {
        let mut a: Vec<f32> = a_bits.iter().map(|&x| f32::from_bits(x)).collect();
        let p = a.as_mut_ptr();
        let ret = unsafe { (imp.spectral_contrast)(p, p, len_arg) };
        (ret.to_bits(), a.iter().map(|x| x.to_bits()).collect())
    };
    let c = run(&both.c);
    let rs = run(&both.rs);
    assert_eq!(
        c, rs,
        "aliased spectral_contrast divergence [{ctx}] len={len_arg} a_in={a_bits:08X?}\n \
         C ret={} RS ret={}",
        f64::from_bits(c.0),
        f64::from_bits(rs.0)
    );
}

/// `spectral_contrast` with raw (possibly null) pointers.
pub fn diff_sc_raw_ptrs(a: *mut f32, b: *mut f32, len_arg: i32, ctx: &str) {
    let both = both();
    let c = unsafe { (both.c.spectral_contrast)(a, b, len_arg) }.to_bits();
    let rs = unsafe { (both.rs.spectral_contrast)(a, b, len_arg) }.to_bits();
    assert_eq!(
        c, rs,
        "spectral_contrast raw-pointer divergence [{ctx}] len={len_arg}: \
         C={:016X} ({}) RS={:016X} ({})",
        c,
        f64::from_bits(c),
        rs,
        f64::from_bits(rs)
    );
}

#[derive(PartialEq, Eq, Debug)]
struct MatchOutcome {
    ret: i32,
    test: Vec<u64>,
    reference: Vec<u64>,
}

fn run_match(imp: &Impl, t_bits: &[u64], r_bits: &[u64], bins: i32, threshold: f64) -> MatchOutcome {
    let mut t: Vec<f64> = t_bits.iter().map(|&x| f64::from_bits(x)).collect();
    let mut r: Vec<f64> = r_bits.iter().map(|&x| f64::from_bits(x)).collect();
    let ret = unsafe { (imp.r#match)(t.as_mut_ptr(), r.as_mut_ptr(), bins, threshold) };
    MatchOutcome {
        ret,
        test: t.iter().map(|x| x.to_bits()).collect(),
        reference: r.iter().map(|x| x.to_bits()).collect(),
    }
}

/// Call `match` in both libraries. Requires an identical return value and --
/// since C only ever *reads* `test`/`reference` -- that neither implementation
/// modified the caller's buffers (CONFIGS row 39).
pub fn diff_match(t_bits: &[u64], r_bits: &[u64], bins: i32, threshold: f64, ctx: &str) {
    let both = both();
    let c = run_match(&both.c, t_bits, r_bits, bins, threshold);
    let rs = run_match(&both.rs, t_bits, r_bits, bins, threshold);
    if c != rs {
        panic!(
            "match divergence [{ctx}] bins={bins} threshold={threshold} ({:016X})\n  \
             test ={t_bits:016X?}\n  ref  ={r_bits:016X?}\n  \
             C  ret={} RS ret={}\n  \
             C  test_out={:016X?}\n  RS test_out={:016X?}\n  \
             C  ref_out ={:016X?}\n  RS ref_out ={:016X?}",
            threshold.to_bits(),
            c.ret,
            rs.ret,
            c.test,
            rs.test,
            c.reference,
            rs.reference
        );
    }
    assert_eq!(c.test, t_bits, "[{ctx}] C match modified `test` (bins={bins})");
    assert_eq!(c.reference, r_bits, "[{ctx}] C match modified `reference` (bins={bins})");
}

// ---------------------------------------------------------------------------
// Boundary recovery: turning `match`'s 1-bit answer into a 53-bit oracle.
//
// `match` returns `1` iff
//       total(test) >= threshold * total(reference)      (the gate)
//   AND spectral_contrast(t, r, bins) >= threshold       (the verdict)
//
// Both conditions are monotone *decreasing* in `threshold`, so the verdict as a
// function of `threshold` is a single step, and the step's location is
// `min(total(test)/total(reference), contrast)`.  Bisecting on `threshold`
// therefore recovers that value to the last bit -- which exposes the internal
// `spectral_contrast` result that the `int` return type otherwise hides.
// ---------------------------------------------------------------------------

/// Order-preserving `f64` -> `u64` key (total order on non-NaN doubles).
fn f64_key(x: f64) -> u64 {
    let b = x.to_bits();
    if b >> 63 != 0 { !b } else { b | (1u64 << 63) }
}

fn key_to_f64(k: u64) -> f64 {
    let b = if k >> 63 != 0 { k & !(1u64 << 63) } else { !k };
    f64::from_bits(b)
}

/// Largest `threshold` (as an order key) for which `imp` still answers `1`,
/// or `None` when it answers `0` even for `-inf` (i.e. the contrast is NaN).
pub fn match_boundary(imp: &Impl, t_bits: &[u64], r_bits: &[u64], bins: i32) -> Option<u64> {
    let call = |th: f64| -> i32 {
        let mut t: Vec<f64> = t_bits.iter().map(|&x| f64::from_bits(x)).collect();
        let mut r: Vec<f64> = r_bits.iter().map(|&x| f64::from_bits(x)).collect();
        unsafe { (imp.r#match)(t.as_mut_ptr(), r.as_mut_ptr(), bins, th) }
    };
    let mut lo = f64_key(f64::NEG_INFINITY);
    let mut hi = f64_key(f64::INFINITY);
    if call(key_to_f64(lo)) != 1 {
        return None;
    }
    if call(key_to_f64(hi)) == 1 {
        return Some(hi);
    }
    // invariant: P(lo) == 1, P(hi) == 0
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if call(key_to_f64(mid)) == 1 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(lo)
}

/// Compare the recovered decision boundaries of both libraries.
pub fn diff_match_boundary(t_bits: &[u64], r_bits: &[u64], bins: i32, ctx: &str) {
    let both = both();
    let c = match_boundary(&both.c, t_bits, r_bits, bins);
    let rs = match_boundary(&both.rs, t_bits, r_bits, bins);
    if c != rs {
        panic!(
            "match decision boundary diverges [{ctx}] bins={bins}\n  \
             test={t_bits:016X?}\n  ref ={r_bits:016X?}\n  \
             C  boundary={:?} ({:?})\n  RS boundary={:?} ({:?})",
            c.map(|k| format!("{:016X}", key_to_f64(k).to_bits())),
            c.map(key_to_f64),
            rs.map(|k| format!("{:016X}", key_to_f64(k).to_bits())),
            rs.map(key_to_f64),
        );
    }
}

/// `match` with `test` and `reference` being the same pointer.
pub fn diff_match_aliased(v_bits: &[u64], bins: i32, threshold: f64, ctx: &str) {
    let both = both();
    let run = |imp: &Impl| -> (i32, Vec<u64>) {
        let mut v: Vec<f64> = v_bits.iter().map(|&x| f64::from_bits(x)).collect();
        let p = v.as_mut_ptr();
        let ret = unsafe { (imp.r#match)(p, p, bins, threshold) };
        (ret, v.iter().map(|x| x.to_bits()).collect())
    };
    let c = run(&both.c);
    let rs = run(&both.rs);
    assert_eq!(
        c, rs,
        "aliased match divergence [{ctx}] bins={bins} threshold={threshold} v={v_bits:016X?}"
    );
}
