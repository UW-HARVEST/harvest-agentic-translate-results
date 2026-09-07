//! Shared differential-test harness.
//!
//! BOTH libraries are loaded through `libloading` — the Rust crate is never
//! called directly, only via the symbols its `cdylib` exports. That way the
//! `#[no_mangle]` / `extern "C"` wrappers and the C ABI struct passing are
//! part of what is under test.

#![allow(non_snake_case, dead_code, non_camel_case_types)]

use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// ABI types — mirror c_src/include/lib.h and c_src/src/lib.c exactly.
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2Raycast {
    pub t: f32,
    pub n: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2x {
    pub p: C2v,
    pub r: C2r,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2AABB {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct C2Poly {
    pub count: c_int,
    pub verts: [C2v; 8],
    pub norms: [C2v; 8],
}

impl Default for C2Poly {
    fn default() -> Self {
        C2Poly {
            count: 0,
            verts: [C2v::default(); 8],
            norms: [C2v::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2Ray {
    pub p: C2v,
    pub d: C2v,
    pub t: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2m {
    pub x: C2v,
    pub y: C2v,
}

/// `c2Poly` plus trailing slack so a `count > 8` overrun reads *initialized*
/// memory that is identical for both libraries instead of random stack bytes.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct PolyBuf {
    pub poly: C2Poly,
    pub slack: [C2v; 32],
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;
pub const C2_TYPE_POLY: c_int = 3;

// ---------------------------------------------------------------------------
// Function pointer types
// ---------------------------------------------------------------------------

pub type FnVff = unsafe extern "C" fn(f32, f32) -> C2v;
pub type FnFvv = unsafe extern "C" fn(C2v, C2v) -> f32;
pub type FnFv = unsafe extern "C" fn(C2v) -> f32;
pub type FnVvv = unsafe extern "C" fn(C2v, C2v) -> C2v;
pub type FnVvf = unsafe extern "C" fn(C2v, f32) -> C2v;
pub type FnVv = unsafe extern "C" fn(C2v) -> C2v;
pub type FnIbb = unsafe extern "C" fn(C2AABB, C2AABB) -> c_int;
pub type FnIbv = unsafe extern "C" fn(C2AABB, C2v) -> c_int;
pub type FnIcv = unsafe extern "C" fn(C2Circle, C2v) -> c_int;
pub type FnRayCircle = unsafe extern "C" fn(C2Ray, C2Circle, *mut C2Raycast) -> c_int;
pub type FnRayAABB = unsafe extern "C" fn(C2Ray, C2AABB, *mut C2Raycast) -> c_int;
pub type FnRayCapsule = unsafe extern "C" fn(C2Ray, C2Capsule, *mut C2Raycast) -> c_int;
pub type FnRayPoly =
    unsafe extern "C" fn(C2Ray, *const C2Poly, *const C2x, *mut C2Raycast) -> c_int;
pub type FnCastRay =
    unsafe extern "C" fn(C2Ray, *const c_void, *const C2x, c_int, *mut C2Raycast) -> c_int;
pub type FnPolyRay = unsafe extern "C" fn(*mut C2Raycast, *mut C2Raycast) -> c_int;
pub type FnR = unsafe extern "C" fn() -> C2r;
pub type FnX = unsafe extern "C" fn() -> C2x;
pub type FnVrv = unsafe extern "C" fn(C2r, C2v) -> C2v;
pub type FnVxv = unsafe extern "C" fn(C2x, C2v) -> C2v;
pub type FnVmv = unsafe extern "C" fn(C2m, C2v) -> C2v;

/// All 28 exported symbols, resolved from one shared object.
pub struct Api {
    pub name: &'static str,
    pub c2V: FnVff,
    pub c2Dot: FnFvv,
    pub c2Len: FnFv,
    pub c2Add: FnVvv,
    pub c2Sub: FnVvv,
    pub c2Mulvs: FnVvf,
    pub c2Div: FnVvf,
    pub c2Norm: FnVv,
    pub c2Minv: FnVvv,
    pub c2Maxv: FnVvv,
    pub c2Skew: FnVv,
    pub c2Absv: FnVv,
    pub c2CCW90: FnVv,
    pub c2RotIdentity: FnR,
    pub c2xIdentity: FnX,
    pub c2Mulrv: FnVrv,
    pub c2MulrvT: FnVrv,
    pub c2MulxvT: FnVxv,
    pub c2MulmvT: FnVmv,
    pub c2AABBtoAABB: FnIbb,
    pub c2AABBtoPoint: FnIbv,
    pub c2CircleToPoint: FnIcv,
    pub c2RaytoCircle: FnRayCircle,
    pub c2RaytoAABB: FnRayAABB,
    pub c2RaytoCapsule: FnRayCapsule,
    pub c2RaytoPoly: FnRayPoly,
    pub c2CastRay: FnCastRay,
    pub poly_ray: FnPolyRay,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<parent-dir-name>.so` — globbed, because CMake derives the
/// project name from the checkout directory name.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|e| e == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, got {:?}",
        build.display(),
        found
    );
    found.pop().unwrap()
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    // The test binary lives in target/<profile>/deps/, so prefer the cdylib in
    // the matching profile directory; fall back to the other profile.
    let exe = std::env::current_exe().unwrap();
    let deps = exe.parent().unwrap().to_path_buf();
    let mut cands = vec![deps.clone(), deps.parent().unwrap().to_path_buf()];
    cands.push(manifest_dir().join("target/debug"));
    cands.push(manifest_dir().join("target/release"));
    for dir in cands {
        let cand = dir.join("libpoly_ray_lib.so");
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "libpoly_ray_lib.so not found near {} — run `cargo build` first",
        exe.display()
    );
}

impl Api {
    pub fn load(name: &'static str, path: &std::path::Path) -> Api {
        unsafe {
            let lib: &'static Library = Box::leak(Box::new(
                Library::new(path)
                    .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display())),
            ));
            macro_rules! sym {
                ($n:literal) => {{
                    *lib.get($n).unwrap_or_else(|e| {
                        panic!("{} missing symbol {}: {e}", path.display(),
                               std::str::from_utf8($n).unwrap())
                    })
                }};
            }
            Api {
                name,
                c2V: sym!(b"c2V\0"),
                c2Dot: sym!(b"c2Dot\0"),
                c2Len: sym!(b"c2Len\0"),
                c2Add: sym!(b"c2Add\0"),
                c2Sub: sym!(b"c2Sub\0"),
                c2Mulvs: sym!(b"c2Mulvs\0"),
                c2Div: sym!(b"c2Div\0"),
                c2Norm: sym!(b"c2Norm\0"),
                c2Minv: sym!(b"c2Minv\0"),
                c2Maxv: sym!(b"c2Maxv\0"),
                c2Skew: sym!(b"c2Skew\0"),
                c2Absv: sym!(b"c2Absv\0"),
                c2CCW90: sym!(b"c2CCW90\0"),
                c2RotIdentity: sym!(b"c2RotIdentity\0"),
                c2xIdentity: sym!(b"c2xIdentity\0"),
                c2Mulrv: sym!(b"c2Mulrv\0"),
                c2MulrvT: sym!(b"c2MulrvT\0"),
                c2MulxvT: sym!(b"c2MulxvT\0"),
                c2MulmvT: sym!(b"c2MulmvT\0"),
                c2AABBtoAABB: sym!(b"c2AABBtoAABB\0"),
                c2AABBtoPoint: sym!(b"c2AABBtoPoint\0"),
                c2CircleToPoint: sym!(b"c2CircleToPoint\0"),
                c2RaytoCircle: sym!(b"c2RaytoCircle\0"),
                c2RaytoAABB: sym!(b"c2RaytoAABB\0"),
                c2RaytoCapsule: sym!(b"c2RaytoCapsule\0"),
                c2RaytoPoly: sym!(b"c2RaytoPoly\0"),
                c2CastRay: sym!(b"c2CastRay\0"),
                poly_ray: sym!(b"poly_ray\0"),
            }
        }
    }
}

/// The pair under test. Loaded once per test binary.
pub struct Pair {
    pub c: Api,
    pub r: Api,
}

pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair {
        c: Api::load("C", &c_so_path()),
        r: Api::load("Rust", &rust_so_path()),
    })
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers. Floats are compared by raw bit pattern so
// NaN payloads and the sign of zero are part of the assertion.
// ---------------------------------------------------------------------------

pub fn fb(v: f32) -> u32 {
    v.to_bits()
}

pub fn vb(v: C2v) -> (u32, u32) {
    (v.x.to_bits(), v.y.to_bits())
}

pub fn rb(v: C2r) -> (u32, u32) {
    (v.c.to_bits(), v.s.to_bits())
}

pub fn xb(v: C2x) -> ((u32, u32), (u32, u32)) {
    (vb(v.p), rb(v.r))
}

pub fn cb(v: C2Raycast) -> (u32, (u32, u32)) {
    (v.t.to_bits(), vb(v.n))
}

/// A dirty sentinel pattern written into `out` before every call, so that
/// "C left the field untouched" is observable and must be reproduced.
pub const DIRTY: C2Raycast = C2Raycast {
    t: -12345.678,
    n: C2v {
        x: 98765.43,
        y: -0.5e-30,
    },
};

/// Result of one raycast-style call: return code plus the full `out` struct.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct RayOut {
    pub ret: c_int,
    pub out: (u32, (u32, u32)),
}

pub fn ray_call<F: FnOnce(*mut C2Raycast) -> c_int>(f: F) -> RayOut {
    let mut o = DIRTY;
    let ret = f(&mut o as *mut C2Raycast);
    RayOut { ret, out: cb(o) }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) + float generators
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
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
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Uniform in [0,1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in [lo,hi).
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }
    pub fn bool(&mut self) -> bool {
        self.next_u32() & 1 == 1
    }

    /// "Well-behaved" float: moderate magnitude, both signs, occasional zero.
    pub fn normal_f32(&mut self) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => self.range(-1e-6, 1e-6),
            3 => self.range(-1e6, 1e6),
            _ => self.range(-20.0, 20.0),
        }
    }

    pub fn normal_v(&mut self) -> C2v {
        C2v {
            x: self.normal_f32(),
            y: self.normal_f32(),
        }
    }

    /// Adversarial float: mixes the IEEE special classes with random bits.
    ///
    /// Several *distinct* NaN payloads are included on purpose: x86
    /// `mulss`/`addss`/`subss` return the destination operand's NaN, so two
    /// different NaNs meeting in one expression is the only way to observe a
    /// mismatch in arithmetic operand order between the C and the Rust.
    pub fn mixed_f32(&mut self) -> f32 {
        const SPECIALS: [u32; 24] = [
            0x0000_0000, // +0.0
            0x8000_0000, // -0.0
            0x3F80_0000, // 1.0
            0xBF80_0000, // -1.0
            0x7F80_0000, // +inf
            0xFF80_0000, // -inf
            0x7FC0_0000, // default quiet NaN
            0xFFC0_0000, // x86 "indefinite" NaN (what 0*inf produces)
            0x7FC0_0001, // quiet NaN, distinct payload
            0xFFC0_1234, // quiet NaN, distinct payload, sign set
            0x7FFF_FFFF, // quiet NaN, all payload bits
            0x7F80_0001, // signaling NaN (quiets to 0x7FC00001)
            0xFF80_4321, // signaling NaN, sign set
            0x7FA0_0000, // signaling NaN
            0x0080_0000, // f32::MIN_POSITIVE
            0x8080_0000, // -f32::MIN_POSITIVE
            0x7F7F_FFFF, // f32::MAX
            0xFF7F_FFFF, // f32::MIN
            0x3400_0000, // f32::EPSILON
            0x0000_0001, // smallest positive subnormal
            0x8000_0001, // smallest negative subnormal
            0x3F00_0000, // 0.5
            0x4000_0000, // 2.0
            0xC000_0000, // -2.0
        ];
        match self.below(10) {
            0 | 1 | 2 | 3 => f32::from_bits(SPECIALS[self.below(SPECIALS.len() as u32) as usize]),
            4 => f32::from_bits(self.next_u32()), // any bit pattern at all
            _ => self.normal_f32(),
        }
    }

    /// Only NaNs, with a wide spread of payloads — used to pin down the
    /// destination-operand semantics of every arithmetic site.
    pub fn nan_f32(&mut self) -> f32 {
        const NANS: [u32; 10] = [
            0x7FC0_0000, 0xFFC0_0000, 0x7FC0_0001, 0xFFC0_0001, 0x7FFF_FFFF, 0xFFFF_FFFF,
            0x7F80_0001, 0xFF80_0001, 0x7FA5_A5A5, 0xFFA5_A5A5,
        ];
        f32::from_bits(NANS[self.below(NANS.len() as u32) as usize])
    }

    /// Either a NaN with a random payload, a signed zero, or an infinity —
    /// the three classes that make `0*inf` / NaN-vs-NaN precedence observable.
    pub fn nan_zero_inf_f32(&mut self) -> f32 {
        match self.below(4) {
            0 => 0.0,
            1 => -0.0,
            2 => {
                if self.bool() {
                    f32::INFINITY
                } else {
                    f32::NEG_INFINITY
                }
            }
            _ => self.nan_f32(),
        }
    }

    pub fn nan_v(&mut self) -> C2v {
        C2v {
            x: self.nan_zero_inf_f32(),
            y: self.nan_zero_inf_f32(),
        }
    }

    pub fn mixed_v(&mut self) -> C2v {
        C2v {
            x: self.mixed_f32(),
            y: self.mixed_f32(),
        }
    }

    pub fn mixed_r(&mut self) -> C2r {
        C2r {
            c: self.mixed_f32(),
            s: self.mixed_f32(),
        }
    }

    pub fn mixed_x(&mut self) -> C2x {
        C2x {
            p: self.mixed_v(),
            r: self.mixed_r(),
        }
    }

    pub fn unit_r(&mut self) -> C2r {
        let th = self.range(-7.0, 7.0);
        C2r {
            c: th.cos(),
            s: th.sin(),
        }
    }

    /// Ray with a normalized-ish direction and a positive length.
    pub fn normal_ray(&mut self) -> C2Ray {
        let th = self.range(-7.0, 7.0);
        C2Ray {
            p: C2v {
                x: self.range(-30.0, 30.0),
                y: self.range(-30.0, 30.0),
            },
            d: C2v {
                x: th.cos(),
                y: th.sin(),
            },
            t: self.range(0.0, 60.0),
        }
    }

    pub fn axis_ray(&mut self, axis: u32) -> C2Ray {
        let d = match axis & 3 {
            0 => C2v { x: 1.0, y: 0.0 },
            1 => C2v { x: -1.0, y: 0.0 },
            2 => C2v { x: 0.0, y: 1.0 },
            _ => C2v { x: 0.0, y: -1.0 },
        };
        C2Ray {
            p: C2v {
                x: self.range(-30.0, 30.0),
                y: self.range(-30.0, 30.0),
            },
            d,
            t: self.range(0.0, 60.0),
        }
    }

    pub fn mixed_ray(&mut self) -> C2Ray {
        C2Ray {
            p: self.mixed_v(),
            d: self.mixed_v(),
            t: self.mixed_f32(),
        }
    }

    /// One of the interesting `t` values from the CONFIGS `A.t` sweep axis.
    pub fn t_sweep(&mut self, i: u32) -> f32 {
        match i % 8 {
            0 => 0.0,
            1 => -0.0,
            2 => 1e-7,
            3 => 1.0,
            4 => self.range(0.0, 100.0),
            5 => 1e30,
            6 => f32::INFINITY,
            _ => -self.range(0.0, 100.0),
        }
    }

    pub fn normal_aabb(&mut self) -> C2AABB {
        let cx = self.range(-30.0, 30.0);
        let cy = self.range(-30.0, 30.0);
        let hx = self.range(0.0, 15.0);
        let hy = self.range(0.0, 15.0);
        C2AABB {
            min: C2v {
                x: cx - hx,
                y: cy - hy,
            },
            max: C2v {
                x: cx + hx,
                y: cy + hy,
            },
        }
    }

    pub fn mixed_aabb(&mut self) -> C2AABB {
        C2AABB {
            min: self.mixed_v(),
            max: self.mixed_v(),
        }
    }

    pub fn normal_circle(&mut self) -> C2Circle {
        C2Circle {
            p: C2v {
                x: self.range(-30.0, 30.0),
                y: self.range(-30.0, 30.0),
            },
            r: self.range(0.0, 15.0),
        }
    }

    pub fn mixed_circle(&mut self) -> C2Circle {
        C2Circle {
            p: self.mixed_v(),
            r: self.mixed_f32(),
        }
    }

    pub fn normal_capsule(&mut self) -> C2Capsule {
        C2Capsule {
            a: C2v {
                x: self.range(-30.0, 30.0),
                y: self.range(-30.0, 30.0),
            },
            b: C2v {
                x: self.range(-30.0, 30.0),
                y: self.range(-30.0, 30.0),
            },
            r: self.range(0.0, 10.0),
        }
    }

    pub fn mixed_capsule(&mut self) -> C2Capsule {
        C2Capsule {
            a: self.mixed_v(),
            b: self.mixed_v(),
            r: self.mixed_f32(),
        }
    }
}

// ---------------------------------------------------------------------------
// Polygon builders
// ---------------------------------------------------------------------------

/// Convex regular n-gon centred at `c` with circumradius `rad`, wound so that
/// `norms[i]` is the outward normal of the plane through `verts[i]`.
pub fn ngon(n: usize, c: C2v, rad: f32, phase: f32) -> C2Poly {
    assert!((1..=8).contains(&n));
    let mut p = C2Poly::default();
    p.count = n as c_int;
    for i in 0..n {
        let a = phase + std::f32::consts::TAU * (i as f32) / (n as f32);
        p.verts[i] = C2v {
            x: c.x + rad * a.cos(),
            y: c.y + rad * a.sin(),
        };
    }
    for i in 0..n {
        let j = (i + 1) % n;
        let e = C2v {
            x: p.verts[j].x - p.verts[i].x,
            y: p.verts[j].y - p.verts[i].y,
        };
        // outward normal for CCW winding is (e.y, -e.x), normalized
        let l = (e.x * e.x + e.y * e.y).sqrt();
        p.norms[i] = C2v {
            x: e.y / l,
            y: -e.x / l,
        };
    }
    p
}

/// The exact polygon `poly_ray` builds.
pub fn poly_ray_poly() -> C2Poly {
    let mut p = C2Poly::default();
    p.verts[0] = C2v { x: 0.875, y: -11.5 };
    p.verts[1] = C2v { x: 0.875, y: 11.5 };
    p.verts[2] = C2v { x: -0.875, y: 11.5 };
    p.verts[3] = C2v { x: -0.875, y: -11.5 };
    p.norms[0] = C2v { x: 1.0, y: 0.0 };
    p.norms[1] = C2v { x: 0.0, y: 1.0 };
    p.norms[2] = C2v { x: -1.0, y: 0.0 };
    p.norms[3] = C2v { x: 0.0, y: -1.0 };
    p.count = 4;
    p
}

pub fn axis_box_poly(min: C2v, max: C2v) -> C2Poly {
    let mut p = C2Poly::default();
    p.count = 4;
    p.verts[0] = C2v { x: max.x, y: min.y };
    p.verts[1] = C2v { x: max.x, y: max.y };
    p.verts[2] = C2v { x: min.x, y: max.y };
    p.verts[3] = C2v { x: min.x, y: min.y };
    p.norms[0] = C2v { x: 1.0, y: 0.0 };
    p.norms[1] = C2v { x: 0.0, y: 1.0 };
    p.norms[2] = C2v { x: -1.0, y: 0.0 };
    p.norms[3] = C2v { x: 0.0, y: -1.0 };
    p
}

// ---------------------------------------------------------------------------
// Assertion helper: reports the CONFIGS/ERRORS row on failure.
// ---------------------------------------------------------------------------

#[macro_export]
macro_rules! diff_eq {
    ($row:expr, $iter:expr, $c:expr, $r:expr, $ctx:expr) => {{
        let cv = $c;
        let rv = $r;
        if cv != rv {
            panic!(
                "row {} iter {}: DIVERGENCE\n  C    = {:?}\n  Rust = {:?}\n  input: {}",
                $row, $iter, cv, rv, $ctx
            );
        }
    }};
}
