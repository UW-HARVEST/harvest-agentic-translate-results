//! Shared harness: loads both the C `.so` and the Rust `.so` through
//! `libloading` and exposes every exported symbol as a plain `extern "C"`
//! function pointer.  Nothing in this crate is ever called directly; every
//! call goes through the dynamic-library boundary exactly like an external
//! C consumer would do it.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use std::ffi::c_int;
use std::ffi::c_void;
use std::path::PathBuf;

use libloading::Library;

// ---------------------------------------------------------------------------
// ABI-compatible type definitions (mirror of the C declarations)
// ---------------------------------------------------------------------------

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2Simplex {
    pub a: c2sv,
    pub b: c2sv,
    pub c: c2sv,
    pub d: c2sv,
    pub div: f32,
    pub count: c_int,
}

// ---------------------------------------------------------------------------
// Signature aliases
// ---------------------------------------------------------------------------

pub type FnVV = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnVVf = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnV = unsafe extern "C" fn(c2v) -> c2v;
pub type FnVf = unsafe extern "C" fn(c2v) -> f32;
pub type FnVsV = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnFFV = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnVVVV = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnRotId = unsafe extern "C" fn() -> c2r;
pub type FnXId = unsafe extern "C" fn() -> c2x;
pub type FnRVV = unsafe extern "C" fn(c2r, c2v) -> c2v;
pub type FnXVV = unsafe extern "C" fn(c2x, c2v) -> c2v;
pub type FnBBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
pub type FnSimplexF = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type FnSimplexV = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type FnSimplex = unsafe extern "C" fn(*mut c2Simplex);
pub type FnSupport = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
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
pub type FnAABBtoAABB = unsafe extern "C" fn(c2AABB, c2AABB) -> c_int;
pub type FnAABBtoCapsule = unsafe extern "C" fn(c2AABB, c2Capsule) -> c_int;
pub type FnCapsuletoCapsule = unsafe extern "C" fn(c2Capsule, c2Capsule) -> c_int;
pub type FnCircletoCircle = unsafe extern "C" fn(c2Circle, c2Circle) -> c_int;
pub type FnCircletoAABB = unsafe extern "C" fn(c2Circle, c2AABB) -> c_int;
pub type FnCircletoCapsule = unsafe extern "C" fn(c2Circle, c2Capsule) -> c_int;
pub type FnCollided = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;
pub type FnReverseCollide = unsafe extern "C" fn(f32, f32, f32) -> c_int;

// ---------------------------------------------------------------------------
// The loaded API surface
// ---------------------------------------------------------------------------

pub struct Api {
    pub label: &'static str,
    pub c2V: FnFFV,
    pub c2Mulvs: FnVsV,
    pub c2Maxv: FnVV,
    pub c2Minv: FnVV,
    pub c2Clampv: FnVVVV,
    pub c2Sub: FnVV,
    pub c2Add: FnVV,
    pub c2Dot: FnVVf,
    pub c2Det2: FnVVf,
    pub c2Len: FnVf,
    pub c2Neg: FnV,
    pub c2Skew: FnV,
    pub c2CCW90: FnV,
    pub c2Norm: FnV,
    pub c2Div: FnVsV,
    pub c2RotIdentity: FnRotId,
    pub c2xIdentity: FnXId,
    pub c2Mulrv: FnRVV,
    pub c2MulrvT: FnRVV,
    pub c2Mulxv: FnXVV,
    pub c2BBVerts: FnBBVerts,
    pub c2MakeProxy: FnMakeProxy,
    pub c2GJKSimplexMetric: FnSimplexF,
    pub c22: FnSimplex,
    pub c23: FnSimplex,
    pub c2D: FnSimplexV,
    pub c2L: FnSimplexV,
    pub c2Support: FnSupport,
    pub c2Witness: FnWitness,
    pub c2GJK: FnGJK,
    pub c2AABBtoAABB: FnAABBtoAABB,
    pub c2AABBtoCapsule: FnAABBtoCapsule,
    pub c2CapsuletoCapsule: FnCapsuletoCapsule,
    pub c2CircletoCircle: FnCircletoCircle,
    pub c2CircletoAABB: FnCircletoAABB,
    pub c2CircletoCapsule: FnCircletoCapsule,
    pub c2Collided: FnCollided,
    pub reverse_collide: FnReverseCollide,
}

macro_rules! sym {
    ($lib:expr, $name:ident, $t:ty) => {{
        let name = concat!(stringify!($name), "\0");
        let s = unsafe { $lib.get::<$t>(name.as_bytes()) }
            .unwrap_or_else(|e| panic!("missing symbol {}: {}", stringify!($name), e));
        *s
    }};
}

impl Api {
    fn load(label: &'static str, path: &PathBuf) -> Api {
        let lib: &'static Library = Box::leak(Box::new(
            unsafe { Library::new(path) }
                .unwrap_or_else(|e| panic!("cannot dlopen {}: {}", path.display(), e)),
        ));
        Api {
            label,
            c2V: sym!(lib, c2V, FnFFV),
            c2Mulvs: sym!(lib, c2Mulvs, FnVsV),
            c2Maxv: sym!(lib, c2Maxv, FnVV),
            c2Minv: sym!(lib, c2Minv, FnVV),
            c2Clampv: sym!(lib, c2Clampv, FnVVVV),
            c2Sub: sym!(lib, c2Sub, FnVV),
            c2Add: sym!(lib, c2Add, FnVV),
            c2Dot: sym!(lib, c2Dot, FnVVf),
            c2Det2: sym!(lib, c2Det2, FnVVf),
            c2Len: sym!(lib, c2Len, FnVf),
            c2Neg: sym!(lib, c2Neg, FnV),
            c2Skew: sym!(lib, c2Skew, FnV),
            c2CCW90: sym!(lib, c2CCW90, FnV),
            c2Norm: sym!(lib, c2Norm, FnV),
            c2Div: sym!(lib, c2Div, FnVsV),
            c2RotIdentity: sym!(lib, c2RotIdentity, FnRotId),
            c2xIdentity: sym!(lib, c2xIdentity, FnXId),
            c2Mulrv: sym!(lib, c2Mulrv, FnRVV),
            c2MulrvT: sym!(lib, c2MulrvT, FnRVV),
            c2Mulxv: sym!(lib, c2Mulxv, FnXVV),
            c2BBVerts: sym!(lib, c2BBVerts, FnBBVerts),
            c2MakeProxy: sym!(lib, c2MakeProxy, FnMakeProxy),
            c2GJKSimplexMetric: sym!(lib, c2GJKSimplexMetric, FnSimplexF),
            c22: sym!(lib, c22, FnSimplex),
            c23: sym!(lib, c23, FnSimplex),
            c2D: sym!(lib, c2D, FnSimplexV),
            c2L: sym!(lib, c2L, FnSimplexV),
            c2Support: sym!(lib, c2Support, FnSupport),
            c2Witness: sym!(lib, c2Witness, FnWitness),
            c2GJK: sym!(lib, c2GJK, FnGJK),
            c2AABBtoAABB: sym!(lib, c2AABBtoAABB, FnAABBtoAABB),
            c2AABBtoCapsule: sym!(lib, c2AABBtoCapsule, FnAABBtoCapsule),
            c2CapsuletoCapsule: sym!(lib, c2CapsuletoCapsule, FnCapsuletoCapsule),
            c2CircletoCircle: sym!(lib, c2CircletoCircle, FnCircletoCircle),
            c2CircletoAABB: sym!(lib, c2CircletoAABB, FnCircletoAABB),
            c2CircletoCapsule: sym!(lib, c2CircletoCapsule, FnCircletoCapsule),
            c2Collided: sym!(lib, c2Collided, FnCollided),
            reverse_collide: sym!(lib, reverse_collide, FnReverseCollide),
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
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
    if !build.exists() {
        // Configure + build the C library on demand.
        let _ = std::fs::create_dir_all(&build);
        let ok = std::process::Command::new("cmake")
            .current_dir(&build)
            .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            let _ = std::process::Command::new("cmake")
                .current_dir(&build)
                .args(["--build", "."])
                .status();
        }
    }
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    found.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so found in {} - build it with cmake first",
            build.display()
        )
    })
}

/// `cargo test` does not emit the `cdylib` artifact (no test target links
/// against it), so the harness builds it explicitly.  A dedicated
/// `--target-dir` is used so the nested invocation does not contend for the
/// build lock held by the outer `cargo test`.
fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = manifest.join("target/dylib-under-test");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    let mut cmd = std::process::Command::new(&cargo);
    cmd.current_dir(&manifest)
        .arg("build")
        .arg("--lib")
        .arg("--target-dir")
        .arg(&target_dir);
    // Mirror the feature selection the test binary itself was built with.
    if option_env!("CARGO_TEST_NO_DEFAULT_FEATURES").is_some() {
        cmd.arg("--no-default-features");
    }
    if let Some(f) = option_env!("CARGO_TEST_FEATURES") {
        if !f.is_empty() {
            cmd.arg("--features").arg(f);
        }
    }
    let status = cmd.status();
    match status {
        Ok(s) if s.success() => {}
        other => panic!("nested `cargo build --lib` failed: {other:?}"),
    }

    let so = target_dir.join("debug/libreverse_collide_lib.so");
    if so.exists() {
        return so;
    }
    // Fall back to whatever an earlier `cargo build` may have produced.
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    for c in [
        deps.join("libreverse_collide_lib.so"),
        deps.parent()
            .unwrap_or(deps)
            .join("libreverse_collide_lib.so"),
    ] {
        if c.exists() {
            return c;
        }
    }
    panic!("libreverse_collide_lib.so not found (looked at {})", so.display());
}

pub fn apis() -> (&'static Api, &'static Api) {
    use std::sync::OnceLock;
    static PAIR: OnceLock<(Api, Api)> = OnceLock::new();
    let p = PAIR.get_or_init(|| {
        preload_libm();
        let (c_so, rust_so) = library_paths();
        let c = Api::load("C", &c_so);
        let r = Api::load("Rust", &rust_so);
        (c, r)
    });
    (&p.0, &p.1)
}

/// Resolved paths of the two shared objects under comparison, building them on
/// demand.
pub fn library_paths() -> (PathBuf, PathBuf) {
    use std::sync::OnceLock;
    static PATHS: OnceLock<(PathBuf, PathBuf)> = OnceLock::new();
    PATHS
        .get_or_init(|| (find_c_so(), find_rust_so()))
        .clone()
}

/// The C library is linked without `-lm`, so `sqrtf` is left undefined and is
/// expected to come from the process' global scope.  Force libm into the global
/// namespace before dlopen'ing it.
fn preload_libm() {
    use libloading::os::unix::{Library, RTLD_GLOBAL, RTLD_NOW};
    for name in ["libm.so.6", "libm.so"] {
        if let Ok(lib) = unsafe { Library::open(Some(name), RTLD_NOW | RTLD_GLOBAL) } {
            std::mem::forget(lib);
            return;
        }
    }
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Compares two `f32` for *bit* equality.  NaN payloads are required to match
/// as well, which is what "byte-identical" means.
#[track_caller]
pub fn eq_f32(c: f32, r: f32, ctx: &str) {
    if c.to_bits() != r.to_bits() {
        panic!(
            "f32 mismatch in {ctx}: C = {c:?} (0x{:08x}) vs Rust = {r:?} (0x{:08x})",
            c.to_bits(),
            r.to_bits()
        );
    }
}

#[track_caller]
pub fn eq_v(c: c2v, r: c2v, ctx: &str) {
    eq_f32(c.x, r.x, &format!("{ctx}.x"));
    eq_f32(c.y, r.y, &format!("{ctx}.y"));
}

#[track_caller]
pub fn eq_r(c: c2r, r: c2r, ctx: &str) {
    eq_f32(c.c, r.c, &format!("{ctx}.c"));
    eq_f32(c.s, r.s, &format!("{ctx}.s"));
}

#[track_caller]
pub fn eq_x(c: c2x, r: c2x, ctx: &str) {
    eq_v(c.p, r.p, &format!("{ctx}.p"));
    eq_r(c.r, r.r, &format!("{ctx}.r"));
}

#[track_caller]
pub fn eq_i(c: c_int, r: c_int, ctx: &str) {
    assert_eq!(c, r, "int mismatch in {ctx}: C = {c} vs Rust = {r}");
}

#[track_caller]
pub fn eq_sv(c: &c2sv, r: &c2sv, ctx: &str) {
    eq_v(c.sA, r.sA, &format!("{ctx}.sA"));
    eq_v(c.sB, r.sB, &format!("{ctx}.sB"));
    eq_v(c.p, r.p, &format!("{ctx}.p"));
    eq_f32(c.u, r.u, &format!("{ctx}.u"));
    eq_i(c.iA, r.iA, &format!("{ctx}.iA"));
    eq_i(c.iB, r.iB, &format!("{ctx}.iB"));
}

#[track_caller]
pub fn eq_simplex(c: &c2Simplex, r: &c2Simplex, ctx: &str) {
    eq_sv(&c.a, &r.a, &format!("{ctx}.a"));
    eq_sv(&c.b, &r.b, &format!("{ctx}.b"));
    eq_sv(&c.c, &r.c, &format!("{ctx}.c"));
    eq_sv(&c.d, &r.d, &format!("{ctx}.d"));
    eq_f32(c.div, r.div, &format!("{ctx}.div"));
    eq_i(c.count, r.count, &format!("{ctx}.count"));
}

#[track_caller]
pub fn eq_proxy(c: &c2Proxy, r: &c2Proxy, ctx: &str) {
    eq_f32(c.radius, r.radius, &format!("{ctx}.radius"));
    eq_i(c.count, r.count, &format!("{ctx}.count"));
    for i in 0..8 {
        eq_v(c.verts[i], r.verts[i], &format!("{ctx}.verts[{i}]"));
    }
}

#[track_caller]
pub fn eq_cache(c: &c2GJKCache, r: &c2GJKCache, ctx: &str) {
    eq_f32(c.metric, r.metric, &format!("{ctx}.metric"));
    eq_i(c.count, r.count, &format!("{ctx}.count"));
    for i in 0..3 {
        eq_i(c.iA[i], r.iA[i], &format!("{ctx}.iA[{i}]"));
        eq_i(c.iB[i], r.iB[i], &format!("{ctx}.iB[{i}]"));
    }
    eq_f32(c.div, r.div, &format!("{ctx}.div"));
}

// ---------------------------------------------------------------------------
// Deterministic pseudo-random input generation
// ---------------------------------------------------------------------------

/// Iteration-count multiplier, settable with `DIFF_ITER_SCALE` (default 1).
/// Lets the same suite run as a fast regression check or as a long soak.
pub fn scaled(n: u32) -> u32 {
    let s: u32 = std::env::var("DIFF_ITER_SCALE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    n.saturating_mul(s.max(1))
}

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

    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    /// Uniform in [-mag, mag], quantised so that values are exactly
    /// representable and reproducible.
    pub fn f32_range(&mut self, mag: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        (u * 2.0 - 1.0) * mag
    }

    /// A "nasty" float: mixes ordinary values with zeros, infinities, several
    /// *distinct* NaN payloads, denormals and huge magnitudes.
    ///
    /// Using more than one NaN payload matters: a single arithmetic operation
    /// returns the same quieted NaN whichever operand it prefers when both
    /// operands carry the same payload, so payload-order bugs only become
    /// observable once different payloads can meet.
    pub fn f32_nasty(&mut self) -> f32 {
        match self.below(24) {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::NAN,                        // 0x7fc00000
            5 => -f32::NAN,                       // 0xffc00000
            6 => f32::from_bits(0x7fc0_1234),     // quiet, custom payload
            7 => f32::from_bits(0xffca_5a5a),     // quiet, negative, payload
            8 => f32::from_bits(0x7f80_0001),     // signalling NaN
            9 => f32::from_bits(0xff80_dead),     // signalling, negative
            10 => f32::MIN_POSITIVE,
            11 => -f32::MIN_POSITIVE,
            12 => f32::from_bits(1),
            13 => f32::MAX,
            14 => f32::MIN,
            15 => f32::EPSILON,
            16 => 1.192_092_895_507_812_5e-7,
            17 => self.f32_range(1.0e30),
            18 => self.f32_range(1.0e-30),
            _ => self.f32_range(100.0),
        }
    }

    pub fn v(&mut self, mag: f32) -> c2v {
        c2v {
            x: self.f32_range(mag),
            y: self.f32_range(mag),
        }
    }

    pub fn v_nasty(&mut self) -> c2v {
        c2v {
            x: self.f32_nasty(),
            y: self.f32_nasty(),
        }
    }

    pub fn rot(&mut self) -> c2r {
        let ang = self.f32_range(4.0);
        c2r {
            c: ang.cos(),
            s: ang.sin(),
        }
    }

    pub fn xform(&mut self) -> c2x {
        c2x {
            p: self.v(50.0),
            r: self.rot(),
        }
    }

    pub fn circle(&mut self) -> c2Circle {
        c2Circle {
            p: self.v(60.0),
            r: self.f32_range(30.0).abs(),
        }
    }

    pub fn aabb(&mut self) -> c2AABB {
        let a = self.v(60.0);
        let b = self.v(60.0);
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

    /// Occasionally produces an inverted / degenerate box on purpose.
    pub fn aabb_any(&mut self) -> c2AABB {
        if self.below(4) == 0 {
            c2AABB {
                min: self.v(60.0),
                max: self.v(60.0),
            }
        } else {
            self.aabb()
        }
    }

    pub fn capsule(&mut self) -> c2Capsule {
        c2Capsule {
            a: self.v(60.0),
            b: self.v(60.0),
            r: self.f32_range(25.0).abs(),
        }
    }

    /// Sometimes produces a zero-length capsule (a == b), which makes
    /// `c2CircletoCapsule` divide by zero - the C behaviour must be matched.
    pub fn capsule_any(&mut self) -> c2Capsule {
        if self.below(8) == 0 {
            let a = self.v(60.0);
            c2Capsule {
                a,
                b: a,
                r: self.f32_range(25.0).abs(),
            }
        } else {
            self.capsule()
        }
    }

    pub fn sv(&mut self, mag: f32) -> c2sv {
        c2sv {
            sA: self.v(mag),
            sB: self.v(mag),
            p: self.v(mag),
            u: self.f32_range(mag),
            iA: self.below(4) as c_int,
            iB: self.below(4) as c_int,
        }
    }
}
