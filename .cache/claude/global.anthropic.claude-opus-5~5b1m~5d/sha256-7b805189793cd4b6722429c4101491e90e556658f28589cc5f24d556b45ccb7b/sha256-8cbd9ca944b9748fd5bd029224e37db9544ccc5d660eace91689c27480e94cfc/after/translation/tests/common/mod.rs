//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and exposes each exported
//! symbol as a pair of function pointers (C, Rust). Nothing in the crate under
//! test is ever called directly — every call goes through `dlsym`, exactly as
//! an external C consumer would, so the `#[no_mangle]` export wrappers are
//! part of what is being verified.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI types — byte-identical mirrors of the C definitions
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: i32,
    pub iA: [i32; 3],
    pub iB: [i32; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: i32,
    pub verts: [c2v; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: i32,
    pub iB: i32,
}

/// `typedef struct { c2sv a, b, c, d; float div; int count; } c2Simplex;`
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: i32,
}

pub const C2_TYPE_CIRCLE: i32 = 0;
pub const C2_TYPE_AABB: i32 = 1;
pub const C2_TYPE_CAPSULE: i32 = 2;

pub const FLT_EPS: f32 = 1.19209289550781250000000000000000000e-7;
pub const FLT_MAX: f32 = 3.40282346638528859811704183484516925e+38;

pub const ZV: c2v = c2v { x: 0.0, y: 0.0 };

impl c2v {
    pub const fn new(x: f32, y: f32) -> c2v {
        c2v { x, y }
    }
}

impl Default for c2sv {
    fn default() -> c2sv {
        c2sv {
            sA: ZV,
            sB: ZV,
            p: ZV,
            u: 0.0,
            iA: 0,
            iB: 0,
        }
    }
}

impl Default for c2Simplex {
    fn default() -> c2Simplex {
        c2Simplex {
            verts: [c2sv::default(); 4],
            div: 1.0,
            count: 1,
        }
    }
}

impl Default for c2Proxy {
    fn default() -> c2Proxy {
        c2Proxy {
            radius: 0.0,
            count: 0,
            verts: [ZV; 8],
        }
    }
}

impl Default for c2GJKCache {
    fn default() -> c2GJKCache {
        c2GJKCache {
            metric: 0.0,
            count: 0,
            iA: [0; 3],
            iB: [0; 3],
            div: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Function-pointer type aliases
// ---------------------------------------------------------------------------

pub type FnV = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnVsV = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnVVV = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnVVVV = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnVVf = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnVf = unsafe extern "C" fn(c2v) -> f32;
pub type FnVV = unsafe extern "C" fn(c2v) -> c2v;
pub type FnR = unsafe extern "C" fn() -> c2r;
pub type FnX = unsafe extern "C" fn() -> c2x;
pub type FnRVV = unsafe extern "C" fn(c2r, c2v) -> c2v;
pub type FnXVV = unsafe extern "C" fn(c2x, c2v) -> c2v;
pub type FnBBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const c_void, i32, *mut c2Proxy);
pub type FnSimplexF = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type FnSimplexVoid = unsafe extern "C" fn(*mut c2Simplex);
pub type FnSimplexV = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type FnSupport = unsafe extern "C" fn(*const c2v, i32, c2v) -> i32;
pub type FnWitness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
pub type FnGJK = unsafe extern "C" fn(
    *const c_void,
    i32,
    *const c2x,
    *const c2v,
    i32,
    *const c2x,
    *mut c2v,
    *mut c2v,
    i32,
    *mut i32,
    *mut c2GJKCache,
) -> f32;
pub type FnGjk = unsafe extern "C" fn(
    std::ffi::c_char,
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
// One side of the differential pair: every symbol from one .so
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: &'static str,
    _lib: Library,

    pub c2V: FnV,
    pub c2Mulvs: FnVsV,
    pub c2Maxv: FnVVV,
    pub c2Minv: FnVVV,
    pub c2Clampv: FnVVVV,
    pub c2Sub: FnVVV,
    pub c2Add: FnVVV,
    pub c2Dot: FnVVf,
    pub c2Det2: FnVVf,
    pub c2Len: FnVf,
    pub c2Div: FnVsV,
    pub c2Norm: FnVV,
    pub c2Neg: FnVV,
    pub c2Skew: FnVV,
    pub c2CCW90: FnVV,
    pub c2RotIdentity: FnR,
    pub c2xIdentity: FnX,
    pub c2Mulrv: FnRVV,
    pub c2MulrvT: FnRVV,
    pub c2Mulxv: FnXVV,
    pub c2BBVerts: FnBBVerts,
    pub c2MakeProxy: FnMakeProxy,
    pub c2GJKSimplexMetric: FnSimplexF,
    pub c22: FnSimplexVoid,
    pub c23: FnSimplexVoid,
    pub c2D: FnSimplexV,
    pub c2L: FnSimplexV,
    pub c2Support: FnSupport,
    pub c2Witness: FnWitness,
    pub c2GJK: FnGJK,
    pub gjk: FnGjk,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    unsafe {
        let s: Symbol<T> = lib.get(name).unwrap_or_else(|e| {
            panic!(
                "symbol {:?} not found: {e}",
                std::str::from_utf8(name).unwrap()
            )
        });
        *s
    }
}

impl Api {
    unsafe fn load(path: &PathBuf, name: &'static str) -> Api {
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
                c2Add: sym(&lib, b"c2Add\0"),
                c2Dot: sym(&lib, b"c2Dot\0"),
                c2Det2: sym(&lib, b"c2Det2\0"),
                c2Len: sym(&lib, b"c2Len\0"),
                c2Div: sym(&lib, b"c2Div\0"),
                c2Norm: sym(&lib, b"c2Norm\0"),
                c2Neg: sym(&lib, b"c2Neg\0"),
                c2Skew: sym(&lib, b"c2Skew\0"),
                c2CCW90: sym(&lib, b"c2CCW90\0"),
                c2RotIdentity: sym(&lib, b"c2RotIdentity\0"),
                c2xIdentity: sym(&lib, b"c2xIdentity\0"),
                c2Mulrv: sym(&lib, b"c2Mulrv\0"),
                c2MulrvT: sym(&lib, b"c2MulrvT\0"),
                c2Mulxv: sym(&lib, b"c2Mulxv\0"),
                c2BBVerts: sym(&lib, b"c2BBVerts\0"),
                c2MakeProxy: sym(&lib, b"c2MakeProxy\0"),
                c2GJKSimplexMetric: sym(&lib, b"c2GJKSimplexMetric\0"),
                c22: sym(&lib, b"c22\0"),
                c23: sym(&lib, b"c23\0"),
                c2D: sym(&lib, b"c2D\0"),
                c2L: sym(&lib, b"c2L\0"),
                c2Support: sym(&lib, b"c2Support\0"),
                c2Witness: sym(&lib, b"c2Witness\0"),
                c2GJK: sym(&lib, b"c2GJK\0"),
                gjk: sym(&lib, b"gjk\0"),
                _lib: lib,
            }
        }
    }
}

pub struct Pair {
    pub c: Api,
    pub r: Api,
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent dir")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut best: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let is_so = p.extension().map(|x| x == "so").unwrap_or(false);
            let is_lib = p
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib"))
                .unwrap_or(false);
            if is_so && is_lib {
                best = Some(p);
            }
        }
    }
    best.unwrap_or_else(|| {
        panic!(
            "no lib*.so under {}. Build the C library first:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("GJK_RUST_SO") {
        return PathBuf::from(p);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Prefer the profile this test binary itself was built under, so that
    // `cargo test` and `cargo test --release` each pick a matching .so.
    let mut cands = Vec::new();
    if cfg!(debug_assertions) {
        cands.push(root.join("target/debug/libgjk_lib.so"));
        cands.push(root.join("target/release/libgjk_lib.so"));
    } else {
        cands.push(root.join("target/release/libgjk_lib.so"));
        cands.push(root.join("target/debug/libgjk_lib.so"));
    }
    for c in &cands {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "libgjk_lib.so not found (looked in {:?}). Run `cargo build` / `cargo build --release` first.",
        cands
    );
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn api() -> &'static Pair {
    PAIR.get_or_init(|| unsafe {
        Pair {
            c: Api::load(&find_c_so(), "C"),
            r: Api::load(&find_rust_so(), "Rust"),
        }
    })
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Raw bytes of any `Copy` POD, for byte-for-byte comparison.
pub fn raw<T: Copy>(v: &T) -> Vec<u8> {
    let p = v as *const T as *const u8;
    unsafe { std::slice::from_raw_parts(p, std::mem::size_of::<T>()) }.to_vec()
}

pub fn hexs(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[track_caller]
pub fn eq_f32(what: &str, ctx: &str, cv: f32, rv: f32) {
    assert_eq!(
        cv.to_bits(),
        rv.to_bits(),
        "{what} mismatch [{ctx}]: C={cv:?} (0x{:08x}) Rust={rv:?} (0x{:08x})",
        cv.to_bits(),
        rv.to_bits()
    );
}

#[track_caller]
pub fn eq_i32(what: &str, ctx: &str, cv: i32, rv: i32) {
    assert_eq!(cv, rv, "{what} mismatch [{ctx}]: C={cv} Rust={rv}");
}

/// Byte-for-byte equality of any POD out-param / return struct.
#[track_caller]
pub fn eq_bits<T: Copy + std::fmt::Debug>(what: &str, ctx: &str, cv: &T, rv: &T) {
    let (a, b) = (raw(cv), raw(rv));
    assert!(
        a == b,
        "{what} mismatch [{ctx}]:\n  C    = {cv:?}\n         {}\n  Rust = {rv:?}\n         {}",
        hexs(&a),
        hexs(&b)
    );
}

#[track_caller]
pub fn eq_v(what: &str, ctx: &str, cv: c2v, rv: c2v) {
    eq_bits(what, ctx, &cv, &rv);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) + float generators
// ---------------------------------------------------------------------------

pub struct Rng(u64);

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
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// Uniform in `[lo, hi)`.
    pub fn uniform(&mut self, lo: f32, hi: f32) -> f32 {
        let t = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        lo + (hi - lo) * t
    }
    /// A "well-behaved" coordinate: uniform in [-10, 10), sometimes snapped to
    /// a small integer / half-integer so that exact ties and exact zeros occur.
    pub fn coord(&mut self) -> f32 {
        match self.below(8) {
            0 => (self.below(11) as f32) - 5.0,
            1 => ((self.below(21) as f32) - 10.0) * 0.5,
            2 => 0.0,
            _ => self.uniform(-10.0, 10.0),
        }
    }
    pub fn vec(&mut self) -> c2v {
        c2v {
            x: self.coord(),
            y: self.coord(),
        }
    }
    /// A radius: mostly small non-negative, sometimes exactly 0 or negative.
    pub fn radius(&mut self) -> f32 {
        match self.below(8) {
            0 => 0.0,
            1 => -self.uniform(0.0, 3.0),
            2 => self.below(4) as f32,
            _ => self.uniform(0.0, 4.0),
        }
    }
    /// A proper unit rotation.
    pub fn rot(&mut self) -> c2r {
        let t = self.uniform(-3.15, 3.15);
        c2r {
            c: t.cos(),
            s: t.sin(),
        }
    }
    /// A rotation that may be unnormalised / degenerate — still legal input.
    pub fn rot_wild(&mut self) -> c2r {
        match self.below(6) {
            0 => c2r { c: 1.0, s: 0.0 },
            1 => c2r { c: 0.0, s: 0.0 },
            2 => c2r {
                c: self.coord(),
                s: self.coord(),
            },
            _ => self.rot(),
        }
    }
    pub fn xform(&mut self) -> c2x {
        c2x {
            p: self.vec(),
            r: self.rot(),
        }
    }
    pub fn xform_wild(&mut self) -> c2x {
        c2x {
            p: self.vec(),
            r: self.rot_wild(),
        }
    }
    /// Any f32 value class, including NaNs with assorted payloads and signs.
    pub fn wild_f32(&mut self) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::NAN,
            5 => f32::from_bits(0x7FC0_1234),
            6 => f32::from_bits(0xFFC0_ABCD),
            7 => f32::from_bits(0x7F80_0001), // signalling NaN
            8 => f32::from_bits(0xFF80_0007), // negative signalling NaN
            9 => f32::MIN_POSITIVE,
            10 => f32::from_bits(0x0000_0001), // subnormal
            11 => f32::from_bits(0x8000_0003), // negative subnormal
            12 => FLT_MAX,
            13 => -FLT_MAX,
            14 => f32::from_bits(self.next_u32()),
            _ => self.coord(),
        }
    }
    pub fn wild_vec(&mut self) -> c2v {
        c2v {
            x: self.wild_f32(),
            y: self.wild_f32(),
        }
    }
    /// A finite-but-extreme f32 (no NaN/Inf) — exercises overflow inside dot
    /// products without poisoning every comparison.
    pub fn big_f32(&mut self) -> f32 {
        let m = match self.below(4) {
            0 => 1.0e18f32,
            1 => 1.0e30f32,
            2 => 1.0e38f32,
            _ => 1.0e12f32,
        };
        m * self.uniform(-1.0, 1.0)
    }
    pub fn tiny_f32(&mut self) -> f32 {
        let m = match self.below(4) {
            0 => 1.0e-18f32,
            1 => 1.0e-30f32,
            2 => 1.0e-38f32,
            _ => 1.0e-12f32,
        };
        m * self.uniform(-1.0, 1.0)
    }
}

// ---------------------------------------------------------------------------
// Shape helpers
// ---------------------------------------------------------------------------

/// A shape as a tagged blob, so it can be handed to `c2GJK` / `c2MakeProxy`
/// as `const void *` with the matching `C2_TYPE`.
#[derive(Copy, Clone, Debug)]
pub enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

impl Shape {
    pub fn ty(&self) -> i32 {
        match self {
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
        }
    }
    pub fn ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(c) => c as *const c2Circle as *const c_void,
            Shape::Aabb(c) => c as *const c2AABB as *const c_void,
            Shape::Capsule(c) => c as *const c2Capsule as *const c_void,
        }
    }
    /// Shift the shape by `d` — used to build overlapping / touching / moved
    /// configurations and cache-invalidation sequences.
    pub fn shifted(&self, d: c2v) -> Shape {
        let s = |v: c2v| c2v {
            x: v.x + d.x,
            y: v.y + d.y,
        };
        match *self {
            Shape::Circle(c) => Shape::Circle(c2Circle { p: s(c.p), r: c.r }),
            Shape::Aabb(b) => Shape::Aabb(c2AABB {
                min: s(b.min),
                max: s(b.max),
            }),
            Shape::Capsule(c) => Shape::Capsule(c2Capsule {
                a: s(c.a),
                b: s(c.b),
                r: c.r,
            }),
        }
    }
    /// Rough centre, for building overlap/separation offsets.
    pub fn centre(&self) -> c2v {
        match *self {
            Shape::Circle(c) => c.p,
            Shape::Aabb(b) => c2v {
                x: (b.min.x + b.max.x) * 0.5,
                y: (b.min.y + b.max.y) * 0.5,
            },
            Shape::Capsule(c) => c2v {
                x: (c.a.x + c.b.x) * 0.5,
                y: (c.a.y + c.b.y) * 0.5,
            },
        }
    }
}

pub const KINDS: [i32; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

/// All 9 ordered (typeA, typeB) pairs.
pub fn shape_pairs() -> Vec<(i32, i32)> {
    let mut v = Vec::new();
    for a in KINDS {
        for b in KINDS {
            v.push((a, b));
        }
    }
    v
}

pub fn kind_name(k: i32) -> &'static str {
    match k {
        C2_TYPE_CIRCLE => "circle",
        C2_TYPE_AABB => "aabb",
        C2_TYPE_CAPSULE => "capsule",
        _ => "?",
    }
}

/// Build a random shape of the given kind, roughly centred on `at` with
/// extent scaled by `scale`.
pub fn rand_shape(rng: &mut Rng, kind: i32, at: c2v, scale: f32) -> Shape {
    match kind {
        C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
            p: at,
            r: rng.uniform(0.0, 1.0) * scale,
        }),
        C2_TYPE_AABB => {
            let hx = rng.uniform(0.05, 1.0) * scale;
            let hy = rng.uniform(0.05, 1.0) * scale;
            Shape::Aabb(c2AABB {
                min: c2v {
                    x: at.x - hx,
                    y: at.y - hy,
                },
                max: c2v {
                    x: at.x + hx,
                    y: at.y + hy,
                },
            })
        }
        _ => {
            let hx = rng.uniform(-1.0, 1.0) * scale;
            let hy = rng.uniform(-1.0, 1.0) * scale;
            Shape::Capsule(c2Capsule {
                a: c2v {
                    x: at.x - hx,
                    y: at.y - hy,
                },
                b: c2v {
                    x: at.x + hx,
                    y: at.y + hy,
                },
                r: rng.uniform(0.0, 1.0) * scale,
            })
        }
    }
}

/// Fully degenerate shape of the given kind at `at` (zero extent, zero radius).
pub fn degenerate_shape(kind: i32, at: c2v) -> Shape {
    match kind {
        C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: at, r: 0.0 }),
        C2_TYPE_AABB => Shape::Aabb(c2AABB { min: at, max: at }),
        _ => Shape::Capsule(c2Capsule { a: at, b: at, r: 0.0 }),
    }
}

// ---------------------------------------------------------------------------
// Differential drivers for the two "big" entry points
// ---------------------------------------------------------------------------

/// Every observable of one `c2GJK` call.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct GjkOut {
    pub dist: f32,
    pub a: c2v,
    pub b: c2v,
    pub iters: i32,
    pub cache: c2GJKCache,
}

#[derive(Copy, Clone, Debug)]
pub struct GjkOpts {
    pub use_radius: i32,
    pub ax: Option<c2x>,
    pub bx: Option<c2x>,
    pub want_a: bool,
    pub want_b: bool,
    pub want_iters: bool,
    pub cache: Option<c2GJKCache>,
}

impl Default for GjkOpts {
    fn default() -> GjkOpts {
        GjkOpts {
            use_radius: 1,
            ax: None,
            bx: None,
            want_a: true,
            want_b: true,
            want_iters: true,
            cache: None,
        }
    }
}

/// Poison pattern written into out-params before the call so that "the callee
/// did not write this" is distinguishable from "the callee wrote zero".
const POISON: c2v = c2v {
    x: -1.2345678e30,
    y: 9.8765434e29,
};

/// Run `c2GJK` through one `.so` and collect every observable.
pub fn call_gjk(api: &Api, a: &Shape, b: &Shape, o: &GjkOpts) -> GjkOut {
    let mut oa = POISON;
    let mut ob = POISON;
    let mut iters: i32 = -424242;
    let mut cache = o.cache.unwrap_or_default();

    let dist = unsafe {
        (api.c2GJK)(
            a.ptr(),
            a.ty(),
            o.ax.as_ref().map_or(std::ptr::null(), |x| x as *const c2x),
            b.ptr() as *const c2v,
            b.ty(),
            o.bx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x),
            if o.want_a {
                &mut oa as *mut c2v
            } else {
                std::ptr::null_mut()
            },
            if o.want_b {
                &mut ob as *mut c2v
            } else {
                std::ptr::null_mut()
            },
            o.use_radius,
            if o.want_iters {
                &mut iters as *mut i32
            } else {
                std::ptr::null_mut()
            },
            if o.cache.is_some() {
                &mut cache as *mut c2GJKCache
            } else {
                std::ptr::null_mut()
            },
        )
    };
    GjkOut {
        dist,
        a: oa,
        b: ob,
        iters,
        cache,
    }
}

/// Differential `c2GJK`: call both libraries and assert every observable
/// matches bit-for-bit.
#[track_caller]
pub fn diff_gjk(ctx: &str, a: &Shape, b: &Shape, o: &GjkOpts) -> GjkOut {
    let p = api();
    let cout = call_gjk(&p.c, a, b, o);
    let rout = call_gjk(&p.r, a, b, o);
    let ctx = format!("{ctx} | A={a:?} B={b:?} opts={o:?}");
    eq_f32("c2GJK return", &ctx, cout.dist, rout.dist);
    eq_v("c2GJK outA", &ctx, cout.a, rout.a);
    eq_v("c2GJK outB", &ctx, cout.b, rout.b);
    eq_i32("c2GJK iterations", &ctx, cout.iters, rout.iters);
    eq_bits("c2GJK cache", &ctx, &cout.cache, &rout.cache);
    cout
}

/// Every observable of one `gjk` call.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct GjkWrapOut {
    pub a: c2v,
    pub b: c2v,
}

pub fn call_gjk_wrap(
    api: &Api,
    reverse: i8,
    want_a: bool,
    want_b: bool,
    f: [f32; 9],
) -> GjkWrapOut {
    let mut oa = POISON;
    let mut ob = POISON;
    unsafe {
        (api.gjk)(
            reverse as std::ffi::c_char,
            if want_a {
                &mut oa as *mut c2v
            } else {
                std::ptr::null_mut()
            },
            if want_b {
                &mut ob as *mut c2v
            } else {
                std::ptr::null_mut()
            },
            f[0],
            f[1],
            f[2],
            f[3],
            f[4],
            f[5],
            f[6],
            f[7],
            f[8],
        );
    }
    GjkWrapOut { a: oa, b: ob }
}

#[track_caller]
pub fn diff_gjk_wrap(ctx: &str, reverse: i8, want_a: bool, want_b: bool, f: [f32; 9]) {
    let p = api();
    let cout = call_gjk_wrap(&p.c, reverse, want_a, want_b, f);
    let rout = call_gjk_wrap(&p.r, reverse, want_a, want_b, f);
    let ctx = format!("{ctx} | reverse={reverse} want=({want_a},{want_b}) f={f:?}");
    eq_v("gjk *a", &ctx, cout.a, rout.a);
    eq_v("gjk *b", &ctx, cout.b, rout.b);
}

// ---------------------------------------------------------------------------
// Differential drivers for the simplex functions
// ---------------------------------------------------------------------------

/// Differential `c22`: mutates a whole `c2Simplex` in place.
#[track_caller]
pub fn diff_c22(ctx: &str, s: &c2Simplex) {
    let p = api();
    let mut cs = *s;
    let mut rs = *s;
    unsafe {
        (p.c.c22)(&mut cs);
        (p.r.c22)(&mut rs);
    }
    eq_bits("c22 simplex", ctx, &cs, &rs);
}

#[track_caller]
pub fn diff_c23(ctx: &str, s: &c2Simplex) {
    let p = api();
    let mut cs = *s;
    let mut rs = *s;
    unsafe {
        (p.c.c23)(&mut cs);
        (p.r.c23)(&mut rs);
    }
    eq_bits("c23 simplex", ctx, &cs, &rs);
}

#[track_caller]
pub fn diff_c2D(ctx: &str, s: &c2Simplex) {
    let p = api();
    let mut cs = *s;
    let mut rs = *s;
    let (cv, rv) = unsafe { ((p.c.c2D)(&mut cs), (p.r.c2D)(&mut rs)) };
    eq_v("c2D return", ctx, cv, rv);
    eq_bits("c2D simplex (must be unmodified)", ctx, &cs, &rs);
}

#[track_caller]
pub fn diff_c2L(ctx: &str, s: &c2Simplex) {
    let p = api();
    let mut cs = *s;
    let mut rs = *s;
    let (cv, rv) = unsafe { ((p.c.c2L)(&mut cs), (p.r.c2L)(&mut rs)) };
    eq_v("c2L return", ctx, cv, rv);
    eq_bits("c2L simplex (must be unmodified)", ctx, &cs, &rs);
}

#[track_caller]
pub fn diff_metric(ctx: &str, s: &c2Simplex) {
    let p = api();
    let mut cs = *s;
    let mut rs = *s;
    let (cv, rv) = unsafe {
        (
            (p.c.c2GJKSimplexMetric)(&mut cs),
            (p.r.c2GJKSimplexMetric)(&mut rs),
        )
    };
    eq_f32("c2GJKSimplexMetric return", ctx, cv, rv);
    eq_bits("c2GJKSimplexMetric simplex", ctx, &cs, &rs);
}

#[track_caller]
pub fn diff_witness(ctx: &str, s: &c2Simplex) {
    let p = api();
    let mut cs = *s;
    let mut rs = *s;
    let (mut ca, mut cb) = (POISON, POISON);
    let (mut ra, mut rb) = (POISON, POISON);
    unsafe {
        (p.c.c2Witness)(&mut cs, &mut ca, &mut cb);
        (p.r.c2Witness)(&mut rs, &mut ra, &mut rb);
    }
    eq_v("c2Witness *a", ctx, ca, ra);
    eq_v("c2Witness *b", ctx, cb, rb);
    eq_bits("c2Witness simplex", ctx, &cs, &rs);
}

/// Build a simplex with the given `count`, `div` and the three `p`/`u` values.
pub fn simplex_with(count: i32, div: f32, ps: [c2v; 3], us: [f32; 3]) -> c2Simplex {
    let mut s = c2Simplex {
        verts: [c2sv::default(); 4],
        div,
        count,
    };
    for i in 0..3 {
        s.verts[i].p = ps[i];
        s.verts[i].u = us[i];
    }
    s
}

/// A fully randomised simplex: every field of all four `c2sv` slots is filled,
/// so that any accidental field mix-up or stale-slot read shows up.
pub fn rand_simplex(rng: &mut Rng, count: i32, wild: bool) -> c2Simplex {
    let mut s = c2Simplex {
        verts: [c2sv::default(); 4],
        div: if wild { rng.wild_f32() } else { rng.uniform(0.05, 6.0) },
        count,
    };
    for i in 0..4 {
        s.verts[i] = c2sv {
            sA: if wild { rng.wild_vec() } else { rng.vec() },
            sB: if wild { rng.wild_vec() } else { rng.vec() },
            p: if wild { rng.wild_vec() } else { rng.vec() },
            u: if wild { rng.wild_f32() } else { rng.uniform(-2.0, 4.0) },
            iA: (rng.below(4)) as i32,
            iB: (rng.below(4)) as i32,
        };
    }
    s
}
