//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and calls every function through its exported symbol only.
#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// C-layout types (must mirror c_src/src/lib.c exactly)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

/// Mirrors `c2Simplex { c2sv a, b, c, d; float div; int count; }`.
#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

pub const C2_TYPE_CIRCLE: u32 = 0;
pub const C2_TYPE_AABB: u32 = 1;
pub const C2_TYPE_CAPSULE: u32 = 2;

pub const FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7;

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub fn bits(v: f32) -> u32 {
    v.to_bits()
}

/// Bit-exact float equality, with one deliberate relaxation: two NaNs compare
/// equal regardless of sign bit / payload.
///
/// Rationale (measured, not assumed): the only divergences observed between the
/// two `.so`s are the *sign bit of a NaN that was already NaN on input*, e.g.
/// `c2Dot((FLT_MAX,-inf), (-NaN,+NaN))` yields `0x7fc00000` from gcc and
/// `0xffc00000` from rustc/LLVM. Which operand's NaN payload survives `mulss`
/// is a property of the x86 instruction's operand order, and the C source
/// `a.x*b.x + a.y*b.y` does not fix that order — gcc and LLVM legally pick
/// differently. Everything else stays strict: `-0.0 != 0.0`, `inf` must match
/// exactly, and NaN-vs-non-NaN is still a failure.
pub fn feq(a: f32, b: f32) -> bool {
    if a.is_nan() || b.is_nan() {
        a.is_nan() && b.is_nan()
    } else {
        a.to_bits() == b.to_bits()
    }
}

/// Fully strict bit equality, used to prove that for NaN-free inputs the two
/// implementations agree down to the last bit (including NaN payloads that the
/// code *generates* rather than propagates).
pub fn feq_strict(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

pub fn veq_strict(a: c2v, b: c2v) -> bool {
    feq_strict(a.x, b.x) && feq_strict(a.y, b.y)
}

pub fn veq(a: c2v, b: c2v) -> bool {
    feq(a.x, b.x) && feq(a.y, b.y)
}

pub fn req(a: c2r, b: c2r) -> bool {
    feq(a.c, b.c) && feq(a.s, b.s)
}

pub fn cache_eq(a: &c2GJKCache, b: &c2GJKCache) -> bool {
    feq(a.metric, b.metric) && a.count == b.count && a.iA == b.iA && a.iB == b.iB
        && feq(a.div, b.div)
}

pub fn proxy_eq(a: &c2Proxy, b: &c2Proxy) -> bool {
    feq(a.radius, b.radius)
        && a.count == b.count
        && a.verts.iter().zip(b.verts.iter()).all(|(x, y)| veq(*x, *y))
}

pub fn sv_eq(a: &c2sv, b: &c2sv) -> bool {
    veq(a.sA, b.sA) && veq(a.sB, b.sB) && veq(a.p, b.p) && feq(a.u, b.u) && a.iA == b.iA
        && a.iB == b.iB
}

pub fn simplex_eq(a: &c2Simplex, b: &c2Simplex) -> bool {
    a.verts.iter().zip(b.verts.iter()).all(|(x, y)| sv_eq(x, y))
        && feq(a.div, b.div)
        && a.count == b.count
}

pub fn fdesc(v: f32) -> String {
    format!("{:e}(0x{:08x})", v, v.to_bits())
}

pub fn vdesc(v: c2v) -> String {
    format!("({}, {})", fdesc(v.x), fdesc(v.y))
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    /// Uniform in [0,1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in [-mag, mag].
    pub fn sym(&mut self, mag: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * mag
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Wide-exponent finite f32 (covers subnormals up to ~1e38).
    pub fn wide_f32(&mut self) -> f32 {
        let m = self.unit() * 2.0 - 1.0;
        let e = self.below(70) as i32 - 40; // 1e-40 .. 1e30
        let v = m * 10f32.powi(e);
        if v.is_finite() { v } else { m }
    }
    /// Anything at all, including NaN / inf / ±0 / subnormals.
    pub fn any_f32(&mut self) -> f32 {
        match self.below(12) {
            0 => f32::NAN,
            1 => -f32::NAN,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => 0.0,
            5 => -0.0,
            6 => f32::MAX,
            7 => f32::MIN_POSITIVE,
            8 => -f32::MAX,
            9 => f32::from_bits(1), // subnormal
            _ => self.wide_f32(),
        }
    }
    pub fn wide_v(&mut self) -> c2v {
        c2v { x: self.wide_f32(), y: self.wide_f32() }
    }
    pub fn any_v(&mut self) -> c2v {
        c2v { x: self.any_f32(), y: self.any_f32() }
    }
    /// Small "geometry-scale" vector: nice values in [-mag, mag].
    pub fn geo_v(&mut self, mag: f32) -> c2v {
        c2v { x: self.sym(mag), y: self.sym(mag) }
    }
}

// ---------------------------------------------------------------------------
// Function-pointer signature types
// ---------------------------------------------------------------------------

pub type FnV = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnVvf = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnVvv = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnVvvv = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnFvv = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnFv = unsafe extern "C" fn(c2v) -> f32;
pub type FnVv = unsafe extern "C" fn(c2v) -> c2v;
pub type FnR = unsafe extern "C" fn() -> c2r;
pub type FnX = unsafe extern "C" fn() -> c2x;
pub type FnVrv = unsafe extern "C" fn(c2r, c2v) -> c2v;
pub type FnVxv = unsafe extern "C" fn(c2x, c2v) -> c2v;
pub type FnBBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const c_void, u32, *mut c2Proxy);
pub type FnSimplexF = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type FnSimplexV = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type FnSimplex = unsafe extern "C" fn(*mut c2Simplex);
pub type FnSupport = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
pub type FnWitness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
pub type FnGJK = unsafe extern "C" fn(
    *const c_void,
    u32,
    *const c2x,
    *const c_void,
    u32,
    *const c2x,
    *mut c2v,
    *mut c2v,
    c_int,
    *mut c_int,
    *mut c2GJKCache,
) -> f32;
pub type FnGjkCache = unsafe extern "C" fn(
    c_char,
    *mut c2v,
    *mut c2v,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
);

// ---------------------------------------------------------------------------
// The loaded library pair
// ---------------------------------------------------------------------------

pub struct Impl {
    pub name: &'static str,
    pub lib: Library,
}

impl Impl {
    pub fn sym<T>(&self, name: &str) -> Symbol<'_, T> {
        unsafe {
            self.lib
                .get(name.as_bytes())
                .unwrap_or_else(|e| panic!("{}: missing symbol `{}`: {}", self.name, name, e))
        }
    }
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    // Overridable so the same suite can be pointed at a C .so built with
    // different optimisation levels.
    if let Ok(p) = std::env::var("DIFF_C_SO") {
        return PathBuf::from(p);
    }
    let build = repo_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {} (build the C library first)", build.display(), e))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    found
        .pop()
        .unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

fn rust_so_path() -> PathBuf {
    // Overridable so the debug-profile cdylib can be verified as well.
    if let Ok(p) = std::env::var("DIFF_RUST_SO") {
        return PathBuf::from(p);
    }
    // Locate <root>/translation/target/{release,debug}/libgjk_cache_lib.so.
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libgjk_cache_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libgjk_cache_lib.so not found under {} (run `cargo build --release`)",
        base.display()
    );
}

/// Loads the C `.so` and the Rust `.so`. Both are used exclusively through
/// their exported symbols — no Rust function is ever called directly.
pub fn load() -> (Impl, Impl) {
    let c = unsafe { Library::new(c_so_path()) }.expect("failed to dlopen the C .so");
    let r = unsafe { Library::new(rust_so_path()) }.expect("failed to dlopen the Rust .so");
    (Impl { name: "C", lib: c }, Impl { name: "Rust", lib: r })
}

/// Number of randomized inputs per `CONFIGS.md` row.
pub const N: usize = 400;
/// Fixed seed so every run is reproducible.
pub const SEED: u64 = 0x5EED_1234_5678_9ABC;
