//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and exposes matching typed entry points, so every assertion in
//! the test suite crosses a real FFI boundary in both directions.
//!
//! Nothing here calls a Rust function directly; the Rust side is always reached
//! through `dlsym` on `libspec_ray_lib.so`, exactly as an external C consumer
//! would reach it.

#![allow(non_snake_case, dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI types -- byte-for-byte mirrors of include/lib.h and src/lib.c
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

impl C2v {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    /// Bitwise identity, so `NaN == NaN` when payloads agree and
    /// `+0.0 != -0.0`. Numeric equality is not good enough for a
    /// byte-identical claim.
    pub fn bits(&self) -> (u32, u32) {
        (self.x.to_bits(), self.y.to_bits())
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2Raycast {
    pub t: f32,
    pub n: C2v,
}

impl C2Raycast {
    pub fn bits(&self) -> (u32, u32, u32) {
        (self.t.to_bits(), self.n.x.to_bits(), self.n.y.to_bits())
    }
    /// A recognisable poison pattern, so a test can tell "the callee wrote
    /// this" from "the callee left it alone".
    pub fn poison() -> Self {
        Self {
            t: f32::from_bits(0xDEAD_BEEF),
            n: C2v {
                x: f32::from_bits(0xCAFE_BABE),
                y: f32::from_bits(0xFEED_FACE),
            },
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2AABB {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2Ray {
    pub p: C2v,
    pub d: C2v,
    pub t: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2m {
    pub x: C2v,
    pub y: C2v,
}

// C2_TYPE enumerators. The C enum has only these three, but a C enum
// parameter accepts any `int`, so tests also pass values outside the set.
pub const C2_TYPE_CIRCLE: u32 = 0;
pub const C2_TYPE_AABB: u32 = 1;
pub const C2_TYPE_CAPSULE: u32 = 2;

// ---------------------------------------------------------------------------
// Function pointer types
// ---------------------------------------------------------------------------

pub type FnVV = unsafe extern "C" fn(C2v) -> C2v;
pub type FnVVV = unsafe extern "C" fn(C2v, C2v) -> C2v;
pub type FnFF = unsafe extern "C" fn(f32, f32) -> C2v;
pub type FnDot = unsafe extern "C" fn(C2v, C2v) -> f32;
pub type FnLen = unsafe extern "C" fn(C2v) -> f32;
pub type FnMulvs = unsafe extern "C" fn(C2v, f32) -> C2v;
pub type FnMulmvT = unsafe extern "C" fn(C2m, C2v) -> C2v;
pub type FnAABBtoAABB = unsafe extern "C" fn(C2AABB, C2AABB) -> i32;
pub type FnAABBtoPoint = unsafe extern "C" fn(C2AABB, C2v) -> i32;
pub type FnCircleToPoint = unsafe extern "C" fn(C2Circle, C2v) -> i32;
pub type FnRaytoCircle = unsafe extern "C" fn(C2Ray, C2Circle, *mut C2Raycast) -> i32;
pub type FnRaytoAABB = unsafe extern "C" fn(C2Ray, C2AABB, *mut C2Raycast) -> i32;
pub type FnRaytoCapsule = unsafe extern "C" fn(C2Ray, C2Capsule, *mut C2Raycast) -> i32;
pub type FnCastRay = unsafe extern "C" fn(C2Ray, *const std::ffi::c_void, u32, *mut C2Raycast) -> i32;
pub type FnSpecRay =
    unsafe extern "C" fn(*mut C2Raycast, f32, f32, f32, f32, f32, f32, f32) -> i32;

/// One loaded shared object plus every symbol the C `.so` exports, resolved
/// eagerly so a missing export fails loudly at load time rather than silently
/// skipping a test.
pub struct Api {
    pub name: &'static str,
    _lib: Library,

    pub c2V: FnFF,
    pub c2Dot: FnDot,
    pub c2Len: FnLen,
    pub c2Add: FnVVV,
    pub c2Sub: FnVVV,
    pub c2Mulvs: FnMulvs,
    pub c2Div: FnMulvs,
    pub c2Norm: FnVV,
    pub c2Minv: FnVVV,
    pub c2Maxv: FnVVV,
    pub c2Skew: FnVV,
    pub c2Absv: FnVV,
    pub c2CCW90: FnVV,
    pub c2MulmvT: FnMulmvT,
    pub c2AABBtoAABB: FnAABBtoAABB,
    pub c2AABBtoPoint: FnAABBtoPoint,
    pub c2CircleToPoint: FnCircleToPoint,
    pub c2RaytoCircle: FnRaytoCircle,
    pub c2RaytoAABB: FnRaytoAABB,
    pub c2RaytoCapsule: FnRaytoCapsule,
    pub c2CastRay: FnCastRay,
    pub spec_ray: FnSpecRay,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    let s: Symbol<T> = unsafe {
        lib.get(name).unwrap_or_else(|e| {
            panic!(
                "symbol {} missing: {e}",
                String::from_utf8_lossy(&name[..name.len() - 1])
            )
        })
    };
    *s
}

impl Api {
    unsafe fn load(name: &'static str, path: &PathBuf) -> Api {
        let lib = unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display()))
        };
        unsafe {
            Api {
                name,
                c2V: sym(&lib, b"c2V\0"),
                c2Dot: sym(&lib, b"c2Dot\0"),
                c2Len: sym(&lib, b"c2Len\0"),
                c2Add: sym(&lib, b"c2Add\0"),
                c2Sub: sym(&lib, b"c2Sub\0"),
                c2Mulvs: sym(&lib, b"c2Mulvs\0"),
                c2Div: sym(&lib, b"c2Div\0"),
                c2Norm: sym(&lib, b"c2Norm\0"),
                c2Minv: sym(&lib, b"c2Minv\0"),
                c2Maxv: sym(&lib, b"c2Maxv\0"),
                c2Skew: sym(&lib, b"c2Skew\0"),
                c2Absv: sym(&lib, b"c2Absv\0"),
                c2CCW90: sym(&lib, b"c2CCW90\0"),
                c2MulmvT: sym(&lib, b"c2MulmvT\0"),
                c2AABBtoAABB: sym(&lib, b"c2AABBtoAABB\0"),
                c2AABBtoPoint: sym(&lib, b"c2AABBtoPoint\0"),
                c2CircleToPoint: sym(&lib, b"c2CircleToPoint\0"),
                c2RaytoCircle: sym(&lib, b"c2RaytoCircle\0"),
                c2RaytoAABB: sym(&lib, b"c2RaytoAABB\0"),
                c2RaytoCapsule: sym(&lib, b"c2RaytoCapsule\0"),
                c2CastRay: sym(&lib, b"c2CastRay\0"),
                spec_ray: sym(&lib, b"spec_ray\0"),
                _lib: lib,
            }
        }
    }
}

pub struct Pair {
    pub c: Api,
    pub rust: Api,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let dir = workspace_root().join("c_src/build");
    let mut found = None;
    for entry in std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("c_src/build not built ({}): {e}", dir.display()))
    {
        let p = entry.expect("readdir entry").path();
        if p.extension().map(|e| e == "so").unwrap_or(false) {
            found = Some(p);
            break;
        }
    }
    found.unwrap_or_else(|| panic!("no .so in {}", dir.display()))
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let root = workspace_root().join("translation/target");
    for profile in ["release", "debug"] {
        let p = root.join(profile).join("libspec_ray_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libspec_ray_lib.so not found; run `cargo build --release` first");
}

/// Loaded once per test binary; both libraries are stateless so sharing is safe.
pub fn apis() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| unsafe {
        Pair {
            c: Api::load("C", &find_c_so()),
            rust: Api::load("Rust", &find_rust_so()),
        }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed -> reproducible failures)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
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
    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        lo + u * (hi - lo)
    }
    /// A "nice" magnitude value: mostly small, occasionally huge or tiny, plus
    /// exact zeros and boundary-ish values. Pure `range()` never produces the
    /// exact-equality cases the C branches on (`t >= 0`, `d != 0`, `disc < 0`).
    pub fn interesting(&mut self) -> f32 {
        match self.next_u32() % 16 {
            0 => 0.0,
            1 => -0.0,
            2 => 1.0,
            3 => -1.0,
            4 => self.range(-1e-6, 1e-6),
            5 => self.range(-1e6, 1e6),
            6 => (self.next_u32() % 9) as f32 - 4.0, // small integers
            7 => self.range(-2.0, 2.0),
            _ => self.range(-10.0, 10.0),
        }
    }
    /// Includes the non-finite and denormal values a real caller can pass.
    pub fn pathological(&mut self) -> f32 {
        match self.next_u32() % 24 {
            0 => f32::NAN,
            1 => -f32::NAN,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::MIN_POSITIVE,
            5 => f32::from_bits(1), // denormal
            6 => f32::MAX,
            7 => -f32::MAX,
            8 => 0.0,
            9 => -0.0,
            _ => self.interesting(),
        }
    }
    pub fn v(&mut self) -> C2v {
        C2v::new(self.interesting(), self.interesting())
    }
    pub fn v_path(&mut self) -> C2v {
        C2v::new(self.pathological(), self.pathological())
    }
}

/// Formats a float so a divergence report is unambiguous about NaN payloads
/// and signed zeros.
pub fn f(v: f32) -> String {
    format!("{v:?}[{:#010x}]", v.to_bits())
}

#[macro_export]
macro_rules! assert_bits_eq {
    ($c:expr, $r:expr, $($ctx:tt)*) => {{
        let (cv, rv) = ($c, $r);
        assert_eq!(
            cv, rv,
            "C/Rust divergence: C={:?} Rust={:?} :: {}",
            cv, rv, format_args!($($ctx)*)
        );
    }};
}

// ---------------------------------------------------------------------------
// Differential comparison helpers
// ---------------------------------------------------------------------------

/// Calls the C and Rust versions of an `int (c2Ray, Shape, c2Raycast*)`
/// function with an identical, pre-poisoned `out` buffer and asserts that the
/// return value AND all three floats of `*out` are bit-identical.
///
/// Poisoning matters: several C paths return without writing `*out`, so
/// "leaves the buffer alone" is part of the contract. A zeroed buffer would
/// hide a Rust that writes zeros where the C writes nothing.
pub fn cmp_ray<S: Copy + std::fmt::Debug>(
    what: &str,
    cfn: unsafe extern "C" fn(C2Ray, S, *mut C2Raycast) -> i32,
    rfn: unsafe extern "C" fn(C2Ray, S, *mut C2Raycast) -> i32,
    ray: C2Ray,
    shape: S,
    ctx: &str,
) {
    let mut oc = C2Raycast::poison();
    let mut or = C2Raycast::poison();
    let rc = unsafe { cfn(ray, shape, &mut oc) };
    let rr = unsafe { rfn(ray, shape, &mut or) };
    if (rc, oc.bits()) != (rr, or.bits()) {
        panic!(
            "{what} divergence [{ctx}]\n  ray   = p=({},{}) d=({},{}) t={}\n  shape = {shape:?}\n  C    : ret={rc} t={} n=({},{})\n  Rust : ret={rr} t={} n=({},{})",
            f(ray.p.x),
            f(ray.p.y),
            f(ray.d.x),
            f(ray.d.y),
            f(ray.t),
            f(oc.t),
            f(oc.n.x),
            f(oc.n.y),
            f(or.t),
            f(or.n.x),
            f(or.n.y),
        );
    }
}

/// Same as [`cmp_ray`] but passes `out == NULL`. Only safe on C paths that
/// provably return before any write; the caller is responsible for that.
pub fn cmp_ray_null_out<S: Copy + std::fmt::Debug>(
    what: &str,
    cfn: unsafe extern "C" fn(C2Ray, S, *mut C2Raycast) -> i32,
    rfn: unsafe extern "C" fn(C2Ray, S, *mut C2Raycast) -> i32,
    ray: C2Ray,
    shape: S,
    ctx: &str,
) {
    let rc = unsafe { cfn(ray, shape, std::ptr::null_mut()) };
    let rr = unsafe { rfn(ray, shape, std::ptr::null_mut()) };
    assert_eq!(
        rc, rr,
        "{what} NULL-out divergence [{ctx}]: C ret={rc} Rust ret={rr}, shape={shape:?}"
    );
}

/// `c2CastRay` differential call. `B` is passed as a raw pointer to the same
/// object for both libraries.
pub fn cmp_castray<S: Copy + std::fmt::Debug>(
    ray: C2Ray,
    shape: &S,
    tag: u32,
    ctx: &str,
) {
    let a = apis();
    let p = shape as *const S as *const std::ffi::c_void;
    let mut oc = C2Raycast::poison();
    let mut or = C2Raycast::poison();
    let rc = unsafe { (a.c.c2CastRay)(ray, p, tag, &mut oc) };
    let rr = unsafe { (a.rust.c2CastRay)(ray, p, tag, &mut or) };
    if (rc, oc.bits()) != (rr, or.bits()) {
        panic!(
            "c2CastRay divergence [{ctx}] tag={tag}\n  shape = {shape:?}\n  C    : ret={rc} out={:?}\n  Rust : ret={rr} out={:?}",
            oc.bits(),
            or.bits()
        );
    }
}

/// `spec_ray` differential call over its seven scalar arguments.
pub fn cmp_specray(args: [f32; 7], ctx: &str) {
    let a = apis();
    let [mp_x, mp_y, c_p_x, c_p_y, c_r, r_p_x, r_p_y] = args;
    let mut oc = C2Raycast::poison();
    let mut or = C2Raycast::poison();
    let rc = unsafe { (a.c.spec_ray)(&mut oc, mp_x, mp_y, c_p_x, c_p_y, c_r, r_p_x, r_p_y) };
    let rr = unsafe { (a.rust.spec_ray)(&mut or, mp_x, mp_y, c_p_x, c_p_y, c_r, r_p_x, r_p_y) };
    if (rc, oc.bits()) != (rr, or.bits()) {
        panic!(
            "spec_ray divergence [{ctx}]\n  args = mp=({},{}) c.p=({},{}) c.r={} ray.p=({},{})\n  C    : ret={rc} t={} n=({},{})\n  Rust : ret={rr} t={} n=({},{})",
            f(mp_x),
            f(mp_y),
            f(c_p_x),
            f(c_p_y),
            f(c_r),
            f(r_p_x),
            f(r_p_y),
            f(oc.t),
            f(oc.n.x),
            f(oc.n.y),
            f(or.t),
            f(or.n.x),
            f(or.n.y),
        );
    }
}

/// Compares a `c2v`-returning unary function bitwise.
pub fn cmp_v1(what: &str, cfn: FnVV, rfn: FnVV, a: C2v, ctx: &str) {
    let c = unsafe { cfn(a) };
    let r = unsafe { rfn(a) };
    assert_eq!(
        c.bits(),
        r.bits(),
        "{what} divergence [{ctx}]: in=({},{}) C=({},{}) Rust=({},{})",
        f(a.x),
        f(a.y),
        f(c.x),
        f(c.y),
        f(r.x),
        f(r.y)
    );
}

/// Compares a `c2v`-returning binary function bitwise.
pub fn cmp_v2(what: &str, cfn: FnVVV, rfn: FnVVV, a: C2v, b: C2v, ctx: &str) {
    let c = unsafe { cfn(a, b) };
    let r = unsafe { rfn(a, b) };
    assert_eq!(
        c.bits(),
        r.bits(),
        "{what} divergence [{ctx}]: a=({},{}) b=({},{}) C=({},{}) Rust=({},{})",
        f(a.x),
        f(a.y),
        f(b.x),
        f(b.y),
        f(c.x),
        f(c.y),
        f(r.x),
        f(r.y)
    );
}

/// Compares a `c2v (c2v, float)` function bitwise.
pub fn cmp_vs(what: &str, cfn: FnMulvs, rfn: FnMulvs, a: C2v, s: f32, ctx: &str) {
    let c = unsafe { cfn(a, s) };
    let r = unsafe { rfn(a, s) };
    assert_eq!(
        c.bits(),
        r.bits(),
        "{what} divergence [{ctx}]: a=({},{}) s={} C=({},{}) Rust=({},{})",
        f(a.x),
        f(a.y),
        f(s),
        f(c.x),
        f(c.y),
        f(r.x),
        f(r.y)
    );
}

/// Interesting f32 values shared by several rows: exact boundaries the C
/// compares against, plus the non-finite and signed-zero cases.
pub const EDGE_F32: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    -2.0,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    f32::MAX,
    -f32::MAX,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    1e-6,
    -1e-6,
    1e6,
    -1e6,
];

/// Denormals and distinct NaN payloads, kept separate so a row can opt in.
pub fn exotic_f32() -> Vec<f32> {
    vec![
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x007F_FFFF),
        f32::from_bits(0x7FC0_0001), // quiet NaN, payload 1
        f32::from_bits(0x7FC0_0002), // quiet NaN, payload 2
        f32::from_bits(0xFFC0_0003), // negative quiet NaN
        f32::from_bits(0x7F80_0001), // signalling NaN
    ]
}

/// One ULP up / down, for "one step past a valid range" probes.
pub fn step(v: f32, n: i32) -> f32 {
    let mut out = v;
    if n >= 0 {
        for _ in 0..n {
            out = next_up(out);
        }
    } else {
        for _ in 0..-n {
            out = next_down(out);
        }
    }
    out
}

fn next_up(v: f32) -> f32 {
    if v.is_nan() || v == f32::INFINITY {
        return v;
    }
    if v == 0.0 {
        return f32::from_bits(1);
    }
    let b = v.to_bits();
    f32::from_bits(if v > 0.0 { b + 1 } else { b - 1 })
}

fn next_down(v: f32) -> f32 {
    if v.is_nan() || v == f32::NEG_INFINITY {
        return v;
    }
    if v == 0.0 {
        return f32::from_bits(0x8000_0001);
    }
    let b = v.to_bits();
    f32::from_bits(if v > 0.0 { b - 1 } else { b + 1 })
}

/// Recomputes `c2RaytoAABB`'s intermediate stages using the C library's own
/// exported L0 helpers, so a test can positively identify WHICH branch a given
/// input takes instead of guessing. Returns
/// `(broad_phase_pass, sat_d, t[4], hit[4], same_side[4], zero_denom[4])`.
pub fn aabb_stages(ray: C2Ray, b: C2AABB) -> (i32, f32, [f32; 4], [bool; 4], [bool; 4], [bool; 4]) {
    let a = &apis().c;
    let p0 = ray.p;
    let p1 = unsafe { (a.c2Add)(ray.p, (a.c2Mulvs)(ray.d, ray.t)) };
    let a_box = C2AABB {
        min: unsafe { (a.c2Minv)(p0, p1) },
        max: unsafe { (a.c2Maxv)(p0, p1) },
    };
    let bp = unsafe { (a.c2AABBtoAABB)(a_box, b) };
    let ab = unsafe { (a.c2Sub)(p1, p0) };
    let n = unsafe { (a.c2Skew)(ab) };
    let abs_n = unsafe { (a.c2Absv)(n) };
    let half = unsafe { (a.c2Mulvs)((a.c2Sub)(b.max, b.min), 0.5) };
    let centre = unsafe { (a.c2Mulvs)((a.c2Add)(b.min, b.max), 0.5) };
    let dot = unsafe { (a.c2Dot)(n, (a.c2Sub)(p0, centre)) };
    let sat_d = (if dot < 0.0 { -dot } else { dot }) - unsafe { (a.c2Dot)(abs_n, half) };

    // static inline float c2SignedDistPointToPlane_OneDimensional(p, n, d)
    let sd = |p: f32, nn: f32, dd: f32| p * nn - dd * nn;
    let da = [
        sd(p0.x, -1.0, b.min.x),
        sd(p0.x, 1.0, b.max.x),
        sd(p0.y, -1.0, b.min.y),
        sd(p0.y, 1.0, b.max.y),
    ];
    let db = [
        sd(p1.x, -1.0, b.min.x),
        sd(p1.x, 1.0, b.max.x),
        sd(p1.y, -1.0, b.min.y),
        sd(p1.y, 1.0, b.max.y),
    ];
    let mut t = [0.0f32; 4];
    let mut same_side = [false; 4];
    let mut zero_denom = [false; 4];
    for i in 0..4 {
        // static inline float c2RayToPlane_OneDimensional(da, db)
        if da[i] < 0.0 {
            t[i] = 0.0;
        } else if da[i] * db[i] > 0.0 {
            t[i] = 1.0;
            same_side[i] = true;
        } else {
            let d = da[i] - db[i];
            if d != 0.0 {
                t[i] = da[i] / d;
            } else {
                t[i] = 0.0;
                zero_denom[i] = true;
            }
        }
    }
    let hit = [t[0] <= 1.0, t[1] <= 1.0, t[2] <= 1.0, t[3] <= 1.0];
    (bp, sat_d, t, hit, same_side, zero_denom)
}

/// Index of the slab the C's `>=` cascade selects (first match wins).
pub fn aabb_winner(t: [f32; 4], hit: [bool; 4]) -> usize {
    let z = [
        if hit[0] { t[0] } else { 0.0 },
        if hit[1] { t[1] } else { 0.0 },
        if hit[2] { t[2] } else { 0.0 },
        if hit[3] { t[3] } else { 0.0 },
    ];
    if z[0] >= z[1] && z[0] >= z[2] && z[0] >= z[3] {
        0
    } else if z[1] >= z[0] && z[1] >= z[2] && z[1] >= z[3] {
        1
    } else if z[2] >= z[0] && z[2] >= z[1] && z[2] >= z[3] {
        2
    } else {
        3
    }
}

