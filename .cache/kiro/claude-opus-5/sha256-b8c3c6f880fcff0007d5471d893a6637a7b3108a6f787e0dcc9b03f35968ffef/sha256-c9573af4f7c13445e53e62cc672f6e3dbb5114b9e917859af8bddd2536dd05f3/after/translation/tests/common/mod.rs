//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls every function
//! through its exported C symbol. The Rust crate is never linked directly, so
//! the `#[no_mangle] extern "C"` wrappers are part of what is under test.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// C-compatible types (layout must match c_src/src/lib.c exactly)
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
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
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
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

pub const C2_TYPE_CIRCLE: u32 = 0;
pub const C2_TYPE_AABB: u32 = 1;
pub const C2_TYPE_CAPSULE: u32 = 2;

pub const ALL_TYPES: [u32; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

// ---------------------------------------------------------------------------
// Function pointer types
// ---------------------------------------------------------------------------

pub type FnVV = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnV_Vf = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnV_VV = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnV_VVV = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnF_VV = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnF_V = unsafe extern "C" fn(c2v) -> f32;
pub type FnV_V = unsafe extern "C" fn(c2v) -> c2v;
pub type FnR_void = unsafe extern "C" fn() -> c2r;
pub type FnX_void = unsafe extern "C" fn() -> c2x;
pub type FnBBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const c_void, u32, *mut c2Proxy);
pub type FnF_S = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type FnV_RV = unsafe extern "C" fn(c2r, c2v) -> c2v;
pub type FnV_XV = unsafe extern "C" fn(c2x, c2v) -> c2v;
pub type FnVoid_S = unsafe extern "C" fn(*mut c2Simplex);
pub type FnV_S = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type FnSupport = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
pub type FnWitness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
pub type FnGJK = unsafe extern "C" fn(
    *const c_void,
    u32,
    *const c2x,
    *const c_void,
    u32,
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

// ---------------------------------------------------------------------------
// A loaded implementation: every symbol resolved from one `.so`
// ---------------------------------------------------------------------------

pub struct Impl {
    _lib: Library,
    pub name: &'static str,
    pub c2V: FnVV,
    pub c2Mulvs: FnV_Vf,
    pub c2Maxv: FnV_VV,
    pub c2Minv: FnV_VV,
    pub c2Clampv: FnV_VVV,
    pub c2Sub: FnV_VV,
    pub c2Dot: FnF_VV,
    pub c2RotIdentity: FnR_void,
    pub c2xIdentity: FnX_void,
    pub c2BBVerts: FnBBVerts,
    pub c2MakeProxy: FnMakeProxy,
    pub c2Len: FnF_V,
    pub c2Det2: FnF_VV,
    pub c2GJKSimplexMetric: FnF_S,
    pub c2Mulrv: FnV_RV,
    pub c2Add: FnV_VV,
    pub c2Mulxv: FnV_XV,
    pub c22: FnVoid_S,
    pub c23: FnVoid_S,
    pub c2Neg: FnV_V,
    pub c2Skew: FnV_V,
    pub c2CCW90: FnV_V,
    pub c2D: FnV_S,
    pub c2Support: FnSupport,
    pub c2Witness: FnWitness,
    pub c2Div: FnV_Vf,
    pub c2Norm: FnV_V,
    pub c2L: FnV_S,
    pub c2MulrvT: FnV_RV,
    pub c2GJK: FnGJK,
    pub gjk_cache: FnGjkCache,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &str) -> T {
    unsafe {
        let s: Symbol<T> = lib
            .get(format!("{name}\0").as_bytes())
            .unwrap_or_else(|e| panic!("missing symbol {name}: {e}"));
        *s
    }
}

impl Impl {
    pub fn load(path: &Path, name: &'static str) -> Impl {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("cannot load {}: {e}", path.display()));
            Impl {
                name,
                c2V: sym(&lib, "c2V"),
                c2Mulvs: sym(&lib, "c2Mulvs"),
                c2Maxv: sym(&lib, "c2Maxv"),
                c2Minv: sym(&lib, "c2Minv"),
                c2Clampv: sym(&lib, "c2Clampv"),
                c2Sub: sym(&lib, "c2Sub"),
                c2Dot: sym(&lib, "c2Dot"),
                c2RotIdentity: sym(&lib, "c2RotIdentity"),
                c2xIdentity: sym(&lib, "c2xIdentity"),
                c2BBVerts: sym(&lib, "c2BBVerts"),
                c2MakeProxy: sym(&lib, "c2MakeProxy"),
                c2Len: sym(&lib, "c2Len"),
                c2Det2: sym(&lib, "c2Det2"),
                c2GJKSimplexMetric: sym(&lib, "c2GJKSimplexMetric"),
                c2Mulrv: sym(&lib, "c2Mulrv"),
                c2Add: sym(&lib, "c2Add"),
                c2Mulxv: sym(&lib, "c2Mulxv"),
                c22: sym(&lib, "c22"),
                c23: sym(&lib, "c23"),
                c2Neg: sym(&lib, "c2Neg"),
                c2Skew: sym(&lib, "c2Skew"),
                c2CCW90: sym(&lib, "c2CCW90"),
                c2D: sym(&lib, "c2D"),
                c2Support: sym(&lib, "c2Support"),
                c2Witness: sym(&lib, "c2Witness"),
                c2Div: sym(&lib, "c2Div"),
                c2Norm: sym(&lib, "c2Norm"),
                c2L: sym(&lib, "c2L"),
                c2MulrvT: sym(&lib, "c2MulrvT"),
                c2GJK: sym(&lib, "c2GJK"),
                gjk_cache: sym(&lib, "gjk_cache"),
                _lib: lib,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_so(dir: &Path, want_prefix: Option<&str>) -> Option<PathBuf> {
    let mut hits: Vec<PathBuf> = Vec::new();
    for e in std::fs::read_dir(dir).ok()? {
        let p = e.ok()?.path();
        if p.extension().and_then(|s| s.to_str()) != Some("so") {
            continue;
        }
        let fname = p.file_name()?.to_str()?.to_string();
        if let Some(pre) = want_prefix {
            if !fname.starts_with(pre) {
                continue;
            }
        }
        hits.push(p);
    }
    hits.sort();
    hits.into_iter().next()
}

pub fn c_so_path() -> PathBuf {
    // Allow an alternative C build (e.g. a -O2 one produced out-of-tree) to be
    // substituted, so the translation can be checked against more than one
    // compilation of the same C source.
    if let Ok(p) = std::env::var("C2_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "C2_C_SO does not point at a file: {}", p.display());
        return p;
    }
    let build = crate_root().parent().unwrap().join("c_src/build");
    find_so(&build, None).unwrap_or_else(|| {
        panic!(
            "no C .so found in {} -- build it with cmake first",
            build.display()
        )
    })
}

/// Newest modification time under `src/`, used to detect a stale `.so`.
fn newest_source_mtime() -> std::time::SystemTime {
    fn walk(dir: &Path, newest: &mut std::time::SystemTime) {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, newest);
                } else if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                    if m > *newest {
                        *newest = m;
                    }
                }
            }
        }
    }
    let mut newest = std::time::UNIX_EPOCH;
    walk(&crate_root().join("src"), &mut newest);
    if let Ok(m) = std::fs::metadata(crate_root().join("Cargo.toml")).and_then(|m| m.modified()) {
        if m > newest {
            newest = m;
        }
    }
    newest
}

pub fn rust_so_path() -> PathBuf {
    // An explicit override is used by the mutation-sensitivity search, which
    // loads a deliberately-broken build to prove the suite can see the change.
    if let Ok(p) = std::env::var("C2_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "C2_RUST_SO does not point at a file: {}", p.display());
        return p;
    }
    // The test binary lives in target/<profile>/deps/, so the cdylib produced by
    // `cargo build` for the same profile is two levels up.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe.parent().and_then(|p| p.parent()).map(|p| p.to_path_buf());
    let mut found = None;
    if let Some(d) = profile_dir {
        found = find_so(&d, Some("libgjk_cache_lib"));
    }
    if found.is_none() {
        for cand in ["target/release", "target/debug"] {
            let d = crate_root().join(cand);
            if let Some(p) = find_so(&d, Some("libgjk_cache_lib")) {
                found = Some(p);
                break;
            }
        }
    }
    let path = found.expect(
        "no Rust libgjk_cache_lib.so found -- `cargo test` does NOT build a cdylib, \
         so you must run `cargo build` (same profile) first",
    );

    // CRITICAL: `cargo test` builds only the test binaries, never the cdylib.
    // Without this guard a stale `.so` from an earlier build would be loaded and
    // every differential test would silently verify the wrong binary.
    let so_mtime = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .expect("cannot stat the Rust .so");
    let src_mtime = newest_source_mtime();
    assert!(
        so_mtime >= src_mtime,
        "STALE Rust .so: {} was built at {:?} but the newest source is {:?}.\n\
         `cargo test` does not rebuild a cdylib -- run `cargo build [--release]` first \
         (or use ./verify_all.sh, which always does).",
        path.display(),
        so_mtime,
        src_mtime
    );
    path
}

pub struct Pair {
    pub c: Impl,
    pub r: Impl,
}

pub fn load_pair() -> Pair {
    Pair {
        c: Impl::load(&c_so_path(), "C"),
        r: Impl::load(&rust_so_path(), "Rust"),
    }
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Strict bit equality, including the NaN sign bit and payload.
pub fn feq_strict(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

/// Bit equality with NaN *payload/sign* treated as equivalent.
///
/// Rationale, verified empirically (see `NAN_NOTE` below): the sign bit of a
/// NaN produced by `mulss`/`addss`/`subss` depends on which operand the
/// compiler places in the destination register, and gcc changes that choice
/// between `-O0` (the CMake default for this project) and `-O1`/`-O2`/`-O3`/`-Os`.
/// It is therefore a property of one particular C *build*, not of the C
/// *source*, and IEEE-754 leaves it unspecified. Everything else -- which
/// results are NaN at all, the sign of infinities, the sign of zeros, and every
/// finite value -- is compared bit-for-bit.
pub fn feq(a: f32, b: f32) -> bool {
    if a.to_bits() == b.to_bits() {
        return true;
    }
    a.is_nan() && b.is_nan()
}

pub const NAN_NOTE: &str = "NaN payload/sign is compiler-build dependent (gcc -O0 vs -O1+ disagree) and is compared NaN-vs-NaN rather than bitwise";

pub fn veq(a: c2v, b: c2v) -> bool {
    feq(a.x, b.x) && feq(a.y, b.y)
}

pub fn req(a: c2r, b: c2r) -> bool {
    feq(a.c, b.c) && feq(a.s, b.s)
}

pub fn xeq(a: c2x, b: c2x) -> bool {
    veq(a.p, b.p) && req(a.r, b.r)
}

pub fn svq(a: &c2sv, b: &c2sv) -> bool {
    veq(a.sA, b.sA) && veq(a.sB, b.sB) && veq(a.p, b.p) && feq(a.u, b.u) && a.iA == b.iA
        && a.iB == b.iB
}

pub fn simplex_eq(a: &c2Simplex, b: &c2Simplex) -> bool {
    (0..4).all(|i| svq(&a.verts[i], &b.verts[i])) && feq(a.div, b.div) && a.count == b.count
}

pub fn proxy_eq(a: &c2Proxy, b: &c2Proxy) -> bool {
    feq(a.radius, b.radius)
        && a.count == b.count
        && (0..8).all(|i| veq(a.verts[i], b.verts[i]))
}

pub fn cache_eq(a: &c2GJKCache, b: &c2GJKCache) -> bool {
    feq(a.metric, b.metric) && a.count == b.count && a.iA == b.iA && a.iB == b.iB
        && feq(a.div, b.div)
}

pub fn verts_eq(a: &[c2v], b: &[c2v]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| veq(*x, *y))
}

/// Raw byte comparison — the strongest possible check for out-parameters.
pub fn bytes_of<T>(v: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v as *const T as *const u8, std::mem::size_of::<T>()) }
}

pub fn beq<T>(a: &T, b: &T) -> bool {
    bytes_of(a) == bytes_of(b)
}

pub fn hex<T>(v: &T) -> String {
    bytes_of(v).iter().map(|b| format!("{b:02x}")).collect()
}

/// Assert two floats are bit-identical (NaN-payload tolerant), with context.
#[track_caller]
pub fn ck_f(c: f32, r: f32, ctx: &str) {
    assert!(
        feq(c, r),
        "float divergence: C={c:?} (0x{:08x}) Rust={r:?} (0x{:08x}) :: {ctx}",
        c.to_bits(),
        r.to_bits()
    );
}

/// Assert two `c2v` are bit-identical, with context.
#[track_caller]
pub fn ck_v(c: c2v, r: c2v, ctx: &str) {
    assert!(
        veq(c, r),
        "c2v divergence: C=({:?},{:?}) [{}] Rust=({:?},{:?}) [{}] :: {ctx}",
        c.x,
        c.y,
        hex(&c),
        r.x,
        r.y,
        hex(&r)
    );
}

#[track_caller]
pub fn ck_i(c: c_int, r: c_int, ctx: &str) {
    assert_eq!(c, r, "int divergence :: {ctx}");
}

/// Assert two values are byte-identical, with context.
#[track_caller]
pub fn ck_b<T>(c: &T, r: &T, ctx: &str) {
    assert!(
        beq(c, r),
        "byte divergence:\n  C   = {}\n  Rust= {}\n  :: {ctx}",
        hex(c),
        hex(r)
    );
}

#[track_caller]
pub fn ck_proxy(c: &c2Proxy, r: &c2Proxy, ctx: &str) {
    assert!(
        proxy_eq(c, r),
        "c2Proxy divergence:\n  C   = {:?}\n  Rust= {:?}\n  C   bytes={}\n  Rustbytes={}\n  :: {ctx}",
        c,
        r,
        hex(c),
        hex(r)
    );
}

#[track_caller]
pub fn ck_simplex(c: &c2Simplex, r: &c2Simplex, ctx: &str) {
    assert!(
        simplex_eq(c, r),
        "c2Simplex divergence:\n  C   = {:?}\n  Rust= {:?}\n  :: {ctx}",
        c,
        r
    );
}

#[track_caller]
pub fn ck_cache(c: &c2GJKCache, r: &c2GJKCache, ctx: &str) {
    assert!(
        cache_eq(c, r),
        "c2GJKCache divergence:\n  C   = {:?}\n  Rust= {:?}\n  :: {ctx}",
        c,
        r
    );
}

#[track_caller]
pub fn ck_verts(c: &[c2v], r: &[c2v], ctx: &str) {
    assert!(
        verts_eq(c, r),
        "c2v[] divergence:\n  C   = {:?}\n  Rust= {:?}\n  :: {ctx}",
        c,
        r
    );
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_9876;

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
    /// Uniform in [0,1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in [-mag, mag].
    pub fn sym(&mut self, mag: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * mag
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    pub fn bool(&mut self) -> bool {
        self.next_u32() & 1 == 1
    }

    /// A float drawn from a wide distribution: normal values most of the time,
    /// plus the interesting specials (zeros, inf, NaN, subnormals, huge).
    pub fn wild_f32(&mut self) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::NAN,
            5 => f32::MIN_POSITIVE,
            6 => -f32::MIN_POSITIVE,
            7 => f32::from_bits(1), // subnormal
            8 => f32::MAX,
            9 => f32::MIN,
            10 => 1.0,
            11 => -1.0,
            12 => self.sym(1.0e18),
            13 => self.sym(1.0e-18),
            _ => self.sym(100.0),
        }
    }

    /// A tame finite float in [-mag, mag] (never NaN/inf).
    pub fn tame_f32(&mut self, mag: f32) -> f32 {
        self.sym(mag)
    }

    pub fn wild_v(&mut self) -> c2v {
        c2v {
            x: self.wild_f32(),
            y: self.wild_f32(),
        }
    }
    pub fn tame_v(&mut self, mag: f32) -> c2v {
        c2v {
            x: self.tame_f32(mag),
            y: self.tame_f32(mag),
        }
    }
    pub fn rot(&mut self) -> c2r {
        match self.below(8) {
            0 => c2r { c: 1.0, s: 0.0 },
            1 => c2r {
                c: self.wild_f32(),
                s: self.wild_f32(),
            },
            2 => c2r { c: 0.0, s: 0.0 },
            _ => {
                let a = self.unit() * std::f32::consts::TAU;
                c2r { c: a.cos(), s: a.sin() }
            }
        }
    }
    pub fn xform(&mut self, mag: f32) -> c2x {
        c2x {
            p: self.tame_v(mag),
            r: self.rot(),
        }
    }
}

// ---------------------------------------------------------------------------
// Random shape builders (valid, interesting shapes)
// ---------------------------------------------------------------------------

/// Owns the bytes a shape occupies so a `*const c_void` stays valid.
#[derive(Clone)]
pub enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

impl Shape {
    pub fn ty(&self) -> u32 {
        match self {
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
        }
    }
    pub fn as_ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(c) => c as *const c2Circle as *const c_void,
            Shape::Aabb(c) => c as *const c2AABB as *const c_void,
            Shape::Capsule(c) => c as *const c2Capsule as *const c_void,
        }
    }
    pub fn describe(&self) -> String {
        match self {
            Shape::Circle(c) => format!("Circle(p=({},{}),r={})", c.p.x, c.p.y, c.r),
            Shape::Aabb(c) => format!(
                "AABB(min=({},{}),max=({},{}))",
                c.min.x, c.min.y, c.max.x, c.max.y
            ),
            Shape::Capsule(c) => format!(
                "Capsule(a=({},{}),b=({},{}),r={})",
                c.a.x, c.a.y, c.b.x, c.b.y, c.r
            ),
        }
    }
}

/// Build a random shape of the requested type with coordinates in `[-mag,mag]`.
/// `degenerate_chance` (0..16) controls how often a degenerate variant is used.
pub fn rand_shape(rng: &mut Rng, ty: u32, mag: f32, degenerate_chance: u32) -> Shape {
    let degen = rng.below(16) < degenerate_chance;
    match ty {
        C2_TYPE_CIRCLE => {
            let p = rng.tame_v(mag);
            let r = if degen {
                match rng.below(3) {
                    0 => 0.0,
                    1 => -rng.unit() * mag, // negative radius: C does not validate
                    _ => mag * 10.0,
                }
            } else {
                rng.unit() * mag * 0.5
            };
            Shape::Circle(c2Circle { p, r })
        }
        C2_TYPE_AABB => {
            let a = rng.tame_v(mag);
            let b = rng.tame_v(mag);
            let bb = if degen {
                match rng.below(4) {
                    0 => c2AABB { min: a, max: a }, // zero area
                    1 => c2AABB {
                        min: a,
                        max: c2v { x: a.x, y: b.y },
                    }, // zero width
                    2 => c2AABB {
                        min: a,
                        max: c2v { x: b.x, y: a.y },
                    }, // zero height
                    _ => c2AABB { min: b, max: a }, // possibly inverted
                }
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
            };
            Shape::Aabb(bb)
        }
        _ => {
            let a = rng.tame_v(mag);
            let b = if degen && rng.bool() { a } else { rng.tame_v(mag) };
            let r = if degen {
                match rng.below(3) {
                    0 => 0.0,
                    1 => -rng.unit() * mag,
                    _ => mag * 10.0,
                }
            } else {
                rng.unit() * mag * 0.5
            };
            Shape::Capsule(c2Capsule { a, b, r })
        }
    }
}

pub fn type_name(t: u32) -> &'static str {
    match t {
        C2_TYPE_CIRCLE => "CIRCLE",
        C2_TYPE_AABB => "AABB",
        C2_TYPE_CAPSULE => "CAPSULE",
        _ => "INVALID",
    }
}

// ---------------------------------------------------------------------------
// Full c2GJK differential call: runs both .so and compares everything
// ---------------------------------------------------------------------------

pub struct GjkOpts {
    pub ax: Option<c2x>,
    pub bx: Option<c2x>,
    pub use_radius: c_int,
    pub want_out_a: bool,
    pub want_out_b: bool,
    pub want_iterations: bool,
    pub cache: bool,
}

impl Default for GjkOpts {
    fn default() -> Self {
        GjkOpts {
            ax: None,
            bx: None,
            use_radius: 1,
            want_out_a: true,
            want_out_b: true,
            want_iterations: true,
            cache: false,
        }
    }
}

/// Result of one side of a differential `c2GJK` call.
#[derive(Debug)]
pub struct GjkOut {
    pub dist: f32,
    pub a: c2v,
    pub b: c2v,
    pub iters: c_int,
    pub cache: c2GJKCache,
}

/// Sentinel patterns pre-written into out-params so "not written" is detectable.
const SENTINEL_V: c2v = c2v {
    x: -1.234_567_8e-11,
    y: 9.876_543e21,
};
const SENTINEL_I: c_int = -0x5A5A_5A5A;

#[allow(clippy::too_many_arguments)]
pub fn gjk_once(
    imp: &Impl,
    A: &Shape,
    tyA: u32,
    B: &Shape,
    tyB: u32,
    opts: &GjkOpts,
    cache_in: &c2GJKCache,
) -> GjkOut {
    let mut a = SENTINEL_V;
    let mut b = SENTINEL_V;
    let mut iters: c_int = SENTINEL_I;
    let mut cache = *cache_in;

    let axs = opts.ax;
    let bxs = opts.bx;
    let ax_ptr = match &axs {
        Some(v) => v as *const c2x,
        None => std::ptr::null(),
    };
    let bx_ptr = match &bxs {
        Some(v) => v as *const c2x,
        None => std::ptr::null(),
    };

    let dist = unsafe {
        (imp.c2GJK)(
            A.as_ptr(),
            tyA,
            ax_ptr,
            B.as_ptr(),
            tyB,
            bx_ptr,
            if opts.want_out_a {
                &mut a as *mut c2v
            } else {
                std::ptr::null_mut()
            },
            if opts.want_out_b {
                &mut b as *mut c2v
            } else {
                std::ptr::null_mut()
            },
            opts.use_radius,
            if opts.want_iterations {
                &mut iters as *mut c_int
            } else {
                std::ptr::null_mut()
            },
            if opts.cache {
                &mut cache as *mut c2GJKCache
            } else {
                std::ptr::null_mut()
            },
        )
    };

    GjkOut {
        dist,
        a,
        b,
        iters,
        cache,
    }
}

/// Run one `c2GJK` configuration through both `.so`s and assert full parity.
#[allow(clippy::too_many_arguments)]
pub fn gjk_diff(
    p: &Pair,
    A: &Shape,
    tyA: u32,
    B: &Shape,
    tyB: u32,
    opts: &GjkOpts,
    cache_in: &c2GJKCache,
    ctx: &str,
) -> (GjkOut, GjkOut) {
    let oc = gjk_once(&p.c, A, tyA, B, tyB, opts, cache_in);
    let or = gjk_once(&p.r, A, tyA, B, tyB, opts, cache_in);
    let where_ = format!(
        "{ctx} | A={} ty={} B={} ty={} | ax={} bx={} use_radius={} cache={}",
        A.describe(),
        type_name(tyA),
        B.describe(),
        type_name(tyB),
        opts.ax.is_some(),
        opts.bx.is_some(),
        opts.use_radius,
        opts.cache
    );
    assert!(
        feq(oc.dist, or.dist),
        "dist divergence C={:?}(0x{:08x}) Rust={:?}(0x{:08x}) :: {where_}",
        oc.dist,
        or.dist.to_bits(),
        or.dist,
        or.dist.to_bits()
    );
    assert!(
        veq(oc.a, or.a),
        "outA divergence C=({:?},{:?}) Rust=({:?},{:?}) :: {where_}",
        oc.a.x,
        oc.a.y,
        or.a.x,
        or.a.y
    );
    assert!(
        veq(oc.b, or.b),
        "outB divergence C=({:?},{:?}) Rust=({:?},{:?}) :: {where_}",
        oc.b.x,
        oc.b.y,
        or.b.x,
        or.b.y
    );
    assert_eq!(oc.iters, or.iters, "iterations divergence :: {where_}");
    assert!(
        cache_eq(&oc.cache, &or.cache),
        "cache divergence\n  C   = {:?}\n  Rust= {:?}\n  :: {where_}",
        oc.cache,
        or.cache
    );
    (oc, or)
}

pub fn sentinel_v() -> c2v {
    SENTINEL_V
}
pub fn sentinel_i() -> c_int {
    SENTINEL_I
}
