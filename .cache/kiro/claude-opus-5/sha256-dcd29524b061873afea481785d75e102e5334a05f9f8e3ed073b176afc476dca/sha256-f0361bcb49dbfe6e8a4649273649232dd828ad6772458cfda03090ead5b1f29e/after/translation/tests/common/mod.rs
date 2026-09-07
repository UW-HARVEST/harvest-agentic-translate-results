//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and exposes every exported symbol as a raw `extern "C"` fn
//! pointer. Nothing in the crate is ever called directly, so the
//! `#[no_mangle]` export wrappers are part of what is under test.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// FFI type mirrors (layouts taken from c_src/src/lib.c + c_src/include/lib.h)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
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
#[derive(Copy, Clone, Debug, Default)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

impl Default for c2Proxy {
    fn default() -> Self {
        c2Proxy {
            radius: 0.0,
            count: 0,
            verts: [c2v::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

/// C: `typedef struct { c2sv a, b, c, d; float div; int count; } c2Simplex;`
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

pub const ALL_TYPES: [c_int; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

// ---------------------------------------------------------------------------
// Function pointer table
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: &'static str,
    pub c2V: extern "C" fn(f32, f32) -> c2v,
    pub c2Mulvs: extern "C" fn(c2v, f32) -> c2v,
    pub c2Maxv: extern "C" fn(c2v, c2v) -> c2v,
    pub c2Minv: extern "C" fn(c2v, c2v) -> c2v,
    pub c2Clampv: extern "C" fn(c2v, c2v, c2v) -> c2v,
    pub c2Sub: extern "C" fn(c2v, c2v) -> c2v,
    pub c2Dot: extern "C" fn(c2v, c2v) -> f32,
    pub c2RotIdentity: extern "C" fn() -> c2r,
    pub c2xIdentity: extern "C" fn() -> c2x,
    pub c2BBVerts: unsafe extern "C" fn(*mut c2v, *mut c2AABB),
    pub c2MakeProxy: unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy),
    pub c2Len: extern "C" fn(c2v) -> f32,
    pub c2Det2: extern "C" fn(c2v, c2v) -> f32,
    pub c2GJKSimplexMetric: unsafe extern "C" fn(*mut c2Simplex) -> f32,
    pub c2Mulrv: extern "C" fn(c2r, c2v) -> c2v,
    pub c2Add: extern "C" fn(c2v, c2v) -> c2v,
    pub c2Mulxv: extern "C" fn(c2x, c2v) -> c2v,
    pub c22: unsafe extern "C" fn(*mut c2Simplex),
    pub c23: unsafe extern "C" fn(*mut c2Simplex),
    pub c2Neg: extern "C" fn(c2v) -> c2v,
    pub c2Skew: extern "C" fn(c2v) -> c2v,
    pub c2CCW90: extern "C" fn(c2v) -> c2v,
    pub c2D: unsafe extern "C" fn(*mut c2Simplex) -> c2v,
    pub c2Support: unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int,
    pub c2Witness: unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v),
    pub c2Div: extern "C" fn(c2v, f32) -> c2v,
    pub c2Norm: extern "C" fn(c2v) -> c2v,
    pub c2L: unsafe extern "C" fn(*mut c2Simplex) -> c2v,
    pub c2MulrvT: extern "C" fn(c2r, c2v) -> c2v,
    #[allow(clippy::type_complexity)]
    pub c2GJK: unsafe extern "C" fn(
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
    ) -> f32,
    #[allow(clippy::type_complexity)]
    pub gjk: unsafe extern "C" fn(
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
    ),
}

macro_rules! sym {
    ($lib:expr, $name:literal, $t:ty) => {{
        let s: libloading::Symbol<$t> = unsafe {
            $lib.get(concat!($name, "\0").as_bytes())
                .unwrap_or_else(|e| panic!("missing symbol {}: {}", $name, e))
        };
        unsafe { *s.into_raw() }
    }};
}

fn load(path: &PathBuf, name: &'static str) -> Api {
    // Leaked so the fn pointers stay valid for the whole process.
    let lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(path).unwrap_or_else(|e| panic!("cannot dlopen {}: {}", path.display(), e))
    }));
    Api {
        name,
        c2V: sym!(lib, "c2V", extern "C" fn(f32, f32) -> c2v),
        c2Mulvs: sym!(lib, "c2Mulvs", extern "C" fn(c2v, f32) -> c2v),
        c2Maxv: sym!(lib, "c2Maxv", extern "C" fn(c2v, c2v) -> c2v),
        c2Minv: sym!(lib, "c2Minv", extern "C" fn(c2v, c2v) -> c2v),
        c2Clampv: sym!(lib, "c2Clampv", extern "C" fn(c2v, c2v, c2v) -> c2v),
        c2Sub: sym!(lib, "c2Sub", extern "C" fn(c2v, c2v) -> c2v),
        c2Dot: sym!(lib, "c2Dot", extern "C" fn(c2v, c2v) -> f32),
        c2RotIdentity: sym!(lib, "c2RotIdentity", extern "C" fn() -> c2r),
        c2xIdentity: sym!(lib, "c2xIdentity", extern "C" fn() -> c2x),
        c2BBVerts: sym!(lib, "c2BBVerts", unsafe extern "C" fn(*mut c2v, *mut c2AABB)),
        c2MakeProxy: sym!(
            lib,
            "c2MakeProxy",
            unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy)
        ),
        c2Len: sym!(lib, "c2Len", extern "C" fn(c2v) -> f32),
        c2Det2: sym!(lib, "c2Det2", extern "C" fn(c2v, c2v) -> f32),
        c2GJKSimplexMetric: sym!(
            lib,
            "c2GJKSimplexMetric",
            unsafe extern "C" fn(*mut c2Simplex) -> f32
        ),
        c2Mulrv: sym!(lib, "c2Mulrv", extern "C" fn(c2r, c2v) -> c2v),
        c2Add: sym!(lib, "c2Add", extern "C" fn(c2v, c2v) -> c2v),
        c2Mulxv: sym!(lib, "c2Mulxv", extern "C" fn(c2x, c2v) -> c2v),
        c22: sym!(lib, "c22", unsafe extern "C" fn(*mut c2Simplex)),
        c23: sym!(lib, "c23", unsafe extern "C" fn(*mut c2Simplex)),
        c2Neg: sym!(lib, "c2Neg", extern "C" fn(c2v) -> c2v),
        c2Skew: sym!(lib, "c2Skew", extern "C" fn(c2v) -> c2v),
        c2CCW90: sym!(lib, "c2CCW90", extern "C" fn(c2v) -> c2v),
        c2D: sym!(lib, "c2D", unsafe extern "C" fn(*mut c2Simplex) -> c2v),
        c2Support: sym!(
            lib,
            "c2Support",
            unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int
        ),
        c2Witness: sym!(
            lib,
            "c2Witness",
            unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v)
        ),
        c2Div: sym!(lib, "c2Div", extern "C" fn(c2v, f32) -> c2v),
        c2Norm: sym!(lib, "c2Norm", extern "C" fn(c2v) -> c2v),
        c2L: sym!(lib, "c2L", unsafe extern "C" fn(*mut c2Simplex) -> c2v),
        c2MulrvT: sym!(lib, "c2MulrvT", extern "C" fn(c2r, c2v) -> c2v),
        c2GJK: sym!(
            lib,
            "c2GJK",
            unsafe extern "C" fn(
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
            ) -> f32
        ),
        gjk: sym!(
            lib,
            "gjk",
            unsafe extern "C" fn(
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
            )
        ),
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("GJK_C_SO") {
        return PathBuf::from(p);
    }
    let dir = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("c_src/build not readable ({e}); build the C library first"))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    found
        .pop()
        .unwrap_or_else(|| panic!("no lib*.so found in {}", dir.display()))
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("GJK_RUST_SO") {
        return PathBuf::from(p);
    }
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libgjk_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libgjk_lib.so not found under {}; run `cargo build --release` first",
        base.display()
    );
}

pub struct Pair {
    pub c: Api,
    pub r: Api,
}

/// Loads both libraries. Called once per test binary (cheap enough to call
/// per test; `dlopen` refcounts the same handle).
pub fn libs() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair {
        c: load(&find_c_so(), "C"),
        r: load(&find_rust_so(), "Rust"),
    })
}

// ---------------------------------------------------------------------------
// Bit-exact comparison
// ---------------------------------------------------------------------------

/// Bit-for-bit float equality. The single exemption is the *payload* of a NaN:
/// both sides must be NaN, but the mantissa bits of a produced NaN are not part
/// of the C library's observable contract (they depend on which SSE operand the
/// hardware propagates). Sign of zero, infinities and every finite value are
/// compared strictly by bit pattern.
#[track_caller]
pub fn feq(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

pub fn veq(a: c2v, b: c2v) -> bool {
    feq(a.x, b.x) && feq(a.y, b.y)
}

pub fn req(a: c2r, b: c2r) -> bool {
    feq(a.c, b.c) && feq(a.s, b.s)
}

pub fn xeq(a: c2x, b: c2x) -> bool {
    veq(a.p, b.p) && req(a.r, b.r)
}

pub fn sveq(a: &c2sv, b: &c2sv) -> bool {
    veq(a.sA, b.sA) && veq(a.sB, b.sB) && veq(a.p, b.p) && feq(a.u, b.u) && a.iA == b.iA && a.iB == b.iB
}

pub fn simplexeq(a: &c2Simplex, b: &c2Simplex) -> bool {
    (0..4).all(|i| sveq(&a.verts[i], &b.verts[i])) && feq(a.div, b.div) && a.count == b.count
}

pub fn proxyeq(a: &c2Proxy, b: &c2Proxy) -> bool {
    feq(a.radius, b.radius) && a.count == b.count && (0..8).all(|i| veq(a.verts[i], b.verts[i]))
}

pub fn cacheeq(a: &c2GJKCache, b: &c2GJKCache) -> bool {
    feq(a.metric, b.metric) && a.count == b.count && a.iA == b.iA && a.iB == b.iB && feq(a.div, b.div)
}

#[track_caller]
pub fn assert_feq(ctx: &str, a: f32, b: f32) {
    assert!(
        feq(a, b),
        "{ctx}: C={a:?} (0x{:08x}) != Rust={b:?} (0x{:08x})",
        a.to_bits(),
        b.to_bits()
    );
}

#[track_caller]
pub fn assert_veq(ctx: &str, a: c2v, b: c2v) {
    assert!(
        veq(a, b),
        "{ctx}: C=({:?},{:?})[0x{:08x},0x{:08x}] != Rust=({:?},{:?})[0x{:08x},0x{:08x}]",
        a.x,
        a.y,
        a.x.to_bits(),
        a.y.to_bits(),
        b.x,
        b.y,
        b.x.to_bits(),
        b.y.to_bits()
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) + value generators
// ---------------------------------------------------------------------------

pub struct Rng(u64);

pub const SEED: u64 = 0x2545_F491_4F6C_DD1D;

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { SEED } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
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
    /// Uniform in [-s, s].
    pub fn sym(&mut self, s: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * s
    }
    /// A "well behaved" coordinate: moderate magnitude, plenty of exact ties.
    pub fn coord(&mut self) -> f32 {
        match self.below(6) {
            0 => self.below(9) as f32 - 4.0,           // integer grid
            1 => (self.below(41) as f32 - 20.0) * 0.5, // half grid
            2 => self.sym(1.0),
            3 => self.sym(100.0),
            4 => self.sym(1.0e6),
            _ => self.sym(10.0),
        }
    }
    /// A radius: mostly non-negative, occasionally zero or negative.
    pub fn radius(&mut self) -> f32 {
        match self.below(8) {
            0 => 0.0,
            1 => -self.unit() * 5.0,
            2 => self.below(5) as f32,
            _ => self.unit() * 8.0,
        }
    }
    /// The full float zoo, including every special class.
    pub fn wild(&mut self) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::NAN,
            5 => -f32::NAN,
            6 => f32::MAX,
            7 => f32::MIN,
            8 => f32::MIN_POSITIVE,
            9 => -f32::MIN_POSITIVE,
            10 => f32::from_bits(1),  // subnormal
            11 => f32::from_bits(0x8000_0001),
            12 => f32::EPSILON,
            13 => 1.192_092_9e-7,
            14 => f32::from_bits(self.next_u32()), // any bit pattern
            _ => self.coord(),
        }
    }
    pub fn wild_v(&mut self) -> c2v {
        c2v {
            x: self.wild(),
            y: self.wild(),
        }
    }
    pub fn coord_v(&mut self) -> c2v {
        c2v {
            x: self.coord(),
            y: self.coord(),
        }
    }
    pub fn wild_r(&mut self) -> c2r {
        match self.below(5) {
            0 => c2r { c: 1.0, s: 0.0 },
            1 => c2r { c: 0.0, s: 0.0 },
            2 => {
                // unit rotation from an angle
                let t = self.unit() * std::f32::consts::TAU;
                c2r {
                    c: t.cos(),
                    s: t.sin(),
                }
            }
            3 => c2r {
                c: self.coord(),
                s: self.coord(),
            },
            _ => c2r {
                c: self.wild(),
                s: self.wild(),
            },
        }
    }
    pub fn unit_r(&mut self) -> c2r {
        let t = self.unit() * std::f32::consts::TAU;
        c2r {
            c: t.cos(),
            s: t.sin(),
        }
    }
    pub fn wild_x(&mut self) -> c2x {
        c2x {
            p: self.coord_v(),
            r: self.wild_r(),
        }
    }
    pub fn unit_x(&mut self) -> c2x {
        c2x {
            p: self.coord_v(),
            r: self.unit_r(),
        }
    }
    pub fn circle(&mut self) -> c2Circle {
        c2Circle {
            p: self.coord_v(),
            r: self.radius(),
        }
    }
    pub fn aabb(&mut self) -> c2AABB {
        let a = self.coord_v();
        let b = self.coord_v();
        if self.below(6) == 0 {
            // inverted / degenerate on purpose
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
        let a = self.coord_v();
        let b = if self.below(8) == 0 { a } else { self.coord_v() };
        c2Capsule {
            a,
            b,
            r: self.radius(),
        }
    }
}

// ---------------------------------------------------------------------------
// Shape plumbing for c2GJK / c2MakeProxy
// ---------------------------------------------------------------------------

/// A shape kept in a fixed-size byte buffer so both libraries see the exact
/// same bytes at the exact same alignment.
#[repr(C, align(8))]
#[derive(Copy, Clone)]
pub struct ShapeBuf {
    pub bytes: [u8; 24],
    pub ty: c_int,
}

impl ShapeBuf {
    pub fn circle(c: c2Circle) -> Self {
        let mut b = ShapeBuf {
            bytes: [0; 24],
            ty: C2_TYPE_CIRCLE,
        };
        unsafe {
            std::ptr::copy_nonoverlapping(
                &c as *const c2Circle as *const u8,
                b.bytes.as_mut_ptr(),
                std::mem::size_of::<c2Circle>(),
            )
        };
        b
    }
    pub fn aabb(a: c2AABB) -> Self {
        let mut b = ShapeBuf {
            bytes: [0; 24],
            ty: C2_TYPE_AABB,
        };
        unsafe {
            std::ptr::copy_nonoverlapping(
                &a as *const c2AABB as *const u8,
                b.bytes.as_mut_ptr(),
                std::mem::size_of::<c2AABB>(),
            )
        };
        b
    }
    pub fn capsule(c: c2Capsule) -> Self {
        let mut b = ShapeBuf {
            bytes: [0; 24],
            ty: C2_TYPE_CAPSULE,
        };
        unsafe {
            std::ptr::copy_nonoverlapping(
                &c as *const c2Capsule as *const u8,
                b.bytes.as_mut_ptr(),
                std::mem::size_of::<c2Capsule>(),
            )
        };
        b
    }
    pub fn ptr(&self) -> *const c_void {
        self.bytes.as_ptr() as *const c_void
    }
}

/// Builds a random shape of the requested type.
pub fn rand_shape(rng: &mut Rng, ty: c_int) -> ShapeBuf {
    match ty {
        C2_TYPE_CIRCLE => ShapeBuf::circle(rng.circle()),
        C2_TYPE_AABB => ShapeBuf::aabb(rng.aabb()),
        _ => ShapeBuf::capsule(rng.capsule()),
    }
}

/// Translates a shape's coordinates by `d` (keeps radius).
pub fn shift_shape(s: &ShapeBuf, d: c2v) -> ShapeBuf {
    let mut out = *s;
    let n = match s.ty {
        C2_TYPE_CIRCLE => 1,
        C2_TYPE_AABB => 2,
        _ => 2,
    };
    unsafe {
        let p = out.bytes.as_mut_ptr() as *mut c2v;
        for i in 0..n {
            let v = *p.add(i);
            *p.add(i) = c2v {
                x: v.x + d.x,
                y: v.y + d.y,
            };
        }
    }
    out
}

/// Full-fidelity result of one `c2GJK` call.
#[derive(Debug, Copy, Clone)]
pub struct GjkOut {
    pub dist: f32,
    pub a: c2v,
    pub b: c2v,
    pub iters: c_int,
    pub cache: c2GJKCache,
    pub a_written: bool,
    pub b_written: bool,
    pub iters_written: bool,
}

const POISON_V: c2v = c2v {
    x: -12345.678,
    y: 98765.43,
};
const POISON_I: c_int = -0x5EED;

/// Calls `c2GJK` on `api`, exercising the NULL-vs-non-NULL out-pointer options.
#[allow(clippy::too_many_arguments)]
pub fn call_gjk(
    api: &Api,
    a: &ShapeBuf,
    ax: Option<&c2x>,
    b: &ShapeBuf,
    bx: Option<&c2x>,
    use_radius: c_int,
    want_out: bool,
    want_iters: bool,
    cache_in: Option<c2GJKCache>,
) -> GjkOut {
    let mut oa = POISON_V;
    let mut ob = POISON_V;
    let mut it = POISON_I;
    let mut cache = cache_in.unwrap_or_default();
    let dist = unsafe {
        (api.c2GJK)(
            a.ptr(),
            a.ty,
            ax.map(|x| x as *const c2x).unwrap_or(std::ptr::null()),
            b.ptr(),
            b.ty,
            bx.map(|x| x as *const c2x).unwrap_or(std::ptr::null()),
            if want_out { &mut oa } else { std::ptr::null_mut() },
            if want_out { &mut ob } else { std::ptr::null_mut() },
            use_radius,
            if want_iters {
                &mut it
            } else {
                std::ptr::null_mut()
            },
            if cache_in.is_some() {
                &mut cache
            } else {
                std::ptr::null_mut()
            },
        )
    };
    GjkOut {
        dist,
        a: oa,
        b: ob,
        iters: it,
        cache,
        a_written: !veq(oa, POISON_V),
        b_written: !veq(ob, POISON_V),
        iters_written: it != POISON_I,
    }
}

#[track_caller]
pub fn assert_gjk_eq(ctx: &str, c: &GjkOut, r: &GjkOut) {
    assert_feq(&format!("{ctx} dist"), c.dist, r.dist);
    assert_veq(&format!("{ctx} outA"), c.a, r.a);
    assert_veq(&format!("{ctx} outB"), c.b, r.b);
    assert_eq!(c.iters, r.iters, "{ctx} iterations");
    assert!(
        cacheeq(&c.cache, &r.cache),
        "{ctx} cache: C={:?} != Rust={:?}",
        c.cache,
        r.cache
    );
    assert_eq!(
        (c.a_written, c.b_written, c.iters_written),
        (r.a_written, r.b_written, r.iters_written),
        "{ctx} out-pointer write pattern"
    );
}

/// Builds a `c2Simplex` with random contents and the requested `count`/`div`.
pub fn rand_simplex(rng: &mut Rng, count: c_int, div: f32, wild: bool) -> c2Simplex {
    let mut s = c2Simplex {
        verts: [c2sv::default(); 4],
        div,
        count,
    };
    for i in 0..4 {
        s.verts[i] = c2sv {
            sA: if wild { rng.wild_v() } else { rng.coord_v() },
            sB: if wild { rng.wild_v() } else { rng.coord_v() },
            p: if wild { rng.wild_v() } else { rng.coord_v() },
            u: if wild { rng.wild() } else { rng.coord() },
            iA: rng.below(8) as c_int,
            iB: rng.below(8) as c_int,
        };
    }
    s
}
