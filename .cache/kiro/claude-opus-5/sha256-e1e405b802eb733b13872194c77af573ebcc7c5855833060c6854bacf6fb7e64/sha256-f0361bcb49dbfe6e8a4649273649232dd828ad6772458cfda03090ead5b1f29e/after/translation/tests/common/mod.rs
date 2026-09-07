//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and compares their exported symbols byte-for-byte.
//!
//! No Rust function is ever called directly — everything goes through
//! `dlsym` on the produced `cdylib`, exactly like an external C consumer.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI type mirrors (all of these are padding-free, so byte comparison is exact)
// ---------------------------------------------------------------------------

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

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

/// `typedef struct { c2sv a, b, c, d; float div; int count; } c2Simplex;`
/// modelled as an array because the C walks it as `c2sv *verts = &s.a;`.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    let build = manifest_dir().parent().unwrap().join("c_src/build");
    let mut cands: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e} (build the C library first)", build.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    cands.sort();
    assert!(!cands.is_empty(), "no .so found in {}", build.display());
    cands.remove(0)
}

fn find_rust_so() -> PathBuf {
    // NOTE: `cargo test` does NOT build a `cdylib`-only lib target, so the
    // `.so` must be produced by an explicit `cargo build` first. Loading a
    // stale artefact would silently validate nothing, so the mtime of the
    // `.so` is checked against every source file and the test aborts if the
    // library is older. Use `scripts/run_all.sh` (or `cargo build` followed by
    // `cargo test`) to keep them in sync.
    let target = manifest_dir().join("target");
    let debug = target.join("debug/libaabb_lib.so");
    let release = target.join("release/libaabb_lib.so");
    let prefer_debug = cfg!(debug_assertions);
    let order = if prefer_debug {
        [debug.clone(), release.clone()]
    } else {
        [release.clone(), debug.clone()]
    };
    let so = order
        .iter()
        .find(|p| p.exists())
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "Rust cdylib not found at {} or {} — run `cargo build` first",
                debug.display(),
                release.display()
            )
        });

    let so_mtime = std::fs::metadata(&so).unwrap().modified().unwrap();
    let mut newest_src: Option<(PathBuf, std::time::SystemTime)> = None;
    let src_dir = manifest_dir().join("src");
    let mut stack = vec![src_dir];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                let m = e.metadata().unwrap().modified().unwrap();
                if newest_src.as_ref().map(|(_, t)| m > *t).unwrap_or(true) {
                    newest_src = Some((p, m));
                }
            }
        }
    }
    if let Some((p, t)) = newest_src {
        assert!(
            so_mtime >= t,
            "STALE Rust cdylib: {} is older than {}.\n\
             `cargo test` does not rebuild a cdylib-only target — \
             run `cargo build` (or scripts/run_all.sh) first.",
            so.display(),
            p.display()
        );
    }
    so
}

/// Absolute paths of the two libraries under test (for reporting).
pub fn lib_paths() -> (PathBuf, PathBuf) {
    (find_c_so(), find_rust_so())
}

static C_LIB: OnceLock<&'static Library> = OnceLock::new();
static R_LIB: OnceLock<&'static Library> = OnceLock::new();

/// The C `.so`.
pub fn c() -> &'static Library {
    C_LIB.get_or_init(|| {
        let p = find_c_so();
        let lib = unsafe { Library::new(&p) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", p.display()));
        Box::leak(Box::new(lib))
    })
}

/// The Rust `.so`.
pub fn r() -> &'static Library {
    R_LIB.get_or_init(|| {
        let p = find_rust_so();
        let lib = unsafe { Library::new(&p) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", p.display()));
        Box::leak(Box::new(lib))
    })
}

pub fn sym<T>(lib: &'static Library, name: &str) -> Symbol<'static, T> {
    let mut n = name.as_bytes().to_vec();
    n.push(0);
    unsafe { lib.get(&n) }.unwrap_or_else(|e| panic!("dlsym {name} failed: {e}"))
}

/// Resolve the same exported symbol in both libraries: `(from_c, from_rust)`.
pub fn pair<T>(name: &str) -> (Symbol<'static, T>, Symbol<'static, T>) {
    (sym(c(), name), sym(r(), name))
}

// ---------------------------------------------------------------------------
// Bit-exact comparison
//
// Every value is decomposed into its `float` / `int` lanes (all the mirrored
// structs are padding-free, so the decomposition is exhaustive). Integer lanes
// and finite float lanes must match BIT for BIT — so `+0.0 != -0.0`, and a
// one-ULP difference is a failure. Two NaNs compare equal regardless of payload
// or sign: the payload a NaN carries out of `mulss`/`addss` depends on which
// source operand the compiler happened to put in the destination register, so
// it is an artefact of instruction selection rather than of the algorithm.
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Lane {
    F(f32),
    I(i32),
}

pub trait Lanes {
    fn lanes(&self, out: &mut Vec<Lane>);
}

impl Lanes for f32 {
    fn lanes(&self, out: &mut Vec<Lane>) {
        out.push(Lane::F(*self));
    }
}
impl Lanes for i32 {
    fn lanes(&self, out: &mut Vec<Lane>) {
        out.push(Lane::I(*self));
    }
}
impl<T: Lanes, const N: usize> Lanes for [T; N] {
    fn lanes(&self, out: &mut Vec<Lane>) {
        for v in self.iter() {
            v.lanes(out);
        }
    }
}
impl<T: Lanes> Lanes for Option<T> {
    fn lanes(&self, out: &mut Vec<Lane>) {
        match self {
            None => out.push(Lane::I(i32::MIN)),
            Some(v) => {
                out.push(Lane::I(0));
                v.lanes(out);
            }
        }
    }
}

macro_rules! impl_lanes {
    ($t:ty, $($f:ident),+) => {
        impl Lanes for $t {
            fn lanes(&self, out: &mut Vec<Lane>) {
                $( self.$f.lanes(out); )+
            }
        }
    };
}

impl_lanes!(c2v, x, y);
impl_lanes!(c2r, c, s);
impl_lanes!(c2x, p, r);
impl_lanes!(c2Circle, p, r);
impl_lanes!(c2AABB, min, max);
impl_lanes!(c2Capsule, a, b, r);
impl_lanes!(c2GJKCache, metric, count, iA, iB, div);
impl_lanes!(c2Proxy, radius, count, verts);
impl_lanes!(c2sv, sA, sB, p, u, iA, iB);
impl_lanes!(c2Simplex, verts, div, count);

fn lanes_of<T: Lanes>(v: &T) -> Vec<Lane> {
    let mut out = Vec::new();
    v.lanes(&mut out);
    out
}

fn lane_eq(a: Lane, b: Lane) -> bool {
    match (a, b) {
        (Lane::F(x), Lane::F(y)) => {
            x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan())
        }
        (Lane::I(x), Lane::I(y)) => x == y,
        _ => false,
    }
}

pub fn bytes_of<T: Copy>(v: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v as *const T as *const u8, std::mem::size_of::<T>()) }
}

pub fn same<T: Lanes>(a: &T, b: &T) -> bool {
    let (la, lb) = (lanes_of(a), lanes_of(b));
    la.len() == lb.len() && la.iter().zip(lb.iter()).all(|(&x, &y)| lane_eq(x, y))
}

#[track_caller]
pub fn assert_same<T: Lanes + std::fmt::Debug>(what: &str, c_val: T, r_val: T) {
    if !same(&c_val, &r_val) {
        let (la, lb) = (lanes_of(&c_val), lanes_of(&r_val));
        let bad: Vec<usize> = la
            .iter()
            .zip(lb.iter())
            .enumerate()
            .filter(|&(_, (&x, &y))| !lane_eq(x, y))
            .map(|(i, _)| i)
            .collect();
        panic!(
            "DIVERGENCE in {what}\n  C   : {c_val:?}\n  Rust: {r_val:?}\n  differing lanes {bad:?}\n  C lanes  : {la:?}\n  Rust lanes: {lb:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
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
    /// Uniform in `[0,1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in `[lo,hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }
    /// A "normal" coordinate: moderate magnitude, both signs.
    pub fn coord(&mut self) -> f32 {
        self.range(-120.0, 120.0)
    }
    pub fn radius(&mut self) -> f32 {
        self.range(0.0, 40.0)
    }
    /// Completely arbitrary bit pattern reinterpreted as `f32`
    /// (may be NaN / Inf / subnormal).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A value drawn from the "interesting" set, biased towards edge cases.
    pub fn spicy_f32(&mut self) -> f32 {
        const SPECIALS: [f32; 18] = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            f32::MAX,
            f32::MIN,
            f32::MIN_POSITIVE,
            -f32::MIN_POSITIVE,
            1e-30,
            -1e-30,
            1e30,
            -1e30,
            f32::EPSILON,
            -f32::EPSILON,
            0.5,
        ];
        match self.next_u32() % 4 {
            0 => SPECIALS[(self.next_u32() as usize) % SPECIALS.len()],
            1 => self.any_f32(),
            _ => self.coord(),
        }
    }
    pub fn vec(&mut self) -> c2v {
        c2v {
            x: self.coord(),
            y: self.coord(),
        }
    }
    pub fn spicy_vec(&mut self) -> c2v {
        c2v {
            x: self.spicy_f32(),
            y: self.spicy_f32(),
        }
    }
    pub fn circle(&mut self) -> c2Circle {
        c2Circle {
            p: self.vec(),
            r: self.radius(),
        }
    }
    pub fn aabb(&mut self) -> c2AABB {
        let a = self.vec();
        let b = self.vec();
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
    pub fn capsule(&mut self) -> c2Capsule {
        c2Capsule {
            a: self.vec(),
            b: self.vec(),
            r: self.radius(),
        }
    }
    /// Unit rotation from a random angle, plus a random translation.
    pub fn xform(&mut self) -> c2x {
        let ang = self.range(-3.2, 3.2);
        c2x {
            p: self.vec(),
            r: c2r {
                c: ang.cos(),
                s: ang.sin(),
            },
        }
    }
    pub fn rot(&mut self) -> c2r {
        let ang = self.range(-3.2, 3.2);
        c2r {
            c: ang.cos(),
            s: ang.sin(),
        }
    }
    /// Fill a `c2sv` with arbitrary bytes (including nonsense indices/u).
    pub fn sv(&mut self) -> c2sv {
        c2sv {
            sA: self.vec(),
            sB: self.vec(),
            p: self.vec(),
            u: self.range(-2.0, 2.0),
            iA: (self.next_u32() % 4) as c_int,
            iB: (self.next_u32() % 4) as c_int,
        }
    }
    pub fn simplex(&mut self, count: c_int) -> c2Simplex {
        c2Simplex {
            verts: [self.sv(), self.sv(), self.sv(), self.sv()],
            div: self.range(-4.0, 4.0),
            count,
        }
    }
}

// ---------------------------------------------------------------------------
// Frequently used signature aliases
// ---------------------------------------------------------------------------

pub type FnVV = extern "C" fn(c2v) -> c2v;
pub type FnVVV = extern "C" fn(c2v, c2v) -> c2v;
pub type FnVVf = extern "C" fn(c2v, c2v) -> f32;
pub type FnVf = extern "C" fn(c2v) -> f32;
pub type FnSimplexV = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type FnSimplexF = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type FnSimplexVoid = unsafe extern "C" fn(*mut c2Simplex);
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
pub type FnCollided = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;
pub type FnMakeProxy = unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
pub type FnBBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
pub type FnSupport = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
pub type FnWitness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
pub type FnAabb = extern "C" fn(f32, f32, f32, f32) -> c_int;

/// Full observable result of one `c2GJK` call, for byte comparison.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GjkResult {
    pub dist: f32,
    pub outA: c2v,
    pub outB: c2v,
    pub iterations: c_int,
    pub cache: c2GJKCache,
    pub cache_used: c_int,
}

impl_lanes!(GjkResult, dist, outA, outB, iterations, cache, cache_used);

/// Drive `c2GJK` through one `.so` and capture everything it can observably
/// write. `cache_in == None` passes a NULL cache pointer.
pub unsafe fn call_gjk(
    f: &FnGJK,
    a: *const c_void,
    ta: c_int,
    ax: Option<&c2x>,
    b: *const c_void,
    tb: c_int,
    bx: Option<&c2x>,
    use_radius: c_int,
    cache_in: Option<c2GJKCache>,
) -> GjkResult {
    // Pre-seed the out-params with a recognisable pattern so a *missing* write
    // is detected rather than silently reading zero.
    let mut outA = c2v {
        x: f32::from_bits(0xDEAD_BEEF),
        y: f32::from_bits(0xCAFE_BABE),
    };
    let mut outB = c2v {
        x: f32::from_bits(0xFEED_FACE),
        y: f32::from_bits(0x8BAD_F00D),
    };
    let mut iters: c_int = -12345;
    let mut cache = cache_in.unwrap_or_default();
    let cache_ptr: *mut c2GJKCache = if cache_in.is_some() {
        &raw mut cache
    } else {
        std::ptr::null_mut()
    };
    let dist = unsafe {
        f(
            a,
            ta,
            ax.map(|x| x as *const c2x).unwrap_or(std::ptr::null()),
            b,
            tb,
            bx.map(|x| x as *const c2x).unwrap_or(std::ptr::null()),
            &raw mut outA,
            &raw mut outB,
            use_radius,
            &raw mut iters,
            cache_ptr,
        )
    };
    GjkResult {
        dist,
        outA,
        outB,
        iterations: iters,
        cache,
        cache_used: cache_in.is_some() as c_int,
    }
}

/// The nine valid `(typeA, typeB)` pairs.
pub const TYPE_PAIRS: [(c_int, c_int); 9] = [
    (C2_TYPE_CIRCLE, C2_TYPE_CIRCLE),
    (C2_TYPE_CIRCLE, C2_TYPE_AABB),
    (C2_TYPE_CIRCLE, C2_TYPE_CAPSULE),
    (C2_TYPE_AABB, C2_TYPE_CIRCLE),
    (C2_TYPE_AABB, C2_TYPE_AABB),
    (C2_TYPE_AABB, C2_TYPE_CAPSULE),
    (C2_TYPE_CAPSULE, C2_TYPE_CIRCLE),
    (C2_TYPE_CAPSULE, C2_TYPE_AABB),
    (C2_TYPE_CAPSULE, C2_TYPE_CAPSULE),
];

/// A shape of any of the three kinds, stored in a buffer with C layout so it
/// can be handed to `c2GJK` / `c2Collided` as `const void *`.
#[derive(Copy, Clone, Debug)]
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
    pub fn ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(c) => c as *const c2Circle as *const c_void,
            Shape::Aabb(c) => c as *const c2AABB as *const c_void,
            Shape::Capsule(c) => c as *const c2Capsule as *const c_void,
        }
    }
}

pub fn rand_shape(rng: &mut Rng, ty: c_int) -> Shape {
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(rng.circle()),
        C2_TYPE_AABB => Shape::Aabb(rng.aabb()),
        _ => Shape::Capsule(rng.capsule()),
    }
}
