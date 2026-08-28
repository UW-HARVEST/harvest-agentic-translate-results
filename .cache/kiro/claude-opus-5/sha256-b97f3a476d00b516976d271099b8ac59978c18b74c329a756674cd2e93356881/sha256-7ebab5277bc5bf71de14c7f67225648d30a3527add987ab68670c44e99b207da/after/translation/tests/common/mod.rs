//! Shared harness: loads the C and the Rust shared objects side by side and
//! exposes bit-exact comparison helpers.
//!
//! Every Rust function is reached through `libloading` on the produced
//! `cdylib` -- never called directly -- so the `#[no_mangle]` export wrappers
//! are part of what is under test.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// ABI-compatible mirrors of the C types
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Raycast {
    pub t: f32,
    pub n: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Ray {
    pub p: c2v,
    pub d: c2v,
    pub t: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2m {
    pub x: c2v,
    pub y: c2v,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

// ---------------------------------------------------------------------------
// Function pointer type aliases
// ---------------------------------------------------------------------------

pub type FnV = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnVV_f = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnV_f = unsafe extern "C" fn(c2v) -> f32;
pub type FnVV_V = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnVf_V = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnV_V = unsafe extern "C" fn(c2v) -> c2v;
pub type FnMV_V = unsafe extern "C" fn(c2m, c2v) -> c2v;
pub type FnAABBAABB_i = unsafe extern "C" fn(c2AABB, c2AABB) -> c_int;
pub type FnAABBV_i = unsafe extern "C" fn(c2AABB, c2v) -> c_int;
pub type FnCircleV_i = unsafe extern "C" fn(c2Circle, c2v) -> c_int;
pub type FnRayCircle_i = unsafe extern "C" fn(c2Ray, c2Circle, *mut c2Raycast) -> c_int;
pub type FnRayAABB_i = unsafe extern "C" fn(c2Ray, c2AABB, *mut c2Raycast) -> c_int;
pub type FnRayCapsule_i = unsafe extern "C" fn(c2Ray, c2Capsule, *mut c2Raycast) -> c_int;
pub type FnCastRay = unsafe extern "C" fn(c2Ray, *const c_void, c_int, *mut c2Raycast) -> c_int;
pub type FnSpecRay =
    unsafe extern "C" fn(*mut c2Raycast, f32, f32, f32, f32, f32, f32, f32) -> c_int;

// ---------------------------------------------------------------------------
// Library discovery / loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn first_so_in(dir: &Path) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "so").unwrap_or(false))
        .collect();
    found.sort();
    found.into_iter().next()
}

pub fn c_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("../c_src/build");
    first_so_in(&build).unwrap_or_else(|| {
        panic!(
            "no C .so found in {:?}; build it with cmake first",
            build
        )
    })
}

pub fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    // Prefer the artifact belonging to the profile the tests were built with.
    // The test binary lives in target/<profile>/deps/, so walk up from it.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|p| p.parent()) {
            let cand = profile_dir.join("libspec_ray_lib.so");
            if cand.exists() {
                return cand;
            }
        }
    }
    for profile in ["release", "debug"] {
        let cand = manifest_dir()
            .join("target")
            .join(profile)
            .join("libspec_ray_lib.so");
        if cand.exists() {
            return cand;
        }
    }
    panic!("libspec_ray_lib.so not found; run `cargo build` first");
}

pub struct Pair {
    pub c: Library,
    pub r: Library,
}

impl Pair {
    pub fn load() -> Pair {
        unsafe {
            let c = Library::new(c_lib_path()).expect("failed to load C .so");
            let r = Library::new(rust_lib_path()).expect("failed to load Rust .so");
            Pair { c, r }
        }
    }

    /// Fetch the same symbol from both libraries.
    pub fn sym<T>(&self, name: &str) -> (Symbol<'_, T>, Symbol<'_, T>) {
        let cname = std::ffi::CString::new(name).unwrap();
        unsafe {
            let cs: Symbol<T> = self
                .c
                .get(cname.as_bytes_with_nul())
                .unwrap_or_else(|e| panic!("C .so missing symbol {name}: {e}"));
            let rs: Symbol<T> = self
                .r
                .get(cname.as_bytes_with_nul())
                .unwrap_or_else(|e| panic!("Rust .so missing symbol {name}: {e}"));
            (cs, rs)
        }
    }
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub fn f_bits(v: f32) -> u32 {
    v.to_bits()
}

pub fn assert_f_eq(name: &str, ctx: &str, c: f32, r: f32) {
    if c.to_bits() != r.to_bits() {
        // A NaN is a NaN regardless of payload/sign; libm and Rust may pick
        // different quiet-NaN encodings for the same input.  Anything else
        // must match bit-for-bit.
        if c.is_nan() && r.is_nan() {
            return;
        }
        panic!(
            "{name} mismatch [{ctx}]: C = {c:?} (0x{:08x}) vs Rust = {r:?} (0x{:08x})",
            c.to_bits(),
            r.to_bits()
        );
    }
}

pub fn assert_v_eq(name: &str, ctx: &str, c: c2v, r: c2v) {
    assert_f_eq(name, &format!("{ctx} .x"), c.x, r.x);
    assert_f_eq(name, &format!("{ctx} .y"), c.y, r.y);
}

pub fn assert_i_eq(name: &str, ctx: &str, c: c_int, r: c_int) {
    assert_eq!(c, r, "{name} return mismatch [{ctx}]");
}

pub fn assert_cast_eq(name: &str, ctx: &str, c: &c2Raycast, r: &c2Raycast) {
    assert_f_eq(name, &format!("{ctx} out.t"), c.t, r.t);
    assert_v_eq(name, &format!("{ctx} out.n"), c.n, r.n);
}

/// Sentinel used to fill the out-parameter so that "C never wrote here" is
/// itself an observable, comparable fact.
pub const SENTINEL: c2Raycast = c2Raycast {
    t: -123.456,
    n: c2v {
        x: 987.654,
        y: -543.21,
    },
};

// ---------------------------------------------------------------------------
// Deterministic input generation
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in [-mag, mag].
    pub fn sym(&mut self, mag: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * mag
    }
    /// A float drawn from a mix of "ordinary" and "nasty" distributions.
    pub fn float(&mut self) -> f32 {
        match self.next_u32() % 16 {
            0 => 0.0,
            1 => -0.0,
            2 => 1.0,
            3 => -1.0,
            4 => f32::INFINITY,
            5 => f32::NEG_INFINITY,
            6 => f32::NAN,
            7 => f32::from_bits(1),               // smallest subnormal
            8 => f32::MIN_POSITIVE,
            9 => f32::MAX,
            10 => -f32::MAX,
            11 => self.sym(1.0e-20),
            12 => self.sym(1.0e20),
            13 => f32::from_bits(self.next_u32()), // fully random bit pattern
            _ => self.sym(100.0),
        }
    }
    /// A "well behaved" finite float in a geometry-friendly range.
    pub fn tame(&mut self) -> f32 {
        match self.next_u32() % 8 {
            0 => 0.0,
            1 => 1.0,
            2 => -1.0,
            3 => self.sym(0.001),
            _ => self.sym(50.0),
        }
    }
    pub fn vec_wild(&mut self) -> c2v {
        c2v {
            x: self.float(),
            y: self.float(),
        }
    }
    pub fn vec_tame(&mut self) -> c2v {
        c2v {
            x: self.tame(),
            y: self.tame(),
        }
    }
}

/// A grid of hand-picked scalars exercised exhaustively where feasible.
pub const EDGE_SCALARS: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    -2.0,
    3.0,
    1.0e-30,
    -1.0e-30,
    1.0e30,
    -1.0e30,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    f32::MAX,
    f32::MIN_POSITIVE,
];
