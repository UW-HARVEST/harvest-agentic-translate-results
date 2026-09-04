//! Differential-test harness: loads BOTH the C `.so` and the Rust `.so` with
//! `libloading` and exposes every exported symbol as a pair of function
//! pointers.  Nothing is ever called directly from the Rust crate — every call
//! goes through the dynamic symbol table, exactly like an external consumer.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// C-compatible POD types (must mirror c_src/src/lib.c exactly)
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
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

/// `typedef struct { c2sv a, b, c, d; float div; int count; } c2Simplex;`
#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2Simplex {
    pub a: c2sv,
    pub b: c2sv,
    pub c: c2sv,
    pub d: c2sv,
    pub div: f32,
    pub count: c_int,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

// ---------------------------------------------------------------------------
// Function-pointer table
// ---------------------------------------------------------------------------

pub type FnV = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnMulvs = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnVV_V = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnVVV_V = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnVV_F = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnV_V = unsafe extern "C" fn(c2v) -> c2v;
pub type FnV_F = unsafe extern "C" fn(c2v) -> f32;
pub type FnRotId = unsafe extern "C" fn() -> c2r;
pub type FnXId = unsafe extern "C" fn() -> c2x;
pub type FnBBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
pub type FnSimplexF = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type FnSimplexV = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type FnSimplexVoid = unsafe extern "C" fn(*mut c2Simplex);
pub type FnRV = unsafe extern "C" fn(c2r, c2v) -> c2v;
pub type FnXV = unsafe extern "C" fn(c2x, c2v) -> c2v;
pub type FnSupport = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
pub type FnWitness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
pub type FnDiv = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnGJK = unsafe extern "C" fn(
    *const c_void,
    c_int,
    *const c2x,
    *const c_void,
    c_int,
    *const c2x,
    *mut c2v,
    *mut c2v,
    c_int,
    *mut c_int,
    *mut c2GJKCache,
) -> f32;
pub type FnAABBtoAABB = unsafe extern "C" fn(c2AABB, c2AABB) -> c_int;
pub type FnAABBtoCapsule = unsafe extern "C" fn(c2AABB, c2Capsule) -> c_int;
pub type FnCapsuletoCapsule = unsafe extern "C" fn(c2Capsule, c2Capsule) -> c_int;
pub type FnCircletoCircle = unsafe extern "C" fn(c2Circle, c2Circle) -> c_int;
pub type FnCircletoAABB = unsafe extern "C" fn(c2Circle, c2AABB) -> c_int;
pub type FnCircletoCapsule = unsafe extern "C" fn(c2Circle, c2Capsule) -> c_int;
pub type FnCollided = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;
pub type FnCapsule = unsafe extern "C" fn(f32, f32, f32, f32, f32) -> c_int;

pub struct Api {
    pub name: &'static str,
    _lib: Library,
    pub c2V: FnV,
    pub c2Mulvs: FnMulvs,
    pub c2Maxv: FnVV_V,
    pub c2Minv: FnVV_V,
    pub c2Clampv: FnVVV_V,
    pub c2Sub: FnVV_V,
    pub c2Add: FnVV_V,
    pub c2Dot: FnVV_F,
    pub c2Det2: FnVV_F,
    pub c2Len: FnV_F,
    pub c2RotIdentity: FnRotId,
    pub c2xIdentity: FnXId,
    pub c2BBVerts: FnBBVerts,
    pub c2MakeProxy: FnMakeProxy,
    pub c2GJKSimplexMetric: FnSimplexF,
    pub c2Mulrv: FnRV,
    pub c2MulrvT: FnRV,
    pub c2Mulxv: FnXV,
    pub c22: FnSimplexVoid,
    pub c23: FnSimplexVoid,
    pub c2Neg: FnV_V,
    pub c2Skew: FnV_V,
    pub c2CCW90: FnV_V,
    pub c2D: FnSimplexV,
    pub c2L: FnSimplexV,
    pub c2Support: FnSupport,
    pub c2Witness: FnWitness,
    pub c2Div: FnDiv,
    pub c2Norm: FnV_V,
    pub c2GJK: FnGJK,
    pub c2AABBtoAABB: FnAABBtoAABB,
    pub c2AABBtoCapsule: FnAABBtoCapsule,
    pub c2CapsuletoCapsule: FnCapsuletoCapsule,
    pub c2CircletoCircle: FnCircletoCircle,
    pub c2CircletoAABB: FnCircletoAABB,
    pub c2CircletoCapsule: FnCircletoCapsule,
    pub c2Collided: FnCollided,
    pub capsule: FnCapsule,
}

macro_rules! sym {
    ($lib:expr, $name:literal, $ty:ty) => {{
        let s: Symbol<$ty> = unsafe {
            $lib.get(concat!($name, "\0").as_bytes())
                .unwrap_or_else(|e| panic!("missing symbol {}: {}", $name, e))
        };
        *s
    }};
}

impl Api {
    fn open(name: &'static str, path: &PathBuf) -> Api {
        let lib = unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("cannot load {}: {}", path.display(), e))
        };
        let api = Api {
            name,
            c2V: sym!(lib, "c2V", FnV),
            c2Mulvs: sym!(lib, "c2Mulvs", FnMulvs),
            c2Maxv: sym!(lib, "c2Maxv", FnVV_V),
            c2Minv: sym!(lib, "c2Minv", FnVV_V),
            c2Clampv: sym!(lib, "c2Clampv", FnVVV_V),
            c2Sub: sym!(lib, "c2Sub", FnVV_V),
            c2Add: sym!(lib, "c2Add", FnVV_V),
            c2Dot: sym!(lib, "c2Dot", FnVV_F),
            c2Det2: sym!(lib, "c2Det2", FnVV_F),
            c2Len: sym!(lib, "c2Len", FnV_F),
            c2RotIdentity: sym!(lib, "c2RotIdentity", FnRotId),
            c2xIdentity: sym!(lib, "c2xIdentity", FnXId),
            c2BBVerts: sym!(lib, "c2BBVerts", FnBBVerts),
            c2MakeProxy: sym!(lib, "c2MakeProxy", FnMakeProxy),
            c2GJKSimplexMetric: sym!(lib, "c2GJKSimplexMetric", FnSimplexF),
            c2Mulrv: sym!(lib, "c2Mulrv", FnRV),
            c2MulrvT: sym!(lib, "c2MulrvT", FnRV),
            c2Mulxv: sym!(lib, "c2Mulxv", FnXV),
            c22: sym!(lib, "c22", FnSimplexVoid),
            c23: sym!(lib, "c23", FnSimplexVoid),
            c2Neg: sym!(lib, "c2Neg", FnV_V),
            c2Skew: sym!(lib, "c2Skew", FnV_V),
            c2CCW90: sym!(lib, "c2CCW90", FnV_V),
            c2D: sym!(lib, "c2D", FnSimplexV),
            c2L: sym!(lib, "c2L", FnSimplexV),
            c2Support: sym!(lib, "c2Support", FnSupport),
            c2Witness: sym!(lib, "c2Witness", FnWitness),
            c2Div: sym!(lib, "c2Div", FnDiv),
            c2Norm: sym!(lib, "c2Norm", FnV_V),
            c2GJK: sym!(lib, "c2GJK", FnGJK),
            c2AABBtoAABB: sym!(lib, "c2AABBtoAABB", FnAABBtoAABB),
            c2AABBtoCapsule: sym!(lib, "c2AABBtoCapsule", FnAABBtoCapsule),
            c2CapsuletoCapsule: sym!(lib, "c2CapsuletoCapsule", FnCapsuletoCapsule),
            c2CircletoCircle: sym!(lib, "c2CircletoCircle", FnCircletoCircle),
            c2CircletoAABB: sym!(lib, "c2CircletoAABB", FnCircletoAABB),
            c2CircletoCapsule: sym!(lib, "c2CircletoCapsule", FnCircletoCapsule),
            c2Collided: sym!(lib, "c2Collided", FnCollided),
            capsule: sym!(lib, "capsule", FnCapsule),
            _lib: lib,
        };
        api
    }
}

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    // Allows re-running the whole suite against a differently-compiled C build
    // (e.g. -O2) without touching c_src/.
    if let Ok(p) = std::env::var("C2_C_SO") {
        return PathBuf::from(p);
    }
    let dir = crate_root().parent().unwrap().join("c_src").join("build");
    let mut best: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                best = Some(p);
                break;
            }
        }
    }
    best.unwrap_or_else(|| {
        panic!(
            "no C .so found in {} -- build it with cmake first",
            dir.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // current_exe() is target/<profile>/deps/<testbin>
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe.parent().unwrap().parent().unwrap().to_path_buf();
    let direct = profile_dir.join("libcapsule_lib.so");
    if direct.exists() {
        return direct;
    }
    for cand in ["release", "debug"] {
        let p = crate_root().join("target").join(cand).join("libcapsule_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libcapsule_lib.so not found (looked in {})",
        profile_dir.display()
    );
}

pub struct Pair {
    pub c: Api,
    pub r: Api,
}

/// Load both libraries. Leaked so the returned `&'static Pair` is usable from
/// every test without lifetime plumbing.
pub fn libs() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair {
        c: Api::open("C", &find_c_so()),
        r: Api::open("Rust", &find_rust_so()),
    })
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub fn bits(x: f32) -> u32 {
    x.to_bits()
}

#[track_caller]
pub fn eq_f32(ctx: &str, c: f32, r: f32) {
    if c.to_bits() != r.to_bits() {
        panic!(
            "{ctx}: f32 mismatch\n  C    = {c:?} (0x{:08x})\n  Rust = {r:?} (0x{:08x})",
            c.to_bits(),
            r.to_bits()
        );
    }
}

#[track_caller]
pub fn eq_v(ctx: &str, c: c2v, r: c2v) {
    if c.x.to_bits() != r.x.to_bits() || c.y.to_bits() != r.y.to_bits() {
        panic!(
            "{ctx}: c2v mismatch\n  C    = ({:?},{:?}) (0x{:08x},0x{:08x})\n  Rust = ({:?},{:?}) (0x{:08x},0x{:08x})",
            c.x, c.y, c.x.to_bits(), c.y.to_bits(),
            r.x, r.y, r.x.to_bits(), r.y.to_bits()
        );
    }
}

#[track_caller]
pub fn eq_r(ctx: &str, c: c2r, r: c2r) {
    eq_f32(&format!("{ctx}.c"), c.c, r.c);
    eq_f32(&format!("{ctx}.s"), c.s, r.s);
}

#[track_caller]
pub fn eq_x(ctx: &str, c: c2x, r: c2x) {
    eq_v(&format!("{ctx}.p"), c.p, r.p);
    eq_r(&format!("{ctx}.r"), c.r, r.r);
}

#[track_caller]
pub fn eq_i(ctx: &str, c: c_int, r: c_int) {
    assert_eq!(c, r, "{ctx}: int mismatch (C={c}, Rust={r})");
}

#[track_caller]
pub fn eq_sv(ctx: &str, c: &c2sv, r: &c2sv) {
    eq_v(&format!("{ctx}.sA"), c.sA, r.sA);
    eq_v(&format!("{ctx}.sB"), c.sB, r.sB);
    eq_v(&format!("{ctx}.p"), c.p, r.p);
    eq_f32(&format!("{ctx}.u"), c.u, r.u);
    eq_i(&format!("{ctx}.iA"), c.iA, r.iA);
    eq_i(&format!("{ctx}.iB"), c.iB, r.iB);
}

#[track_caller]
pub fn eq_simplex(ctx: &str, c: &c2Simplex, r: &c2Simplex) {
    eq_sv(&format!("{ctx}.a"), &c.a, &r.a);
    eq_sv(&format!("{ctx}.b"), &c.b, &r.b);
    eq_sv(&format!("{ctx}.c"), &c.c, &r.c);
    eq_sv(&format!("{ctx}.d"), &c.d, &r.d);
    eq_f32(&format!("{ctx}.div"), c.div, r.div);
    eq_i(&format!("{ctx}.count"), c.count, r.count);
}

#[track_caller]
pub fn eq_proxy(ctx: &str, c: &c2Proxy, r: &c2Proxy) {
    eq_f32(&format!("{ctx}.radius"), c.radius, r.radius);
    eq_i(&format!("{ctx}.count"), c.count, r.count);
    for i in 0..8 {
        eq_v(&format!("{ctx}.verts[{i}]"), c.verts[i], r.verts[i]);
    }
}

#[track_caller]
pub fn eq_cache(ctx: &str, c: &c2GJKCache, r: &c2GJKCache) {
    eq_f32(&format!("{ctx}.metric"), c.metric, r.metric);
    eq_i(&format!("{ctx}.count"), c.count, r.count);
    for i in 0..3 {
        eq_i(&format!("{ctx}.iA[{i}]"), c.iA[i], r.iA[i]);
        eq_i(&format!("{ctx}.iB[{i}]"), c.iB[i], r.iB[i]);
    }
    eq_f32(&format!("{ctx}.div"), c.div, r.div);
}

// ---------------------------------------------------------------------------
// Deterministic RNG (PCG32) + float generators
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        let mut r = Rng(0);
        r.0 = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        r.next_u32();
        r
    }
    pub fn next_u32(&mut self) -> u32 {
        // splitmix64 -> high 32 bits
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 32) as u32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Uniform in [0,1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in [-a, a].
    pub fn range(&mut self, a: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * a
    }
    /// "Ordinary" coordinate: mostly small, sometimes large.
    pub fn coord(&mut self) -> f32 {
        match self.below(10) {
            0..=5 => self.range(100.0),
            6 => self.range(1.0),
            7 => self.range(1.0e6),
            8 => (self.below(9) as f32) - 4.0, // small integers, produces ties
            _ => self.range(1.0e-4),
        }
    }
    /// Radius: mostly non-negative, sometimes degenerate/negative.
    pub fn radius(&mut self) -> f32 {
        match self.below(10) {
            0..=6 => self.unit() * 50.0,
            7 => 0.0,
            8 => -self.unit() * 20.0,
            _ => self.unit() * 1.0e5,
        }
    }
    /// The full float zoo, including specials.
    pub fn wild(&mut self) -> f32 {
        match self.below(16) {
            0..=6 => self.coord(),
            7 => 0.0,
            8 => -0.0,
            9 => f32::INFINITY,
            10 => f32::NEG_INFINITY,
            11 => f32::NAN,
            12 => f32::from_bits(0xFFC0_0000), // negative quiet NaN
            13 => f32::MAX,
            14 => f32::MIN_POSITIVE / 3.0, // denormal
            _ => f32::from_bits(self.next_u32()),
        }
    }
    pub fn v(&mut self) -> c2v {
        c2v {
            x: self.coord(),
            y: self.coord(),
        }
    }
    pub fn wild_v(&mut self) -> c2v {
        c2v {
            x: self.wild(),
            y: self.wild(),
        }
    }
    pub fn rot(&mut self) -> c2r {
        match self.below(6) {
            0 => c2r { c: 1.0, s: 0.0 },
            1 => c2r { c: 0.0, s: 0.0 },
            2 => c2r {
                c: self.range(3.0),
                s: self.range(3.0),
            },
            _ => {
                let t = self.unit() * std::f32::consts::TAU;
                c2r {
                    c: t.cos(),
                    s: t.sin(),
                }
            }
        }
    }
    pub fn x(&mut self) -> c2x {
        c2x {
            p: self.v(),
            r: self.rot(),
        }
    }
    pub fn circle(&mut self) -> c2Circle {
        c2Circle {
            p: self.v(),
            r: self.radius(),
        }
    }
    pub fn aabb(&mut self) -> c2AABB {
        let a = self.v();
        let b = self.v();
        match self.below(8) {
            0 => c2AABB { min: a, max: a },     // zero area
            1 => c2AABB { min: b, max: a },     // possibly inverted
            _ => c2AABB {
                min: c2v {
                    x: a.x.min(b.x),
                    y: a.y.min(b.y),
                },
                max: c2v {
                    x: a.x.max(b.x),
                    y: a.y.max(b.y),
                },
            },
        }
    }
    pub fn capsule(&mut self) -> c2Capsule {
        let a = self.v();
        let b = if self.below(8) == 0 { a } else { self.v() };
        c2Capsule {
            a,
            b,
            r: self.radius(),
        }
    }
    pub fn sv(&mut self) -> c2sv {
        c2sv {
            sA: self.v(),
            sB: self.v(),
            p: self.v(),
            u: self.coord(),
            iA: self.below(8) as c_int,
            iB: self.below(8) as c_int,
        }
    }
}

/// A `c2Simplex` with every field randomised (so field-copy bugs surface),
/// then `count`/`div` set explicitly.
pub fn rand_simplex(rng: &mut Rng, count: c_int, div: f32) -> c2Simplex {
    c2Simplex {
        a: rng.sv(),
        b: rng.sv(),
        c: rng.sv(),
        d: rng.sv(),
        div,
        count,
    }
}
