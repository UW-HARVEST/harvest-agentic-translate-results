//! Shared differential-test harness.
//!
//! Loads BOTH shared objects (the C one built by CMake and the Rust `cdylib`)
//! through `libloading` and exposes every exported symbol as a pair of function
//! pointers.  Nothing in the Rust crate is ever called directly, so the
//! `#[no_mangle]` / `extern "C"` wrappers and the struct ABI are under test too.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// C-layout types (mirrors of the ones in c_src/src/lib.c)
// ---------------------------------------------------------------------------

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

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
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

// ---------------------------------------------------------------------------
// Function-pointer table
// ---------------------------------------------------------------------------

pub type FnV = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnMulvs = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnVV2V = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnVVV2V = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnVV2F = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnV2F = unsafe extern "C" fn(c2v) -> f32;
pub type FnV2V = unsafe extern "C" fn(c2v) -> c2v;
pub type FnRotId = unsafe extern "C" fn() -> c2r;
pub type FnXId = unsafe extern "C" fn() -> c2x;
pub type FnBBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
pub type FnSimplex2F = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type FnSimplex2V = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type FnSimplex = unsafe extern "C" fn(*mut c2Simplex);
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
    _lib: Library,
    pub name: &'static str,
    pub c2V: FnV,
    pub c2Mulvs: FnMulvs,
    pub c2Maxv: FnVV2V,
    pub c2Minv: FnVV2V,
    pub c2Clampv: FnVVV2V,
    pub c2Sub: FnVV2V,
    pub c2Dot: FnVV2F,
    pub c2RotIdentity: FnRotId,
    pub c2xIdentity: FnXId,
    pub c2BBVerts: FnBBVerts,
    pub c2MakeProxy: FnMakeProxy,
    pub c2Len: FnV2F,
    pub c2Det2: FnVV2F,
    pub c2GJKSimplexMetric: FnSimplex2F,
    pub c2Mulrv: FnRV,
    pub c2Add: FnVV2V,
    pub c2Mulxv: FnXV,
    pub c22: FnSimplex,
    pub c23: FnSimplex,
    pub c2Neg: FnV2V,
    pub c2Skew: FnV2V,
    pub c2CCW90: FnV2V,
    pub c2D: FnSimplex2V,
    pub c2Support: FnSupport,
    pub c2Witness: FnWitness,
    pub c2Div: FnDiv,
    pub c2Norm: FnV2V,
    pub c2L: FnSimplex2V,
    pub c2MulrvT: FnRV,
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

unsafe fn sym<T: Copy>(lib: &Library, n: &[u8]) -> T {
    let s: Symbol<T> = lib
        .get(n)
        .unwrap_or_else(|e| panic!("symbol {:?} not found: {e}", String::from_utf8_lossy(n)));
    *s
}

impl Api {
    pub fn load(path: &PathBuf, name: &'static str) -> Api {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display()));
            Api {
                name,
                c2V: sym(&lib, b"c2V\0"),
                c2Mulvs: sym(&lib, b"c2Mulvs\0"),
                c2Maxv: sym(&lib, b"c2Maxv\0"),
                c2Minv: sym(&lib, b"c2Minv\0"),
                c2Clampv: sym(&lib, b"c2Clampv\0"),
                c2Sub: sym(&lib, b"c2Sub\0"),
                c2Dot: sym(&lib, b"c2Dot\0"),
                c2RotIdentity: sym(&lib, b"c2RotIdentity\0"),
                c2xIdentity: sym(&lib, b"c2xIdentity\0"),
                c2BBVerts: sym(&lib, b"c2BBVerts\0"),
                c2MakeProxy: sym(&lib, b"c2MakeProxy\0"),
                c2Len: sym(&lib, b"c2Len\0"),
                c2Det2: sym(&lib, b"c2Det2\0"),
                c2GJKSimplexMetric: sym(&lib, b"c2GJKSimplexMetric\0"),
                c2Mulrv: sym(&lib, b"c2Mulrv\0"),
                c2Add: sym(&lib, b"c2Add\0"),
                c2Mulxv: sym(&lib, b"c2Mulxv\0"),
                c22: sym(&lib, b"c22\0"),
                c23: sym(&lib, b"c23\0"),
                c2Neg: sym(&lib, b"c2Neg\0"),
                c2Skew: sym(&lib, b"c2Skew\0"),
                c2CCW90: sym(&lib, b"c2CCW90\0"),
                c2D: sym(&lib, b"c2D\0"),
                c2Support: sym(&lib, b"c2Support\0"),
                c2Witness: sym(&lib, b"c2Witness\0"),
                c2Div: sym(&lib, b"c2Div\0"),
                c2Norm: sym(&lib, b"c2Norm\0"),
                c2L: sym(&lib, b"c2L\0"),
                c2MulrvT: sym(&lib, b"c2MulrvT\0"),
                c2GJK: sym(&lib, b"c2GJK\0"),
                c2AABBtoAABB: sym(&lib, b"c2AABBtoAABB\0"),
                c2AABBtoCapsule: sym(&lib, b"c2AABBtoCapsule\0"),
                c2CapsuletoCapsule: sym(&lib, b"c2CapsuletoCapsule\0"),
                c2CircletoCircle: sym(&lib, b"c2CircletoCircle\0"),
                c2CircletoAABB: sym(&lib, b"c2CircletoAABB\0"),
                c2CircletoCapsule: sym(&lib, b"c2CircletoCapsule\0"),
                c2Collided: sym(&lib, b"c2Collided\0"),
                capsule: sym(&lib, b"capsule\0"),
                _lib: lib,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let n = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if n.starts_with("lib") && n.ends_with(".so") {
                cands.push(p);
            }
        }
    }
    cands.sort();
    cands.pop().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    // Prefer the profile the tests were built with, then the other one.
    let mut cands = vec![
        target.join("release").join("libcapsule_lib.so"),
        target.join("debug").join("libcapsule_lib.so"),
    ];
    if let Ok(d) = std::env::var("CAPSULE_RUST_SO") {
        cands.insert(0, PathBuf::from(d));
    }
    for c in &cands {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "no Rust cdylib found; tried {:?}. Build it with `cargo build --release`.",
        cands
    );
}

pub struct Pair {
    pub c: Api,
    pub r: Api,
}

/// Loads both libraries once per test binary.
pub fn apis() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair {
        c: Api::load(&find_c_so(), "C"),
        r: Api::load(&find_rust_so(), "Rust"),
    })
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub fn bits(x: f32) -> u32 {
    x.to_bits()
}

#[track_caller]
pub fn eq_f32(what: &str, ctx: &dyn std::fmt::Debug, c: f32, r: f32) {
    assert_eq!(
        c.to_bits(),
        r.to_bits(),
        "{what}: C={c:?} (0x{:08x}) != Rust={r:?} (0x{:08x})  input={ctx:?}",
        c.to_bits(),
        r.to_bits()
    );
}

#[track_caller]
pub fn eq_v(what: &str, ctx: &dyn std::fmt::Debug, c: c2v, r: c2v) {
    if c.x.to_bits() != r.x.to_bits() || c.y.to_bits() != r.y.to_bits() {
        panic!(
            "{what}: C=({:?},{:?}) [0x{:08x},0x{:08x}] != Rust=({:?},{:?}) [0x{:08x},0x{:08x}]  input={ctx:?}",
            c.x, c.y, c.x.to_bits(), c.y.to_bits(),
            r.x, r.y, r.x.to_bits(), r.y.to_bits()
        );
    }
}

#[track_caller]
pub fn eq_r(what: &str, ctx: &dyn std::fmt::Debug, c: c2r, r: c2r) {
    assert!(
        c.c.to_bits() == r.c.to_bits() && c.s.to_bits() == r.s.to_bits(),
        "{what}: C={c:?} != Rust={r:?} input={ctx:?}"
    );
}

#[track_caller]
pub fn eq_x(what: &str, ctx: &dyn std::fmt::Debug, c: c2x, r: c2x) {
    eq_v(&format!("{what}.p"), ctx, c.p, r.p);
    eq_r(&format!("{what}.r"), ctx, c.r, r.r);
}

#[track_caller]
pub fn eq_int(what: &str, ctx: &dyn std::fmt::Debug, c: c_int, r: c_int) {
    assert_eq!(c, r, "{what}: C={c} != Rust={r}  input={ctx:?}");
}

/// Raw byte comparison — the strongest possible check for out-params.
#[track_caller]
pub fn eq_bytes<T: Copy>(what: &str, ctx: &dyn std::fmt::Debug, c: &T, r: &T) {
    let n = std::mem::size_of::<T>();
    let cb = unsafe { std::slice::from_raw_parts(c as *const T as *const u8, n) };
    let rb = unsafe { std::slice::from_raw_parts(r as *const T as *const u8, n) };
    if cb != rb {
        let first = cb.iter().zip(rb).position(|(a, b)| a != b).unwrap();
        panic!(
            "{what}: {n}-byte struct differs at byte {first}\n  C   ={cb:02x?}\n  Rust={rb:02x?}\n  input={ctx:?}"
        );
    }
}

#[track_caller]
pub fn eq_simplex(what: &str, ctx: &dyn std::fmt::Debug, c: &c2Simplex, r: &c2Simplex) {
    eq_bytes(what, ctx, c, r);
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) + float generators
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    pub fn bool(&mut self) -> bool {
        self.next_u32() & 1 == 1
    }
    /// Uniform in `[lo, hi)`.
    pub fn f32_in(&mut self, lo: f32, hi: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        lo + u * (hi - lo)
    }
    /// A "geometry-ish" coordinate: mostly small, sometimes tiny/huge/zero.
    pub fn coord(&mut self) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => self.f32_in(-1.0e-30, 1.0e-30),
            3 => self.f32_in(-1.0e18, 1.0e18),
            4 => (self.next_u32() % 21) as f32 - 10.0, // exact small integers
            _ => self.f32_in(-120.0, 120.0),
        }
    }
    /// A radius: non-negative most of the time, occasionally 0 or negative.
    pub fn radius(&mut self) -> f32 {
        match self.below(12) {
            0 => 0.0,
            1 => -self.f32_in(0.0, 30.0),
            2 => self.f32_in(0.0, 1.0e18),
            _ => self.f32_in(0.0, 40.0),
        }
    }
    /// Any float, including NaN payloads, infinities and denormals.
    pub fn any_f32(&mut self) -> f32 {
        match self.below(8) {
            0 => f32::from_bits(self.next_u32()),
            1 => *pick(SPECIAL_F32, self),
            _ => self.coord(),
        }
    }
    pub fn v(&mut self) -> c2v {
        c2v {
            x: self.coord(),
            y: self.coord(),
        }
    }
    pub fn any_v(&mut self) -> c2v {
        c2v {
            x: self.any_f32(),
            y: self.any_f32(),
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
        if self.below(8) == 0 {
            // sometimes leave it inverted / degenerate on purpose
            c2AABB { min: a, max: b }
        } else {
            c2AABB {
                min: c2v {
                    x: a.x.min(b.x),
                    y: a.y.min(b.y),
                },
                max: c2v {
                    x: a.x.max(b.x),
                    y: a.y.max(b.y),
                },
            }
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
    pub fn rot(&mut self) -> c2r {
        match self.below(8) {
            0 => c2r { c: 1.0, s: 0.0 },
            1 => c2r { c: 0.0, s: 1.0 },
            2 => c2r { c: 0.0, s: -1.0 },
            3 => c2r {
                // deliberately NOT normalised: the C never normalises either
                c: self.f32_in(-4.0, 4.0),
                s: self.f32_in(-4.0, 4.0),
            },
            _ => {
                let t = self.f32_in(-3.14159265, 3.14159265);
                c2r {
                    c: t.cos(),
                    s: t.sin(),
                }
            }
        }
    }
    pub fn xform(&mut self) -> c2x {
        c2x {
            p: self.v(),
            r: self.rot(),
        }
    }
    pub fn sv(&mut self) -> c2sv {
        c2sv {
            sA: self.v(),
            sB: self.v(),
            p: self.v(),
            u: self.f32_in(-4.0, 4.0),
            iA: self.below(8) as c_int,
            iB: self.below(8) as c_int,
        }
    }
    pub fn simplex(&mut self, count: c_int) -> c2Simplex {
        c2Simplex {
            verts: [self.sv(), self.sv(), self.sv(), self.sv()],
            div: match self.below(10) {
                0 => 0.0,
                1 => 1.0,
                2 => self.f32_in(-1.0e-20, 1.0e-20),
                _ => self.f32_in(-8.0, 8.0),
            },
            count,
        }
    }
}

pub const SPECIAL_F32: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    f32::from_bits(1),  // smallest denormal
    f32::from_bits(0x8000_0001),
    f32::EPSILON,
    -f32::EPSILON,
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    f32::from_bits(0x7f80_0001), // signalling NaN
    f32::from_bits(0xff80_0001), // negative signalling NaN
    f32::from_bits(0x7fc0_1234), // quiet NaN with payload
    f32::from_bits(0xffc0_1234),
    1.0e-30,
    1.0e30,
    -1.0e30,
    16777216.0,  // 2^24
    16777217.0,  // not representable -> rounds
    1.1920929e-7,
];

pub fn pick<'a, T>(s: &'a [T], rng: &mut Rng) -> &'a T {
    &s[(rng.next_u32() as usize) % s.len()]
}

/// Number of random cases per configuration row (override with `CAPSULE_N`).
pub fn n_cases(default: usize) -> usize {
    std::env::var("CAPSULE_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
