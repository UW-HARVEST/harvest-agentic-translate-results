//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and exposes one typed accessor per exported symbol.
//!
//! Nothing in here ever calls a Rust function directly — every call goes
//! through `dlsym` on the built `cdylib`, so the `#[no_mangle]` export
//! wrappers and the C ABI of every argument/return struct are under test too.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// ABI-compatible mirrors of the C types
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
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
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
#[derive(Copy, Clone, Debug)]
pub struct c2Poly {
    pub count: c_int,
    pub verts: [c2v; 8],
    pub norms: [c2v; 8],
}

impl Default for c2Poly {
    fn default() -> Self {
        c2Poly {
            count: 0,
            verts: [c2v::default(); 8],
            norms: [c2v::default(); 8],
        }
    }
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

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;
pub const C2_TYPE_POLY: c_int = 3;

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Bit-exact float identity: distinguishes `+0.0` from `-0.0` and compares NaN
/// payloads, which is what "byte-identical results" requires.
pub fn bits_eq(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

pub fn v_eq(a: c2v, b: c2v) -> bool {
    bits_eq(a.x, b.x) && bits_eq(a.y, b.y)
}

pub fn r_eq(a: c2r, b: c2r) -> bool {
    bits_eq(a.c, b.c) && bits_eq(a.s, b.s)
}

pub fn cast_eq(a: c2Raycast, b: c2Raycast) -> bool {
    bits_eq(a.t, b.t) && v_eq(a.n, b.n)
}

pub fn fmt_v(v: c2v) -> String {
    format!("({:e}/{:#010x}, {:e}/{:#010x})", v.x, v.x.to_bits(), v.y, v.y.to_bits())
}

pub fn fmt_cast(c: c2Raycast) -> String {
    format!("{{ t: {:e}/{:#010x}, n: {} }}", c.t, c.t.to_bits(), fmt_v(c.n))
}

/// A poison pattern written into the out-params before every call, so that a
/// divergence in *which fields get written* is detected too (several C paths
/// return 0 without touching `*out`, and `c2RaytoCapsule` writes `*out` even
/// when it returns 0).
pub const POISON: c2Raycast = c2Raycast {
    t: -12345.678,
    n: c2v {
        x: 98765.4,
        y: -54321.25,
    },
};

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|s| s == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    found.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}: build it with \
             `cd c_src && mkdir -p build && cd build && cmake .. \
             -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    // current_exe is <...>/target/<profile>/deps/<testbin>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe.parent().unwrap().parent().unwrap();
    let p = profile_dir.join("libpoly_ray_lib.so");
    if p.exists() {
        return p;
    }
    for prof in ["release", "debug"] {
        let q = manifest_dir()
            .join("target")
            .join(prof)
            .join("libpoly_ray_lib.so");
        if q.exists() {
            return q;
        }
    }
    panic!("no Rust libpoly_ray_lib.so found near {}", exe.display());
}

macro_rules! decl_api {
    ( $( $name:ident : $ty:ty ; )* ) => {
        /// One loaded shared object with every exported symbol resolved.
        pub struct Api {
            _lib: Library,
            pub tag: &'static str,
            $( pub $name: $ty, )*
        }

        impl Api {
            unsafe fn load(path: &std::path::Path, tag: &'static str) -> Api {
                let lib = Library::new(path)
                    .unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
                $(
                    let $name: $ty = {
                        let s: Symbol<$ty> = lib
                            .get(concat!(stringify!($name), "\0").as_bytes())
                            .unwrap_or_else(|e| panic!(
                                "dlsym {} in {}: {e}", stringify!($name), path.display()));
                        *s
                    };
                )*
                Api { _lib: lib, tag, $( $name, )* }
            }
        }
    };
}

decl_api! {
    c2V            : unsafe extern "C" fn(f32, f32) -> c2v;
    c2Dot          : unsafe extern "C" fn(c2v, c2v) -> f32;
    c2Len          : unsafe extern "C" fn(c2v) -> f32;
    c2Add          : unsafe extern "C" fn(c2v, c2v) -> c2v;
    c2Sub          : unsafe extern "C" fn(c2v, c2v) -> c2v;
    c2Mulvs        : unsafe extern "C" fn(c2v, f32) -> c2v;
    c2Div          : unsafe extern "C" fn(c2v, f32) -> c2v;
    c2Norm         : unsafe extern "C" fn(c2v) -> c2v;
    c2Minv         : unsafe extern "C" fn(c2v, c2v) -> c2v;
    c2Maxv         : unsafe extern "C" fn(c2v, c2v) -> c2v;
    c2Skew         : unsafe extern "C" fn(c2v) -> c2v;
    c2Absv         : unsafe extern "C" fn(c2v) -> c2v;
    c2CCW90        : unsafe extern "C" fn(c2v) -> c2v;
    c2MulmvT       : unsafe extern "C" fn(c2m, c2v) -> c2v;
    c2RotIdentity  : unsafe extern "C" fn() -> c2r;
    c2xIdentity    : unsafe extern "C" fn() -> c2x;
    c2Mulrv        : unsafe extern "C" fn(c2r, c2v) -> c2v;
    c2MulrvT       : unsafe extern "C" fn(c2r, c2v) -> c2v;
    c2MulxvT       : unsafe extern "C" fn(c2x, c2v) -> c2v;
    c2AABBtoAABB   : unsafe extern "C" fn(c2AABB, c2AABB) -> c_int;
    c2AABBtoPoint  : unsafe extern "C" fn(c2AABB, c2v) -> c_int;
    c2CircleToPoint: unsafe extern "C" fn(c2Circle, c2v) -> c_int;
    c2RaytoCircle  : unsafe extern "C" fn(c2Ray, c2Circle, *mut c2Raycast) -> c_int;
    c2RaytoAABB    : unsafe extern "C" fn(c2Ray, c2AABB, *mut c2Raycast) -> c_int;
    c2RaytoCapsule : unsafe extern "C" fn(c2Ray, c2Capsule, *mut c2Raycast) -> c_int;
    c2RaytoPoly    : unsafe extern "C" fn(c2Ray, *const c2Poly, *const c2x, *mut c2Raycast) -> c_int;
    c2CastRay      : unsafe extern "C" fn(c2Ray, *const c_void, *const c2x, c_int, *mut c2Raycast) -> c_int;
    poly_ray       : unsafe extern "C" fn(*mut c2Raycast, *mut c2Raycast) -> c_int;
}

/// The C and Rust implementations, both loaded via `dlopen`/`dlsym`.
pub struct Pair {
    pub c: Api,
    pub rs: Api,
}

pub fn load_pair() -> Pair {
    let cp = find_c_so();
    let rp = find_rust_so();
    unsafe {
        Pair {
            c: Api::load(&cp, "C"),
            rs: Api::load(&rp, "RUST"),
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed, reproducible) + value generators
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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

    /// Uniform in [0,1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in [-a, a].
    pub fn sym(&mut self, a: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * a
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// A "normal" finite float with a wide but well-behaved exponent range.
    pub fn f_normal(&mut self) -> f32 {
        let mag = 10f32.powf(self.range(-4.0, 4.0));
        self.sym(1.0) * mag
    }

    /// Small finite float, good for geometry that must actually intersect.
    pub fn f_small(&mut self) -> f32 {
        self.sym(20.0)
    }

    pub fn v_normal(&mut self) -> c2v {
        c2v {
            x: self.f_normal(),
            y: self.f_normal(),
        }
    }

    pub fn v_small(&mut self) -> c2v {
        c2v {
            x: self.f_small(),
            y: self.f_small(),
        }
    }

    /// Unit-ish direction from a random angle.
    pub fn v_dir(&mut self) -> c2v {
        let a = self.range(-7.0, 7.0);
        c2v {
            x: a.cos(),
            y: a.sin(),
        }
    }

    pub fn rot_unit(&mut self) -> c2r {
        let a = self.range(-7.0, 7.0);
        c2r {
            c: a.cos(),
            s: a.sin(),
        }
    }

    /// Pick from the pathological-float pool.
    pub fn f_weird(&mut self) -> f32 {
        WEIRD[self.below(WEIRD.len())]
    }

    pub fn v_weird(&mut self) -> c2v {
        c2v {
            x: self.f_weird(),
            y: self.f_weird(),
        }
    }

    /// Mostly-normal float that occasionally injects a pathological value.
    pub fn f_mixed(&mut self) -> f32 {
        if self.below(8) == 0 {
            self.f_weird()
        } else {
            self.f_normal()
        }
    }

    pub fn v_mixed(&mut self) -> c2v {
        c2v {
            x: self.f_mixed(),
            y: self.f_mixed(),
        }
    }
}

/// Every boundary / pathological float the C source can be pushed through:
/// the comparison constants that literally appear in `lib.c` (`0`, `1.0f`,
/// `0.5f`, `-1.0f`), signed zeros, denormals, infinities and NaNs.
pub const WEIRD: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    -2.0,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    1.0e-45, // smallest positive denormal
    -1.0e-45,
    f32::MAX,
    f32::MIN,
    1.0e30,
    -1.0e30,
    1.0e-30,
    -1.0e-30,
    16_777_216.0, // 2^24, first f32 with unit ULP > 1
    -16_777_216.0,
    3.869_416,
    -3.869_416,
    13.069_341,
    11.5,
    -11.5,
    0.875,
    -0.875,
    // NaNs with DISTINCT payloads. Without these, two different NaNs would
    // often compare equal by accident and a wrong operand-selection would go
    // unnoticed; with them the surviving payload names its source operand.
    QNAN_A,
    QNAN_B,
    QNAN_NEG_A,
    QNAN_NEG_B,
    // Signaling NaNs: arithmetic must quiet them (set the mantissa MSB) while
    // preserving sign and payload; a plain move must not.
    SNAN_A,
    SNAN_NEG_A,
];

pub const QNAN_A: f32 = f32::from_bits(0x7fc0_1234);
pub const QNAN_B: f32 = f32::from_bits(0x7fc7_6543);
pub const QNAN_NEG_A: f32 = f32::from_bits(0xffc0_1234);
pub const QNAN_NEG_B: f32 = f32::from_bits(0xffc7_6543);
pub const SNAN_A: f32 = f32::from_bits(0x7f80_4321);
pub const SNAN_NEG_A: f32 = f32::from_bits(0xff80_4321);

// ---------------------------------------------------------------------------
// Differential assertion helpers
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct Diff {
    pub checked: usize,
    pub failures: Vec<String>,
}

impl Diff {
    pub fn new() -> Diff {
        Diff::default()
    }

    pub fn scalar(&mut self, what: &str, cv: f32, rv: f32) {
        self.checked += 1;
        if !bits_eq(cv, rv) {
            self.push(format!(
                "{what}: C = {cv:e} ({:#010x})  RUST = {rv:e} ({:#010x})",
                cv.to_bits(),
                rv.to_bits()
            ));
        }
    }

    pub fn vec(&mut self, what: &str, cv: c2v, rv: c2v) {
        self.checked += 1;
        if !v_eq(cv, rv) {
            self.push(format!(
                "{what}: C = {}  RUST = {}",
                fmt_v(cv),
                fmt_v(rv)
            ));
        }
    }

    pub fn rot(&mut self, what: &str, cv: c2r, rv: c2r) {
        self.checked += 1;
        if !r_eq(cv, rv) {
            self.push(format!("{what}: C = {cv:?}  RUST = {rv:?}"));
        }
    }

    pub fn int(&mut self, what: &str, cv: c_int, rv: c_int) {
        self.checked += 1;
        if cv != rv {
            self.push(format!("{what}: C = {cv}  RUST = {rv}"));
        }
    }

    /// Compares return value AND the full out-param (including fields the C
    /// leaves at the poison pattern).
    pub fn ray(&mut self, what: &str, cr: (c_int, c2Raycast), rr: (c_int, c2Raycast)) {
        self.checked += 1;
        if cr.0 != rr.0 || !cast_eq(cr.1, rr.1) {
            self.push(format!(
                "{what}:\n     C   ret={} out={}\n     RUST ret={} out={}",
                cr.0,
                fmt_cast(cr.1),
                rr.0,
                fmt_cast(rr.1)
            ));
        }
    }

    fn push(&mut self, msg: String) {
        if self.failures.len() < 25 {
            self.failures.push(msg);
        } else if self.failures.len() == 25 {
            self.failures.push("... (further failures suppressed)".into());
        }
    }

    pub fn finish(self, row: &str) {
        if !self.failures.is_empty() {
            panic!(
                "\n{} DIVERGENCE(S) out of {} comparisons in [{}]:\n  - {}\n",
                self.failures.len(),
                self.checked,
                row,
                self.failures.join("\n  - ")
            );
        }
        assert!(self.checked > 0, "[{row}] made no comparisons");
        eprintln!("[{row}] ok ({} comparisons)", self.checked);
    }
}

// ---------------------------------------------------------------------------
// Call wrappers that poison the out-param first
// ---------------------------------------------------------------------------

pub unsafe fn call_circle(api: &Api, a: c2Ray, b: c2Circle) -> (c_int, c2Raycast) {
    let mut out = POISON;
    let r = (api.c2RaytoCircle)(a, b, &mut out);
    (r, out)
}

pub unsafe fn call_aabb(api: &Api, a: c2Ray, b: c2AABB) -> (c_int, c2Raycast) {
    let mut out = POISON;
    let r = (api.c2RaytoAABB)(a, b, &mut out);
    (r, out)
}

pub unsafe fn call_capsule(api: &Api, a: c2Ray, b: c2Capsule) -> (c_int, c2Raycast) {
    let mut out = POISON;
    let r = (api.c2RaytoCapsule)(a, b, &mut out);
    (r, out)
}

pub unsafe fn call_poly(
    api: &Api,
    a: c2Ray,
    p: &c2Poly,
    bx: Option<&c2x>,
) -> (c_int, c2Raycast) {
    let mut out = POISON;
    let bxp = match bx {
        Some(x) => x as *const c2x,
        None => std::ptr::null(),
    };
    let r = (api.c2RaytoPoly)(a, p as *const c2Poly, bxp, &mut out);
    (r, out)
}

/// `c2RaytoPoly` against a raw byte buffer, so `count > 8` reads defined
/// (and *identical*) bytes in both languages instead of two different stacks.
pub unsafe fn call_poly_raw(
    api: &Api,
    a: c2Ray,
    buf: *const c2Poly,
    bx: Option<&c2x>,
) -> (c_int, c2Raycast) {
    let mut out = POISON;
    let bxp = match bx {
        Some(x) => x as *const c2x,
        None => std::ptr::null(),
    };
    let r = (api.c2RaytoPoly)(a, buf, bxp, &mut out);
    (r, out)
}

pub unsafe fn call_cast(
    api: &Api,
    a: c2Ray,
    shape: *const c_void,
    bx: Option<&c2x>,
    ty: c_int,
) -> (c_int, c2Raycast) {
    let mut out = POISON;
    let bxp = match bx {
        Some(x) => x as *const c2x,
        None => std::ptr::null(),
    };
    let r = (api.c2CastRay)(a, shape, bxp, ty, &mut out);
    (r, out)
}

// ---------------------------------------------------------------------------
// Polygon construction (valid convex hulls with correct outward normals)
// ---------------------------------------------------------------------------

/// Build a convex polygon with `count` vertices on an ellipse, in CCW order,
/// with the outward normals the C's `c2RaytoPoly` expects (edge `i` runs from
/// `verts[i]` to `verts[i+1]`, normal `i` is that edge's outward normal).
pub fn make_convex_poly(rng: &mut Rng, count: usize) -> c2Poly {
    assert!((1..=8).contains(&count));
    let cx = rng.sym(5.0);
    let cy = rng.sym(5.0);
    let rx = rng.range(0.5, 8.0);
    let ry = rng.range(0.5, 8.0);
    let phase = rng.range(0.0, 6.283_185);

    let mut p = c2Poly::default();
    p.count = count as c_int;
    for i in 0..count {
        let a = phase + 6.283_185_3 * (i as f32) / (count as f32);
        p.verts[i] = c2v {
            x: cx + rx * a.cos(),
            y: cy + ry * a.sin(),
        };
    }
    for i in 0..count {
        let j = (i + 1) % count;
        // outward normal of a CCW edge is the edge direction rotated -90 deg
        let ex = p.verts[j].x - p.verts[i].x;
        let ey = p.verts[j].y - p.verts[i].y;
        let (nx, ny) = if count == 1 {
            let a = rng.range(-7.0, 7.0);
            (a.cos(), a.sin())
        } else {
            let l = (ex * ex + ey * ey).sqrt();
            if l == 0.0 {
                (1.0, 0.0)
            } else {
                (ey / l, -ex / l)
            }
        };
        p.norms[i] = c2v { x: nx, y: ny };
    }
    p
}

/// An axis-aligned quad, normals exactly `(±1,0)/(0,±1)` — the shape family
/// `poly_ray` itself uses, and the one that produces `den == 0` most often.
pub fn make_axis_quad(rng: &mut Rng) -> c2Poly {
    let cx = rng.sym(6.0);
    let cy = rng.sym(6.0);
    let hw = rng.range(0.1, 6.0);
    let hh = rng.range(0.1, 6.0);
    let mut p = c2Poly::default();
    p.count = 4;
    p.verts[0] = c2v { x: cx + hw, y: cy - hh };
    p.verts[1] = c2v { x: cx + hw, y: cy + hh };
    p.verts[2] = c2v { x: cx - hw, y: cy + hh };
    p.verts[3] = c2v { x: cx - hw, y: cy - hh };
    p.norms[0] = c2v { x: 1.0, y: 0.0 };
    p.norms[1] = c2v { x: 0.0, y: 1.0 };
    p.norms[2] = c2v { x: -1.0, y: 0.0 };
    p.norms[3] = c2v { x: 0.0, y: -1.0 };
    p
}

pub fn poly_centroid(p: &c2Poly) -> c2v {
    let n = p.count.max(1) as usize;
    let n = n.min(8);
    let mut sx = 0.0f32;
    let mut sy = 0.0f32;
    for i in 0..n {
        sx += p.verts[i].x;
        sy += p.verts[i].y;
    }
    c2v {
        x: sx / n as f32,
        y: sy / n as f32,
    }
}

/// The four axis-aligned unit directions — these make `c2Dot(norm, d)` exactly
/// zero for half the edges of an axis-aligned polygon.
pub const AXIS_DIRS: [c2v; 4] = [
    c2v { x: 1.0, y: 0.0 },
    c2v { x: -1.0, y: 0.0 },
    c2v { x: 0.0, y: 1.0 },
    c2v { x: 0.0, y: -1.0 },
];
