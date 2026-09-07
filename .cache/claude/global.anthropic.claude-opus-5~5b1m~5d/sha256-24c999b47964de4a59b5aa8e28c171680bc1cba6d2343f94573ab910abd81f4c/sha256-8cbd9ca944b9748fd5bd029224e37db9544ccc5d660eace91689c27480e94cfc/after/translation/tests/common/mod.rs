//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and exposes every exported
//! symbol as a raw `extern "C"` function pointer, so the Rust library is
//! always called across the real FFI boundary (exercising the
//! `#[no_mangle]` export wrappers), never as a direct Rust call.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// ABI mirror types (must match c_src/src/lib.c exactly)
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
        c2Proxy { radius: 0.0, count: 0, verts: [c2v::default(); 8] }
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

/// C spells this `c2sv a, b, c, d;` — four consecutive `c2sv`.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

pub const C2_TYPE_CIRCLE: c_uint = 0;
pub const C2_TYPE_AABB: c_uint = 1;
pub const C2_TYPE_CAPSULE: c_uint = 2;

pub const FLT_MAX: f32 = 3.402_823_466_385_288_6e38;
pub const FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7;

// ---------------------------------------------------------------------------
// Function-pointer table
// ---------------------------------------------------------------------------

pub type FnV = extern "C" fn(f32, f32) -> c2v;
pub type FnVs = extern "C" fn(c2v, f32) -> c2v;
pub type FnVV = extern "C" fn(c2v, c2v) -> c2v;
pub type FnVVV = extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnVVf = extern "C" fn(c2v, c2v) -> f32;
pub type FnVf = extern "C" fn(c2v) -> f32;
pub type FnVU = extern "C" fn(c2v) -> c2v;
pub type FnR = extern "C" fn() -> c2r;
pub type FnX = extern "C" fn() -> c2x;
pub type FnBBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const c_void, c_uint, *mut c2Proxy);
pub type FnSimplexF = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type FnSimplexV = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type FnSimplexVoid = unsafe extern "C" fn(*mut c2Simplex);
pub type FnRV = extern "C" fn(c2r, c2v) -> c2v;
pub type FnXV = extern "C" fn(c2x, c2v) -> c2v;
pub type FnSupport = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
pub type FnWitness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
pub type FnGJK = unsafe extern "C" fn(
    *const c_void,
    c_uint,
    *const c2x,
    *const c_void,
    c_uint,
    *const c2x,
    *mut c2v,
    *mut c2v,
    c_int,
    *mut c_int,
    *mut c2GJKCache,
) -> f32;
pub type FnGjkCache = unsafe extern "C" fn(
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

pub struct Api {
    pub name: &'static str,
    pub c2V: FnV,
    pub c2Mulvs: FnVs,
    pub c2Maxv: FnVV,
    pub c2Minv: FnVV,
    pub c2Clampv: FnVVV,
    pub c2Sub: FnVV,
    pub c2Dot: FnVVf,
    pub c2RotIdentity: FnR,
    pub c2xIdentity: FnX,
    pub c2BBVerts: FnBBVerts,
    pub c2MakeProxy: FnMakeProxy,
    pub c2Len: FnVf,
    pub c2Det2: FnVVf,
    pub c2GJKSimplexMetric: FnSimplexF,
    pub c2Mulrv: FnRV,
    pub c2Add: FnVV,
    pub c2Mulxv: FnXV,
    pub c22: FnSimplexVoid,
    pub c23: FnSimplexVoid,
    pub c2Neg: FnVU,
    pub c2Skew: FnVU,
    pub c2CCW90: FnVU,
    pub c2D: FnSimplexV,
    pub c2Support: FnSupport,
    pub c2Witness: FnWitness,
    pub c2Div: FnVs,
    pub c2Norm: FnVU,
    pub c2L: FnSimplexV,
    pub c2MulrvT: FnRV,
    pub c2GJK: FnGJK,
    pub gjk_cache: FnGjkCache,
}

unsafe fn get<T: Copy>(lib: &'static Library, name: &str) -> T {
    let s: Symbol<T> = lib
        .get(name.as_bytes())
        .unwrap_or_else(|e| panic!("symbol `{name}` not found: {e}"));
    *s
}

impl Api {
    unsafe fn from_lib(name: &'static str, lib: &'static Library) -> Api {
        Api {
            name,
            c2V: get(lib, "c2V"),
            c2Mulvs: get(lib, "c2Mulvs"),
            c2Maxv: get(lib, "c2Maxv"),
            c2Minv: get(lib, "c2Minv"),
            c2Clampv: get(lib, "c2Clampv"),
            c2Sub: get(lib, "c2Sub"),
            c2Dot: get(lib, "c2Dot"),
            c2RotIdentity: get(lib, "c2RotIdentity"),
            c2xIdentity: get(lib, "c2xIdentity"),
            c2BBVerts: get(lib, "c2BBVerts"),
            c2MakeProxy: get(lib, "c2MakeProxy"),
            c2Len: get(lib, "c2Len"),
            c2Det2: get(lib, "c2Det2"),
            c2GJKSimplexMetric: get(lib, "c2GJKSimplexMetric"),
            c2Mulrv: get(lib, "c2Mulrv"),
            c2Add: get(lib, "c2Add"),
            c2Mulxv: get(lib, "c2Mulxv"),
            c22: get(lib, "c22"),
            c23: get(lib, "c23"),
            c2Neg: get(lib, "c2Neg"),
            c2Skew: get(lib, "c2Skew"),
            c2CCW90: get(lib, "c2CCW90"),
            c2D: get(lib, "c2D"),
            c2Support: get(lib, "c2Support"),
            c2Witness: get(lib, "c2Witness"),
            c2Div: get(lib, "c2Div"),
            c2Norm: get(lib, "c2Norm"),
            c2L: get(lib, "c2L"),
            c2MulrvT: get(lib, "c2MulrvT"),
            c2GJK: get(lib, "c2GJK"),
            gjk_cache: get(lib, "gjk_cache"),
        }
    }
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = repo_root().join("c_src").join("build");
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                cands.push(p);
            }
        }
    }
    cands.sort();
    cands.pop().unwrap_or_else(|| {
        panic!(
            "no .so found in {}; build the C first:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // An explicit override always wins (used by run_all.sh).
    if let Ok(p) = std::env::var("RUST_SO") {
        let p = PathBuf::from(p);
        if p.exists() {
            return p;
        }
    }
    let target = repo_root().join("translation").join("target");
    // Derive the profile from THIS test binary's own path, so a `--release`
    // test run always loads the release cdylib and a debug run always loads the
    // debug one.  Picking "whichever file is newest" would silently test the
    // wrong artifact.
    let me = std::env::current_exe().expect("current_exe");
    let profile = me
        .components()
        .rev()
        .find_map(|c| match c.as_os_str().to_str() {
            Some(s @ ("release" | "debug")) => Some(s.to_string()),
            _ => None,
        })
        .unwrap_or_else(|| "release".to_string());
    let p = target.join(&profile).join("libgjk_cache_lib.so");
    if p.exists() {
        return p;
    }
    // Fall back to the other profile only if the matching one is absent.
    for other in ["release", "debug"] {
        let q = target.join(other).join("libgjk_cache_lib.so");
        if q.exists() {
            return q;
        }
    }
    panic!(
        "libgjk_cache_lib.so not found under {}; run `cargo build --release`",
        target.display()
    )
}

/// Paths of the two loaded shared objects, for the harness self-check.
pub fn so_paths() -> (PathBuf, PathBuf) {
    (find_c_so(), find_rust_so())
}

/// The two loaded APIs: `.0` is C (ground truth), `.1` is Rust.
pub fn apis() -> &'static (Api, Api) {
    use std::sync::OnceLock;
    static ONCE: OnceLock<(Api, Api)> = OnceLock::new();
    ONCE.get_or_init(|| unsafe {
        let cpath = find_c_so();
        let rpath = find_rust_so();
        let clib: &'static Library =
            Box::leak(Box::new(Library::new(&cpath).expect("dlopen C .so")));
        let rlib: &'static Library =
            Box::leak(Box::new(Library::new(&rpath).expect("dlopen Rust .so")));
        (Api::from_lib("C", clib), Api::from_lib("RUST", rlib))
    })
}

pub mod gjk;

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub fn fb(x: f32) -> u32 {
    x.to_bits()
}

pub fn vb(v: c2v) -> (u32, u32) {
    (v.x.to_bits(), v.y.to_bits())
}

pub fn rb(r: c2r) -> (u32, u32) {
    (r.c.to_bits(), r.s.to_bits())
}

pub fn xb(x: c2x) -> ((u32, u32), (u32, u32)) {
    (vb(x.p), rb(x.r))
}

pub fn proxy_bits(p: &c2Proxy) -> Vec<u32> {
    let mut out = vec![p.radius.to_bits(), p.count as u32];
    for v in p.verts.iter() {
        out.push(v.x.to_bits());
        out.push(v.y.to_bits());
    }
    out
}

pub fn cache_bits(c: &c2GJKCache) -> Vec<u32> {
    let mut out = vec![c.metric.to_bits(), c.count as u32];
    for i in 0..3 {
        out.push(c.iA[i] as u32);
    }
    for i in 0..3 {
        out.push(c.iB[i] as u32);
    }
    out.push(c.div.to_bits());
    out
}

pub fn simplex_bits(s: &c2Simplex) -> Vec<u32> {
    let mut out = Vec::new();
    for v in s.verts.iter() {
        out.extend_from_slice(&[
            v.sA.x.to_bits(),
            v.sA.y.to_bits(),
            v.sB.x.to_bits(),
            v.sB.y.to_bits(),
            v.p.x.to_bits(),
            v.p.y.to_bits(),
            v.u.to_bits(),
            v.iA as u32,
            v.iB as u32,
        ]);
    }
    out.push(s.div.to_bits());
    out.push(s.count as u32);
    out
}

/// Raw byte view of any `Copy` POD, for "was this buffer left untouched?" checks.
pub fn raw_bytes<T: Copy>(t: &T) -> Vec<u8> {
    let p = t as *const T as *const u8;
    unsafe { std::slice::from_raw_parts(p, std::mem::size_of::<T>()) }.to_vec()
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*) + float generators
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
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Uniform in [0,1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in [-mag, mag].
    pub fn sym(&mut self, mag: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * mag
    }
    /// Arbitrary bit pattern (NaNs, infs, denormals all reachable).
    pub fn any_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A "small integer-ish" value, so exact ties / exact zeros happen often.
    pub fn coarse(&mut self, mag: f32) -> f32 {
        let n = (self.below(21) as i32) - 10;
        n as f32 * (mag / 10.0)
    }
    /// Mostly-finite generator with a sprinkling of hard special values.
    pub fn spicy(&mut self, mag: f32) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::NAN,
            5 => f32::from_bits(0xFFC0_0001), // negative NaN, non-canonical payload
            6 => f32::MIN_POSITIVE / 3.0,     // denormal
            7 => FLT_MAX,
            8 => -FLT_MAX,
            9 => FLT_EPSILON,
            10 => self.coarse(mag),
            _ => self.sym(mag),
        }
    }
    pub fn v(&mut self, mag: f32) -> c2v {
        c2v { x: self.sym(mag), y: self.sym(mag) }
    }
    pub fn v_coarse(&mut self, mag: f32) -> c2v {
        c2v { x: self.coarse(mag), y: self.coarse(mag) }
    }
    pub fn v_spicy(&mut self, mag: f32) -> c2v {
        c2v { x: self.spicy(mag), y: self.spicy(mag) }
    }
    pub fn v_bits(&mut self) -> c2v {
        c2v { x: self.any_bits(), y: self.any_bits() }
    }
    /// A unit-ish rotation.
    pub fn rot(&mut self) -> c2r {
        let t = self.unit() * std::f32::consts::TAU;
        c2r { c: t.cos(), s: t.sin() }
    }
}

// ---------------------------------------------------------------------------
// assertion macro
// ---------------------------------------------------------------------------

#[macro_export]
macro_rules! diff_eq {
    ($ctx:expr, $c:expr, $r:expr) => {{
        let cv = $c;
        let rv = $r;
        if cv != rv {
            panic!(
                "DIVERGENCE [{}]\n  C    = {:?}\n  RUST = {:?}",
                $ctx, cv, rv
            );
        }
    }};
}
