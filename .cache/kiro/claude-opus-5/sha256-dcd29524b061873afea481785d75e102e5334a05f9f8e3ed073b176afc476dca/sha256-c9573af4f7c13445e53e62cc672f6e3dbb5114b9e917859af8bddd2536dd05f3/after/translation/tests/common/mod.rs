//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries (the C original and the Rust translation) with
//! `libloading` and exposes each exported symbol as a pair of function
//! pointers. The Rust crate is *never* called directly — always through its
//! `.so` exports — so the `#[no_mangle] extern "C"` wrappers and the struct
//! ABI are part of what is under test.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Types — layouts mirror c_src/src/lib.c exactly
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
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
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

/// `typedef struct { c2sv a, b, c, d; float div; int count; } c2Simplex;`
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

pub const FLT_EPSILON: f32 = 1.192_092_9e-7;

// ---------------------------------------------------------------------------
// Function pointer types
// ---------------------------------------------------------------------------

pub type FnVV = unsafe extern "C" fn(c2v) -> c2v;
pub type FnVVV = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnVVVV = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnVVF = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnVF = unsafe extern "C" fn(c2v) -> f32;
pub type FnVSV = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnFFV = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnRV = unsafe extern "C" fn(c2r, c2v) -> c2v;
pub type FnXV = unsafe extern "C" fn(c2x, c2v) -> c2v;
pub type FnR = unsafe extern "C" fn() -> c2r;
pub type FnX = unsafe extern "C" fn() -> c2x;
pub type FnBBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
pub type FnSupport = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
pub type FnSimplexF = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type FnSimplexV = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type FnSimplexVoid = unsafe extern "C" fn(*mut c2Simplex);
pub type FnWitness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
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
pub type FnGjk = unsafe extern "C" fn(
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
// Library wrapper
// ---------------------------------------------------------------------------

/// One loaded shared library, with every exported symbol resolved.
pub struct Impl {
    pub name: &'static str,
    _lib: Library,

    pub c2V: FnFFV,
    pub c2Mulvs: FnVSV,
    pub c2Maxv: FnVVV,
    pub c2Minv: FnVVV,
    pub c2Clampv: FnVVVV,
    pub c2Sub: FnVVV,
    pub c2Add: FnVVV,
    pub c2Dot: FnVVF,
    pub c2Det2: FnVVF,
    pub c2Len: FnVF,
    pub c2Neg: FnVV,
    pub c2Skew: FnVV,
    pub c2CCW90: FnVV,
    pub c2Div: FnVSV,
    pub c2Norm: FnVV,
    pub c2Mulrv: FnRV,
    pub c2MulrvT: FnRV,
    pub c2Mulxv: FnXV,
    pub c2RotIdentity: FnR,
    pub c2xIdentity: FnX,
    pub c2BBVerts: FnBBVerts,
    pub c2MakeProxy: FnMakeProxy,
    pub c2Support: FnSupport,
    pub c2GJKSimplexMetric: FnSimplexF,
    pub c22: FnSimplexVoid,
    pub c23: FnSimplexVoid,
    pub c2D: FnSimplexV,
    pub c2L: FnSimplexV,
    pub c2Witness: FnWitness,
    pub c2GJK: FnGJK,
    pub gjk: FnGjk,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8], which: &str) -> T {
    unsafe {
        let s: Symbol<T> = lib
            .get(name)
            .unwrap_or_else(|e| panic!("{which}: missing symbol {:?}: {e}", String::from_utf8_lossy(name)));
        *s
    }
}

impl Impl {
    pub fn load(name: &'static str, path: &PathBuf) -> Impl {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("cannot load {}: {e}", path.display()));
            Impl {
                name,
                c2V: sym(&lib, b"c2V\0", name),
                c2Mulvs: sym(&lib, b"c2Mulvs\0", name),
                c2Maxv: sym(&lib, b"c2Maxv\0", name),
                c2Minv: sym(&lib, b"c2Minv\0", name),
                c2Clampv: sym(&lib, b"c2Clampv\0", name),
                c2Sub: sym(&lib, b"c2Sub\0", name),
                c2Add: sym(&lib, b"c2Add\0", name),
                c2Dot: sym(&lib, b"c2Dot\0", name),
                c2Det2: sym(&lib, b"c2Det2\0", name),
                c2Len: sym(&lib, b"c2Len\0", name),
                c2Neg: sym(&lib, b"c2Neg\0", name),
                c2Skew: sym(&lib, b"c2Skew\0", name),
                c2CCW90: sym(&lib, b"c2CCW90\0", name),
                c2Div: sym(&lib, b"c2Div\0", name),
                c2Norm: sym(&lib, b"c2Norm\0", name),
                c2Mulrv: sym(&lib, b"c2Mulrv\0", name),
                c2MulrvT: sym(&lib, b"c2MulrvT\0", name),
                c2Mulxv: sym(&lib, b"c2Mulxv\0", name),
                c2RotIdentity: sym(&lib, b"c2RotIdentity\0", name),
                c2xIdentity: sym(&lib, b"c2xIdentity\0", name),
                c2BBVerts: sym(&lib, b"c2BBVerts\0", name),
                c2MakeProxy: sym(&lib, b"c2MakeProxy\0", name),
                c2Support: sym(&lib, b"c2Support\0", name),
                c2GJKSimplexMetric: sym(&lib, b"c2GJKSimplexMetric\0", name),
                c22: sym(&lib, b"c22\0", name),
                c23: sym(&lib, b"c23\0", name),
                c2D: sym(&lib, b"c2D\0", name),
                c2L: sym(&lib, b"c2L\0", name),
                c2Witness: sym(&lib, b"c2Witness\0", name),
                c2GJK: sym(&lib, b"c2GJK\0", name),
                gjk: sym(&lib, b"gjk\0", name),
                _lib: lib,
            }
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub r: Impl,
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = repo_root().join("c_src").join("build");
    let mut found: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found = Some(p);
                break;
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "no .so found in {}. Build the C library first:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // Allows the same suite to be run against a different build profile's
    // artifact (see scripts/check_all_configs.sh).
    if let Ok(p) = std::env::var("GJK_RUST_SO") {
        let p = PathBuf::from(p);
        if p.exists() {
            return p;
        }
        panic!("GJK_RUST_SO points at {} which does not exist", p.display());
    }
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    // Prefer the release artifact (the real shipped library), fall back to debug.
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("libgjk_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libgjk_lib.so not found under {}. Run `cargo build --release` first.",
        target.display()
    )
}

/// Path to the C `.so`, for the symbol-parity test.
pub fn c_so_path() -> PathBuf {
    find_c_so()
}

/// Path to the Rust `.so` actually under test.
pub fn rust_so_path() -> PathBuf {
    find_rust_so()
}

/// Loads both libraries. Cheap enough to call per test.
pub fn load_pair() -> Pair {
    Pair {
        c: Impl::load("C", &find_c_so()),
        r: Impl::load("Rust", &find_rust_so()),
    }
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

#[track_caller]
pub fn eq_f32(ctx: &str, c: f32, r: f32) {
    if c.to_bits() != r.to_bits() {
        panic!(
            "{ctx}: f32 mismatch\n  C    = {c:?} (bits 0x{:08x})\n  Rust = {r:?} (bits 0x{:08x})",
            c.to_bits(),
            r.to_bits()
        );
    }
}

#[track_caller]
pub fn eq_v(ctx: &str, c: c2v, r: c2v) {
    if c.x.to_bits() != r.x.to_bits() || c.y.to_bits() != r.y.to_bits() {
        panic!(
            "{ctx}: c2v mismatch\n  C    = ({:?}, {:?}) bits (0x{:08x}, 0x{:08x})\n  \
             Rust = ({:?}, {:?}) bits (0x{:08x}, 0x{:08x})",
            c.x,
            c.y,
            c.x.to_bits(),
            c.y.to_bits(),
            r.x,
            r.y,
            r.x.to_bits(),
            r.y.to_bits()
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
    eq_i(&format!("{ctx}.count"), c.count, r.count);
    eq_f32(&format!("{ctx}.div"), c.div, r.div);
    for i in 0..4 {
        eq_sv(&format!("{ctx}.verts[{i}]"), &c.verts[i], &r.verts[i]);
    }
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
    eq_f32(&format!("{ctx}.div"), c.div, r.div);
    for i in 0..3 {
        eq_i(&format!("{ctx}.iA[{i}]"), c.iA[i], r.iA[i]);
        eq_i(&format!("{ctx}.iB[{i}]"), c.iB[i], r.iB[i]);
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in [0, n).
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    /// Uniform in [-1, 1).
    pub fn unit(&mut self) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        u * 2.0 - 1.0
    }

    /// Uniform in [-scale, scale).
    pub fn scaled(&mut self, scale: f32) -> f32 {
        self.unit() * scale
    }

    /// A "nasty" f32: mixes ordinary values, exact halves, zeros (both signs),
    /// denormals, and huge magnitudes. Never NaN/inf (those live in the
    /// error-path tests).
    pub fn finite(&mut self) -> f32 {
        match self.below(10) {
            0 => 0.0,
            1 => -0.0,
            2 => self.scaled(1.0),
            3 => self.scaled(10.0),
            4 => self.scaled(1000.0),
            5 => self.scaled(1.0e18),
            6 => self.scaled(1.0e-30),
            7 => (self.below(9) as f32 - 4.0) * 0.5,
            8 => f32::from_bits(self.next_u32() & 0x007F_FFFF), // denormal
            _ => self.scaled(1.0e-6),
        }
    }

    /// A well-scaled coordinate, for geometry that should mostly be non-degenerate.
    pub fn coord(&mut self) -> f32 {
        self.scaled(20.0)
    }

    pub fn v(&mut self) -> c2v {
        c2v {
            x: self.finite(),
            y: self.finite(),
        }
    }

    pub fn v_coord(&mut self) -> c2v {
        c2v {
            x: self.coord(),
            y: self.coord(),
        }
    }

    /// A rotation from a random angle (unit `c2r`).
    pub fn rot(&mut self) -> c2r {
        let a = self.unit() * std::f32::consts::PI;
        c2r {
            c: a.cos(),
            s: a.sin(),
        }
    }

    pub fn x_transform(&mut self) -> c2x {
        c2x {
            p: self.v_coord(),
            r: self.rot(),
        }
    }
}

// ---------------------------------------------------------------------------
// Shape generation
// ---------------------------------------------------------------------------

/// A shape plus its `C2_TYPE`, kept in a box so the pointer stays stable.
pub enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

impl Shape {
    pub fn ty(&self) -> c_int {
        match self {
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
        }
    }

    pub fn as_ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(c) => c as *const c2Circle as *const c_void,
            Shape::Aabb(b) => b as *const c2AABB as *const c_void,
            Shape::Capsule(c) => c as *const c2Capsule as *const c_void,
        }
    }
}

/// Random shape of a given type, centred near `centre`, with extent ~`ext`.
pub fn gen_shape(rng: &mut Rng, ty: c_int, centre: c2v, ext: f32) -> Shape {
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
            p: centre,
            r: (rng.unit().abs() * ext).max(0.0),
        }),
        C2_TYPE_AABB => {
            let hx = rng.unit().abs() * ext + 1.0e-3;
            let hy = rng.unit().abs() * ext + 1.0e-3;
            Shape::Aabb(c2AABB {
                min: c2v {
                    x: centre.x - hx,
                    y: centre.y - hy,
                },
                max: c2v {
                    x: centre.x + hx,
                    y: centre.y + hy,
                },
            })
        }
        _ => Shape::Capsule(c2Capsule {
            a: c2v {
                x: centre.x + rng.scaled(ext),
                y: centre.y + rng.scaled(ext),
            },
            b: c2v {
                x: centre.x + rng.scaled(ext),
                y: centre.y + rng.scaled(ext),
            },
            r: rng.unit().abs() * ext,
        }),
    }
}

/// Result bundle of one `c2GJK` call, for differential comparison.
pub struct GjkOut {
    pub dist: f32,
    pub a: c2v,
    pub b: c2v,
    pub iters: c_int,
    pub cache: c2GJKCache,
}

/// Drives `c2GJK` on one implementation with the given configuration.
///
/// # Safety
/// `A`/`B` must be valid pointers for the given types.
#[allow(clippy::too_many_arguments)]
pub unsafe fn call_gjk(
    im: &Impl,
    a_shape: &Shape,
    ax: Option<&c2x>,
    b_shape: &Shape,
    bx: Option<&c2x>,
    use_radius: c_int,
    want_a: bool,
    want_b: bool,
    want_iters: bool,
    cache: Option<&mut c2GJKCache>,
) -> GjkOut {
    unsafe {
        let mut oa = c2v { x: 12.5, y: -7.25 };
        let mut ob = c2v { x: -3.125, y: 9.5 };
        let mut it: c_int = -12345;
        let mut local_cache = c2GJKCache::default();
        let cache_present = cache.is_some();
        if let Some(cc) = &cache {
            local_cache = **cc;
        }
        let dist = (im.c2GJK)(
            a_shape.as_ptr(),
            a_shape.ty(),
            ax.map(|p| p as *const c2x).unwrap_or(std::ptr::null()),
            b_shape.as_ptr(),
            b_shape.ty(),
            bx.map(|p| p as *const c2x).unwrap_or(std::ptr::null()),
            if want_a { &mut oa } else { std::ptr::null_mut() },
            if want_b { &mut ob } else { std::ptr::null_mut() },
            use_radius,
            if want_iters {
                &mut it
            } else {
                std::ptr::null_mut()
            },
            if cache_present {
                &mut local_cache
            } else {
                std::ptr::null_mut()
            },
        );
        if let Some(cc) = cache {
            *cc = local_cache;
        }
        GjkOut {
            dist,
            a: oa,
            b: ob,
            iters: it,
            cache: local_cache,
        }
    }
}

#[track_caller]
pub fn eq_gjk_out(ctx: &str, c: &GjkOut, r: &GjkOut) {
    eq_f32(&format!("{ctx}.dist"), c.dist, r.dist);
    eq_v(&format!("{ctx}.outA"), c.a, r.a);
    eq_v(&format!("{ctx}.outB"), c.b, r.b);
    eq_i(&format!("{ctx}.iterations"), c.iters, r.iters);
    eq_cache(&format!("{ctx}.cache"), &c.cache, &r.cache);
}
