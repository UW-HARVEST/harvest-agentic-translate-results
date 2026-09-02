//! Shared harness: loads BOTH the C `.so` and the Rust `.so` with `libloading`
//! and exposes one struct per library so every call in every test crosses a
//! real FFI boundary through the exported symbols.
//!
//! Nothing here calls the Rust crate directly — `translation` is a `cdylib`, and
//! the tests deliberately treat it as an opaque shared object, exactly as an
//! external C consumer would. This also exercises the `#[no_mangle]` wrappers
//! and the System V struct-passing ABI.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// ABI-identical mirrors of the C types
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Raycast {
    pub t: f32,
    pub n: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Ray {
    pub p: c2v,
    pub d: c2v,
    pub t: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2m {
    pub x: c2v,
    pub y: c2v,
}

pub const C2_TYPE_CIRCLE: i32 = 0;
pub const C2_TYPE_AABB: i32 = 1;
pub const C2_TYPE_CAPSULE: i32 = 2;

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Bit-for-bit float identity. NaNs with different signs or payloads are NOT
/// equal, and `+0.0 != -0.0`. This is the only comparison the tests use.
pub fn bits_eq(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

pub fn v_eq(a: c2v, b: c2v) -> bool {
    bits_eq(a.x, b.x) && bits_eq(a.y, b.y)
}

pub fn rc_eq(a: c2Raycast, b: c2Raycast) -> bool {
    bits_eq(a.t, b.t) && v_eq(a.n, b.n)
}

pub fn fmt_f(v: f32) -> String {
    format!("{:e}(0x{:08x})", v, v.to_bits())
}

pub fn fmt_v(v: c2v) -> String {
    format!("({}, {})", fmt_f(v.x), fmt_f(v.y))
}

pub fn fmt_rc(v: c2Raycast) -> String {
    format!("{{ t: {}, n: {} }}", fmt_f(v.t), fmt_v(v.n))
}

// ---------------------------------------------------------------------------
// Deterministic RNG (fixed seed -> reproducible property-style testing)
// ---------------------------------------------------------------------------

/// SplitMix64. Self-contained so the tests need no extra dependency and the
/// sequence is stable across platforms and toolchains.
pub struct Rng(u64);

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

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// A "well behaved" coordinate in a modest range — the distribution that
    /// actually produces geometric hits.
    pub fn coord(&mut self) -> f32 {
        self.range(-100.0, 100.0)
    }

    /// A positive radius / length.
    pub fn radius(&mut self) -> f32 {
        self.range(0.001, 50.0)
    }

    /// A float drawn from a wide log-uniform magnitude range, both signs.
    pub fn wide(&mut self) -> f32 {
        let exp = self.range(-30.0, 30.0);
        let mag = 10f32.powf(exp);
        if self.bool() {
            -mag
        } else {
            mag
        }
    }

    /// A completely arbitrary bit pattern reinterpreted as `f32` (may be any
    /// class: normal, subnormal, inf, NaN with arbitrary payload).
    pub fn any_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// Mixes normal values with the interesting special classes.
    pub fn spicy(&mut self) -> f32 {
        match self.below(10) {
            0..=4 => self.coord(),
            5 => self.wide(),
            6 => SPECIALS[self.below(SPECIALS.len())],
            7 => self.any_bits(),
            8 => 0.0,
            _ => -0.0,
        }
    }

    pub fn vec_coord(&mut self) -> c2v {
        c2v {
            x: self.coord(),
            y: self.coord(),
        }
    }

    pub fn vec_spicy(&mut self) -> c2v {
        c2v {
            x: self.spicy(),
            y: self.spicy(),
        }
    }
}

/// Every float class the C branches distinguish, including several distinct NaN
/// bit patterns (sign bit set/clear, non-default payloads) because the C's
/// ternary `abs`/`min`/`max` idioms and `addss`/`mulss` operand order are all
/// NaN-sign/payload sensitive.
pub const SPECIALS: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    f32::EPSILON,
    -f32::EPSILON,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    1e-45,  // smallest positive subnormal
    -1e-45, // smallest negative subnormal
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    1e30,
    -1e30,
    1e-30,
    -1e-30,
    16777216.0,  // 2^24, first integer not exactly representable + 1
    -16777216.0,
    3.4028235e38,
];

/// NaN payloads that are distinguishable bit patterns.
pub const NAN_BITS: &[u32] = &[
    0x7fc0_0000, // default quiet NaN
    0xffc0_0000, // negative quiet NaN
    0x7fc0_1234, // quiet NaN, custom payload
    0xffff_ffff, // negative quiet NaN, all payload bits set
    0x7f80_0001, // signalling NaN
    0xff80_0001, // negative signalling NaN
];

// ---------------------------------------------------------------------------
// Library location
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let entries = std::fs::read_dir(&build).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}. Build the C library first:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    });
    let mut found = None;
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().map(|x| x == "so").unwrap_or(false) {
            found = Some(p);
            break;
        }
    }
    found.unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    // The integration-test binary lives in target/<profile>/deps/, so the
    // cdylib built by the same `cargo test` invocation is one level up.
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().unwrap().to_path_buf(); // .../deps
    if dir.file_name().map(|n| n == "deps").unwrap_or(false) {
        dir.pop();
    }
    let candidate = dir.join("libgen_ray_lib.so");
    if candidate.exists() {
        return candidate;
    }
    for profile in ["release", "debug"] {
        let c = workspace_root()
            .join("translation/target")
            .join(profile)
            .join("libgen_ray_lib.so");
        if c.exists() {
            return c;
        }
    }
    panic!(
        "libgen_ray_lib.so not found (looked in {}). Run `cargo build` first.",
        dir.display()
    );
}

// ---------------------------------------------------------------------------
// Typed function pointers
// ---------------------------------------------------------------------------

type FnVV = unsafe extern "C" fn(f32, f32) -> c2v;
type FnV_F = unsafe extern "C" fn(c2v) -> f32;
type FnVV_F = unsafe extern "C" fn(c2v, c2v) -> f32;
type FnV_V = unsafe extern "C" fn(c2v) -> c2v;
type FnVV_V = unsafe extern "C" fn(c2v, c2v) -> c2v;
type FnVF_V = unsafe extern "C" fn(c2v, f32) -> c2v;
type FnMV_V = unsafe extern "C" fn(c2m, c2v) -> c2v;
type FnRayCircle = unsafe extern "C" fn(c2Ray, c2Circle, *mut c2Raycast) -> i32;
type FnRayAABB = unsafe extern "C" fn(c2Ray, c2AABB, *mut c2Raycast) -> i32;
type FnRayCapsule = unsafe extern "C" fn(c2Ray, c2Capsule, *mut c2Raycast) -> i32;
type FnAABBAABB = unsafe extern "C" fn(c2AABB, c2AABB) -> i32;
type FnAABBPoint = unsafe extern "C" fn(c2AABB, c2v) -> i32;
type FnCirclePoint = unsafe extern "C" fn(c2Circle, c2v) -> i32;
type FnCastRay = unsafe extern "C" fn(c2Ray, *const std::ffi::c_void, i32, *mut c2Raycast) -> i32;
#[rustfmt::skip]
type FnGenRay = unsafe extern "C" fn(
    *mut c2Raycast, *mut c2Raycast, *mut c2Raycast,
    f32, f32, f32, f32, f32, f32, f32,
    f32, f32, f32, f32, f32,
    f32, f32, f32, f32,
) -> i32;

/// One loaded shared object with every exported symbol resolved.
pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,

    pub c2V: FnVV,
    pub c2Dot: FnVV_F,
    pub c2Len: FnV_F,
    pub c2Add: FnVV_V,
    pub c2Sub: FnVV_V,
    pub c2Mulvs: FnVF_V,
    pub c2Div: FnVF_V,
    pub c2Norm: FnV_V,
    pub c2Minv: FnVV_V,
    pub c2Maxv: FnVV_V,
    pub c2Skew: FnV_V,
    pub c2Absv: FnV_V,
    pub c2CCW90: FnV_V,
    pub c2MulmvT: FnMV_V,
    pub c2RaytoCircle: FnRayCircle,
    pub c2AABBtoAABB: FnAABBAABB,
    pub c2RaytoAABB: FnRayAABB,
    pub c2AABBtoPoint: FnAABBPoint,
    pub c2CircleToPoint: FnCirclePoint,
    pub c2RaytoCapsule: FnRayCapsule,
    pub c2CastRay: FnCastRay,
    pub gen_ray: FnGenRay,
}

macro_rules! sym {
    ($lib:expr, $name:literal, $ty:ty) => {{
        let s: Symbol<$ty> = $lib
            .get(concat!($name, "\0").as_bytes())
            .unwrap_or_else(|e| panic!("symbol `{}` missing: {e}", $name));
        *s
    }};
}

impl Lib {
    unsafe fn open(name: &'static str, path: PathBuf) -> Lib {
        let lib = Library::new(&path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
        let l = Lib {
            name,
            c2V: sym!(lib, "c2V", FnVV),
            c2Dot: sym!(lib, "c2Dot", FnVV_F),
            c2Len: sym!(lib, "c2Len", FnV_F),
            c2Add: sym!(lib, "c2Add", FnVV_V),
            c2Sub: sym!(lib, "c2Sub", FnVV_V),
            c2Mulvs: sym!(lib, "c2Mulvs", FnVF_V),
            c2Div: sym!(lib, "c2Div", FnVF_V),
            c2Norm: sym!(lib, "c2Norm", FnV_V),
            c2Minv: sym!(lib, "c2Minv", FnVV_V),
            c2Maxv: sym!(lib, "c2Maxv", FnVV_V),
            c2Skew: sym!(lib, "c2Skew", FnV_V),
            c2Absv: sym!(lib, "c2Absv", FnV_V),
            c2CCW90: sym!(lib, "c2CCW90", FnV_V),
            c2MulmvT: sym!(lib, "c2MulmvT", FnMV_V),
            c2RaytoCircle: sym!(lib, "c2RaytoCircle", FnRayCircle),
            c2AABBtoAABB: sym!(lib, "c2AABBtoAABB", FnAABBAABB),
            c2RaytoAABB: sym!(lib, "c2RaytoAABB", FnRayAABB),
            c2AABBtoPoint: sym!(lib, "c2AABBtoPoint", FnAABBPoint),
            c2CircleToPoint: sym!(lib, "c2CircleToPoint", FnCirclePoint),
            c2RaytoCapsule: sym!(lib, "c2RaytoCapsule", FnRayCapsule),
            c2CastRay: sym!(lib, "c2CastRay", FnCastRay),
            gen_ray: sym!(lib, "gen_ray", FnGenRay),
            path,
            _lib: lib,
        };
        l
    }
}

/// Both libraries, loaded once per test process.
pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

static INIT: std::sync::OnceLock<Pair> = std::sync::OnceLock::new();

pub fn libs() -> &'static Pair {
    INIT.get_or_init(|| unsafe {
        Pair {
            c: Lib::open("C", find_c_so()),
            r: Lib::open("Rust", find_rust_so()),
        }
    })
}

// ---------------------------------------------------------------------------
// Differential assertion plumbing
// ---------------------------------------------------------------------------

/// Accumulates divergences so one test run reports many failures at once
/// instead of aborting on the first.
pub struct Diff {
    pub label: String,
    pub cases: usize,
    pub fails: Vec<String>,
}

impl Diff {
    pub fn new(label: impl Into<String>) -> Diff {
        Diff {
            label: label.into(),
            cases: 0,
            fails: Vec::new(),
        }
    }

    pub fn check(&mut self, ok: bool, detail: impl FnOnce() -> String) {
        self.cases += 1;
        if !ok && self.fails.len() < 20 {
            self.fails.push(detail());
        } else if !ok {
            self.fails.push(String::from("<...further failures elided>"));
        }
    }

    /// Compare an `int` return plus the (possibly written) out-parameter.
    pub fn check_ray(
        &mut self,
        cr: i32,
        co: c2Raycast,
        rr: i32,
        ro: c2Raycast,
        ctx: impl FnOnce() -> String,
    ) {
        let ok = cr == rr && rc_eq(co, ro);
        self.check(ok, || {
            format!(
                "{}\n    C   -> ret={} out={}\n    Rust-> ret={} out={}",
                ctx(),
                cr,
                fmt_rc(co),
                rr,
                fmt_rc(ro)
            )
        });
    }

    pub fn check_v(&mut self, cv: c2v, rv: c2v, ctx: impl FnOnce() -> String) {
        let ok = v_eq(cv, rv);
        self.check(ok, || {
            format!(
                "{}\n    C   -> {}\n    Rust-> {}",
                ctx(),
                fmt_v(cv),
                fmt_v(rv)
            )
        });
    }

    pub fn check_f(&mut self, cf: f32, rf: f32, ctx: impl FnOnce() -> String) {
        let ok = bits_eq(cf, rf);
        self.check(ok, || {
            format!(
                "{}\n    C   -> {}\n    Rust-> {}",
                ctx(),
                fmt_f(cf),
                fmt_f(rf)
            )
        });
    }

    pub fn check_i(&mut self, ci: i32, ri: i32, ctx: impl FnOnce() -> String) {
        let ok = ci == ri;
        self.check(ok, || {
            format!("{}\n    C   -> {}\n    Rust-> {}", ctx(), ci, ri)
        });
    }

    #[track_caller]
    pub fn finish(self) {
        assert!(self.cases > 0, "{}: no cases were exercised", self.label);
        if !self.fails.is_empty() {
            panic!(
                "{}: {} of {} cases diverged:\n{}",
                self.label,
                self.fails.len(),
                self.cases,
                self.fails.join("\n")
            );
        }
        eprintln!("  {} ok ({} cases)", self.label, self.cases);
    }
}

/// A poison pattern written into out-params before each call so a test can tell
/// "untouched" from "written with the same value".
pub const POISON: c2Raycast = c2Raycast {
    t: -12345.678,
    n: c2v {
        x: -98765.4,
        y: 54321.125,
    },
};

/// Runs one raycast-style call against both libraries with fresh poisoned
/// out-params and returns `(c_ret, c_out, rust_ret, rust_out)`.
pub fn both_ray<T: Copy>(
    f: impl Fn(&Lib, c2Ray, T, *mut c2Raycast) -> i32,
    ray: c2Ray,
    shape: T,
) -> (i32, c2Raycast, i32, c2Raycast) {
    let l = libs();
    let mut co = POISON;
    let mut ro = POISON;
    let cr = f(&l.c, ray, shape, &mut co);
    let rr = f(&l.r, ray, shape, &mut ro);
    (cr, co, rr, ro)
}
