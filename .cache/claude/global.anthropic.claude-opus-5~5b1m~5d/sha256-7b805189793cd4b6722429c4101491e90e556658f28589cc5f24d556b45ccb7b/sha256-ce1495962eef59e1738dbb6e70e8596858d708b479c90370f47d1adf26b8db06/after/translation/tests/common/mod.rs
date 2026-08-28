//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and calls everything through
//! their exported C symbols. The Rust crate is never linked directly, so the
//! `#[no_mangle] extern "C"` wrappers are themselves under test.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI mirrors of the C structs (c_src/src/lib.c). None has padding, so raw
// byte comparison is meaningful for every one of them.
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

pub const FLT_MAX: f32 = 3.402_823_5e38;
pub const FLT_EPSILON: f32 = 1.192_092_9e-7;

// Layout guards: if these ever fire, byte comparison would be comparing
// padding and every test in the suite would be meaningless.
const _: () = {
    assert!(size_of::<c2v>() == 8);
    assert!(size_of::<c2r>() == 8);
    assert!(size_of::<c2x>() == 16);
    assert!(size_of::<c2Circle>() == 12);
    assert!(size_of::<c2AABB>() == 16);
    assert!(size_of::<c2Capsule>() == 20);
    assert!(size_of::<c2GJKCache>() == 36);
    assert!(size_of::<c2Proxy>() == 72);
    assert!(size_of::<c2sv>() == 36);
    assert!(size_of::<c2Simplex>() == 152);
};

// ---------------------------------------------------------------------------
// Function-pointer types for the 31 exported symbols
// ---------------------------------------------------------------------------

pub type F_c2V = unsafe extern "C" fn(f32, f32) -> c2v;
pub type F_c2Mulvs = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type F_c2Maxv = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type F_c2Minv = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type F_c2Clampv = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type F_c2Sub = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type F_c2Add = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type F_c2Dot = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type F_c2Det2 = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type F_c2Len = unsafe extern "C" fn(c2v) -> f32;
pub type F_c2RotIdentity = unsafe extern "C" fn() -> c2r;
pub type F_c2xIdentity = unsafe extern "C" fn() -> c2x;
pub type F_c2BBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
pub type F_c2MakeProxy = unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
pub type F_c2GJKSimplexMetric = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type F_c2Mulrv = unsafe extern "C" fn(c2r, c2v) -> c2v;
pub type F_c2MulrvT = unsafe extern "C" fn(c2r, c2v) -> c2v;
pub type F_c2Mulxv = unsafe extern "C" fn(c2x, c2v) -> c2v;
pub type F_c22 = unsafe extern "C" fn(*mut c2Simplex);
pub type F_c23 = unsafe extern "C" fn(*mut c2Simplex);
pub type F_c2Neg = unsafe extern "C" fn(c2v) -> c2v;
pub type F_c2Skew = unsafe extern "C" fn(c2v) -> c2v;
pub type F_c2CCW90 = unsafe extern "C" fn(c2v) -> c2v;
pub type F_c2D = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type F_c2Support = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
pub type F_c2Witness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
pub type F_c2Div = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type F_c2Norm = unsafe extern "C" fn(c2v) -> c2v;
pub type F_c2L = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
#[rustfmt::skip]
pub type F_c2GJK = unsafe extern "C" fn(
    *const c_void, c_int, *const c2x,
    *const c_void, c_int, *const c2x,
    *mut c2v, *mut c2v, c_int, *mut c_int, *mut c2GJKCache,
) -> f32;
#[rustfmt::skip]
pub type F_gjk = unsafe extern "C" fn(
    c_char, *mut c2v, *mut c2v,
    f32, f32, f32, f32, f32, f32, f32, f32, f32,
);

/// Every exported symbol of one shared object, resolved eagerly.
pub struct Api {
    pub name: &'static str,
    pub c2V: Symbol<'static, F_c2V>,
    pub c2Mulvs: Symbol<'static, F_c2Mulvs>,
    pub c2Maxv: Symbol<'static, F_c2Maxv>,
    pub c2Minv: Symbol<'static, F_c2Minv>,
    pub c2Clampv: Symbol<'static, F_c2Clampv>,
    pub c2Sub: Symbol<'static, F_c2Sub>,
    pub c2Add: Symbol<'static, F_c2Add>,
    pub c2Dot: Symbol<'static, F_c2Dot>,
    pub c2Det2: Symbol<'static, F_c2Det2>,
    pub c2Len: Symbol<'static, F_c2Len>,
    pub c2RotIdentity: Symbol<'static, F_c2RotIdentity>,
    pub c2xIdentity: Symbol<'static, F_c2xIdentity>,
    pub c2BBVerts: Symbol<'static, F_c2BBVerts>,
    pub c2MakeProxy: Symbol<'static, F_c2MakeProxy>,
    pub c2GJKSimplexMetric: Symbol<'static, F_c2GJKSimplexMetric>,
    pub c2Mulrv: Symbol<'static, F_c2Mulrv>,
    pub c2MulrvT: Symbol<'static, F_c2MulrvT>,
    pub c2Mulxv: Symbol<'static, F_c2Mulxv>,
    pub c22: Symbol<'static, F_c22>,
    pub c23: Symbol<'static, F_c23>,
    pub c2Neg: Symbol<'static, F_c2Neg>,
    pub c2Skew: Symbol<'static, F_c2Skew>,
    pub c2CCW90: Symbol<'static, F_c2CCW90>,
    pub c2D: Symbol<'static, F_c2D>,
    pub c2Support: Symbol<'static, F_c2Support>,
    pub c2Witness: Symbol<'static, F_c2Witness>,
    pub c2Div: Symbol<'static, F_c2Div>,
    pub c2Norm: Symbol<'static, F_c2Norm>,
    pub c2L: Symbol<'static, F_c2L>,
    pub c2GJK: Symbol<'static, F_c2GJK>,
    pub gjk: Symbol<'static, F_gjk>,
}

impl Api {
    fn load(name: &'static str, path: &Path) -> Api {
        let lib: &'static Library = Box::leak(Box::new(
            unsafe { Library::new(path) }
                .unwrap_or_else(|e| panic!("dlopen {} ({}) failed: {e}", path.display(), name)),
        ));
        macro_rules! sym {
            ($s:literal) => {
                unsafe { lib.get(concat!($s, "\0").as_bytes()) }
                    .unwrap_or_else(|e| panic!("{} is missing symbol {}: {e}", name, $s))
            };
        }
        Api {
            name,
            c2V: sym!("c2V"),
            c2Mulvs: sym!("c2Mulvs"),
            c2Maxv: sym!("c2Maxv"),
            c2Minv: sym!("c2Minv"),
            c2Clampv: sym!("c2Clampv"),
            c2Sub: sym!("c2Sub"),
            c2Add: sym!("c2Add"),
            c2Dot: sym!("c2Dot"),
            c2Det2: sym!("c2Det2"),
            c2Len: sym!("c2Len"),
            c2RotIdentity: sym!("c2RotIdentity"),
            c2xIdentity: sym!("c2xIdentity"),
            c2BBVerts: sym!("c2BBVerts"),
            c2MakeProxy: sym!("c2MakeProxy"),
            c2GJKSimplexMetric: sym!("c2GJKSimplexMetric"),
            c2Mulrv: sym!("c2Mulrv"),
            c2MulrvT: sym!("c2MulrvT"),
            c2Mulxv: sym!("c2Mulxv"),
            c22: sym!("c22"),
            c23: sym!("c23"),
            c2Neg: sym!("c2Neg"),
            c2Skew: sym!("c2Skew"),
            c2CCW90: sym!("c2CCW90"),
            c2D: sym!("c2D"),
            c2Support: sym!("c2Support"),
            c2Witness: sym!("c2Witness"),
            c2Div: sym!("c2Div"),
            c2Norm: sym!("c2Norm"),
            c2L: sym!("c2L"),
            c2GJK: sym!("c2GJK"),
            gjk: sym!("gjk"),
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<project>.so`. The CMake project name is derived from the
/// parent directory name, so glob rather than hard-code it.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("GJK_C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}\nBuild the C reference first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    match found.len() {
        0 => panic!("no .so found in {}", build.display()),
        _ => found.remove(0),
    }
}

/// The Rust `cdylib` under test.
///
/// IMPORTANT: `cargo test` does **not** rebuild the `cdylib`, because the
/// integration tests never link it — they `dlopen` it. Running the suite would
/// therefore happily test a **stale `.so`** and report a meaningless green.
/// (This is not hypothetical: it bit this very test suite once.)
///
/// To make `cargo test` correct on its own, the harness rebuilds the release
/// `cdylib` itself, exactly once per test binary, before resolving the path.
/// `cargo` has already released its build lock by the time test binaries run,
/// and any concurrent invocation blocks on that lock rather than racing.
///
/// Set `GJK_RUST_SO` to test a specific artifact (this also skips the rebuild),
/// or `GJK_NO_BUILD=1` to skip only the rebuild.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("GJK_RUST_SO") {
        return PathBuf::from(p);
    }
    let manifest = manifest_dir();
    let target = manifest.join("target");

    if std::env::var_os("GJK_NO_BUILD").is_none() {
        let out = std::process::Command::new(
            std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()),
        )
        .args(["build", "--release", "--lib"])
        .current_dir(&manifest)
        // Don't inherit the test run's RUSTFLAGS-ish env that could change the
        // fingerprint and cause a pointless rebuild loop.
        .env_remove("RUSTC_WRAPPER")
        .output();
        match out {
            Ok(o) if o.status.success() => {}
            Ok(o) => panic!(
                "`cargo build --release --lib` failed while preparing the \
                 library under test:\n{}",
                String::from_utf8_lossy(&o.stderr)
            ),
            Err(e) => panic!(
                "could not run cargo to rebuild the cdylib ({e}). \
                 Set GJK_NO_BUILD=1 and build it yourself, or set GJK_RUST_SO."
            ),
        }
    }

    let p = target.join("release/libgjk_lib.so");
    if p.exists() {
        return p;
    }
    let d = target.join("debug/libgjk_lib.so");
    if d.exists() {
        return d;
    }
    panic!(
        "libgjk_lib.so not found under {}. Run `cargo build --release` first.",
        target.display()
    );
}

static C_API: OnceLock<Api> = OnceLock::new();
static RUST_API: OnceLock<Api> = OnceLock::new();

pub fn c() -> &'static Api {
    C_API.get_or_init(|| Api::load("C", &c_so_path()))
}

pub fn r() -> &'static Api {
    RUST_API.get_or_init(|| Api::load("Rust", &rust_so_path()))
}

/// `(c_api, rust_api)`
pub fn both() -> (&'static Api, &'static Api) {
    (c(), r())
}

// ---------------------------------------------------------------------------
// Bit-exact comparison
// ---------------------------------------------------------------------------

pub fn bytes_of<T>(v: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v as *const T as *const u8, size_of::<T>()) }
}

pub fn hex<T>(v: &T) -> String {
    bytes_of(v).iter().map(|b| format!("{b:02x}")).collect()
}

#[track_caller]
pub fn assert_bits_eq<T>(ctx: &str, cv: &T, rv: &T) {
    if bytes_of(cv) != bytes_of(rv) {
        panic!(
            "MISMATCH [{ctx}]\n  C    = {}\n  Rust = {}\n  (raw bytes, little-endian)",
            hex(cv),
            hex(rv)
        );
    }
}

#[track_caller]
pub fn assert_f32_bits_eq(ctx: &str, cv: f32, rv: f32) {
    if cv.to_bits() != rv.to_bits() {
        panic!(
            "MISMATCH [{ctx}]\n  C    = {cv:?} (0x{:08x})\n  Rust = {rv:?} (0x{:08x})",
            cv.to_bits(),
            rv.to_bits()
        );
    }
}

#[track_caller]
pub fn assert_eq_ctx<T: PartialEq + std::fmt::Debug>(ctx: &str, cv: T, rv: T) {
    if cv != rv {
        panic!("MISMATCH [{ctx}]\n  C    = {cv:?}\n  Rust = {rv:?}");
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) + float value-class generator
// ---------------------------------------------------------------------------

pub struct Rng(u64);

/// The single seed for the whole suite; every test derives a stream from it so
/// failures are reproducible.
pub const SEED: u64 = 0x5eed_1234_c0ff_ee01;

impl Rng {
    pub fn new(stream: u64) -> Rng {
        Rng(SEED ^ stream.wrapping_mul(0x9E37_79B9_7F4A_7C15))
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
        self.next_u64() & 1 == 1
    }

    /// Uniform in `[-1, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() as f32 / 4_294_967_296.0) * 2.0 - 1.0
    }

    /// "Geometry-shaped" coordinate: mostly ordinary magnitudes, with exact
    /// halves and integers mixed in so that ties and exact-zero determinants
    /// actually occur.
    pub fn coord(&mut self) -> f32 {
        match self.below(10) {
            0..=4 => self.unit() * 10.0,
            5 => self.unit() * 0.001,
            6 => self.unit() * 1000.0,
            7 => (self.below(17) as f32) - 8.0,
            8 => ((self.below(33) as f32) - 16.0) * 0.5,
            _ => self.unit(),
        }
    }

    /// Non-negative radius-shaped value.
    pub fn radius(&mut self) -> f32 {
        match self.below(8) {
            0 => 0.0,
            1 => (self.below(9) as f32) * 0.5,
            2 => self.unit().abs() * 100.0,
            3 => FLT_EPSILON,
            4 => FLT_EPSILON * 0.5,
            _ => self.unit().abs() * 5.0,
        }
    }

    /// Any `f32` bit pattern at all, weighted so that the interesting classes
    /// (NaN payloads, both infinities, both zeros, subnormals, the exact
    /// constants the C source mentions) show up often.
    pub fn any_f32(&mut self) -> f32 {
        match self.below(16) {
            0..=3 => f32::from_bits(self.next_u32()),
            4..=6 => self.coord(),
            7 => self.nan(),
            8 => self.nan(),
            9 => {
                // subnormal
                let m = self.next_u32() & 0x007f_ffff;
                let s = (self.next_u32() & 1) << 31;
                f32::from_bits(s | m)
            }
            10 => *pick(
                &[
                    0.0f32,
                    -0.0f32,
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                    FLT_MAX,
                    -FLT_MAX,
                    FLT_EPSILON,
                    -FLT_EPSILON,
                    FLT_EPSILON * FLT_EPSILON,
                    -1.0e8f32,
                    1.0f32,
                    -1.0f32,
                    0.5f32,
                ],
                self,
            ),
            11 => {
                // huge but finite, so products overflow to inf
                let e = 200 + self.below(54);
                f32::from_bits(((self.next_u32() & 1) << 31) | (e << 23))
            }
            _ => self.coord(),
        }
    }

    /// A NaN with a random payload and random sign, quiet or signalling.
    pub fn nan(&mut self) -> f32 {
        let payload = self.next_u32() & 0x003f_ffff;
        let sign = (self.next_u32() & 1) << 31;
        let quiet = if self.bool() { 0x0040_0000 } else { 0 };
        // A zero payload with quiet=0 would be infinity, not NaN.
        let payload = if payload == 0 && quiet == 0 { 1 } else { payload };
        f32::from_bits(sign | 0x7f80_0000 | quiet | payload)
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

    pub fn any_r(&mut self) -> c2r {
        c2r {
            c: self.any_f32(),
            s: self.any_f32(),
        }
    }

    /// A rotation: usually a real unit rotation, sometimes not normalised at
    /// all (the C never checks).
    pub fn rot(&mut self) -> c2r {
        match self.below(6) {
            0 => c2r { c: 1.0, s: 0.0 },
            1 => c2r { c: 0.0, s: 1.0 },
            2 => c2r { c: -1.0, s: 0.0 },
            3 => c2r {
                c: self.coord(),
                s: self.coord(),
            },
            _ => {
                let a = self.unit() * std::f32::consts::PI;
                c2r {
                    c: a.cos(),
                    s: a.sin(),
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

    pub fn aabb(&mut self) -> c2AABB {
        match self.below(8) {
            0 => {
                // degenerate: min == max
                let p = self.v();
                c2AABB { min: p, max: p }
            }
            1 => {
                // inverted: min > max (the C never validates)
                let a = self.v();
                let b = c2v {
                    x: a.x - self.radius(),
                    y: a.y - self.radius(),
                };
                c2AABB { min: a, max: b }
            }
            _ => {
                let a = self.v();
                let b = c2v {
                    x: a.x + self.radius(),
                    y: a.y + self.radius(),
                };
                c2AABB { min: a, max: b }
            }
        }
    }

    pub fn circle(&mut self) -> c2Circle {
        c2Circle {
            p: self.v(),
            r: self.radius(),
        }
    }

    pub fn capsule(&mut self) -> c2Capsule {
        let a = self.v();
        let b = if self.below(8) == 0 {
            a // degenerate: zero-length capsule
        } else {
            self.v()
        };
        c2Capsule {
            a,
            b,
            r: self.radius(),
        }
    }

    /// A `c2sv` with random support points and indices.
    pub fn sv(&mut self) -> c2sv {
        c2sv {
            sA: self.v(),
            sB: self.v(),
            p: self.v(),
            u: self.coord(),
            iA: self.below(4) as c_int,
            iB: self.below(4) as c_int,
        }
    }
}

pub fn pick<'a, T>(xs: &'a [T], rng: &mut Rng) -> &'a T {
    &xs[rng.below(xs.len() as u32) as usize]
}

// ---------------------------------------------------------------------------
// Shape plumbing shared by the c2GJK tests
// ---------------------------------------------------------------------------

/// One of the three shapes, kept alive so a `*const c_void` to it stays valid.
#[derive(Copy, Clone, Debug)]
pub enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

impl Shape {
    pub fn type_id(&self) -> c_int {
        match self {
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
        }
    }

    pub fn as_ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(c) => c as *const _ as *const c_void,
            Shape::Aabb(c) => c as *const _ as *const c_void,
            Shape::Capsule(c) => c as *const _ as *const c_void,
        }
    }

    /// Number of proxy vertices `c2MakeProxy` writes for this shape — the
    /// in-range index bound for a warm `c2GJKCache`.
    pub fn vert_count(&self) -> c_int {
        match self {
            Shape::Circle(_) => 1,
            Shape::Aabb(_) => 4,
            Shape::Capsule(_) => 2,
        }
    }

    /// A random shape of a random type (avoids a double `&mut rng` borrow at
    /// the call site).
    pub fn any(rng: &mut Rng) -> Shape {
        let which = ALL_TYPES[rng.below(3) as usize];
        Shape::random(rng, which)
    }

    pub fn random(rng: &mut Rng, which: c_int) -> Shape {
        match which {
            C2_TYPE_CIRCLE => Shape::Circle(rng.circle()),
            C2_TYPE_AABB => Shape::Aabb(rng.aabb()),
            _ => Shape::Capsule(rng.capsule()),
        }
    }
}

pub const ALL_TYPES: [c_int; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

/// Everything `c2GJK` can observably produce.
#[derive(Debug)]
pub struct GjkOut {
    pub dist: f32,
    pub a: c2v,
    pub b: c2v,
    pub iters: c_int,
    pub cache: c2GJKCache,
    /// Bytes of the `outA`/`outB`/`iterations` buffers as the callee left them,
    /// so "was not written" is observable when the pointer is NULL.
    pub wrote_a: bool,
    pub wrote_b: bool,
    pub wrote_iters: bool,
}

/// Invocation options for `c2GJK`, mirroring the axes in `CONFIGS.md`.
#[derive(Copy, Clone, Debug)]
pub struct GjkOpts {
    pub ax: Option<c2x>,
    pub bx: Option<c2x>,
    pub use_radius: c_int,
    /// `None` -> pass NULL; `Some(c)` -> pass a cache initialised to `c`.
    pub cache: Option<c2GJKCache>,
    pub want_a: bool,
    pub want_b: bool,
    pub want_iters: bool,
}

impl Default for GjkOpts {
    fn default() -> Self {
        GjkOpts {
            ax: None,
            bx: None,
            use_radius: 1,
            cache: None,
            want_a: true,
            want_b: true,
            want_iters: true,
        }
    }
}

/// Sentinel bit patterns so "the callee never wrote here" is detectable.
const SENTINEL_V: c2v = c2v {
    x: -1.234_567_8e-11,
    y: 9.876_543e-13,
};
const SENTINEL_I: c_int = -0x5EED_BEEF;

pub fn call_gjk(api: &Api, a: &Shape, b: &Shape, o: &GjkOpts) -> GjkOut {
    let mut out_a = SENTINEL_V;
    let mut out_b = SENTINEL_V;
    let mut iters: c_int = SENTINEL_I;
    let mut cache = o.cache.unwrap_or_default();

    let ax_ptr = o.ax.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
    let bx_ptr = o.bx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);

    let dist = unsafe {
        (api.c2GJK)(
            a.as_ptr(),
            a.type_id(),
            ax_ptr,
            b.as_ptr(),
            b.type_id(),
            bx_ptr,
            if o.want_a { &mut out_a } else { std::ptr::null_mut() },
            if o.want_b { &mut out_b } else { std::ptr::null_mut() },
            o.use_radius,
            if o.want_iters {
                &mut iters
            } else {
                std::ptr::null_mut()
            },
            if o.cache.is_some() {
                &mut cache
            } else {
                std::ptr::null_mut()
            },
        )
    };

    GjkOut {
        dist,
        a: out_a,
        b: out_b,
        iters,
        cache,
        wrote_a: bytes_of(&out_a) != bytes_of(&SENTINEL_V),
        wrote_b: bytes_of(&out_b) != bytes_of(&SENTINEL_V),
        wrote_iters: iters != SENTINEL_I,
    }
}

#[track_caller]
pub fn assert_gjk_eq(ctx: &str, cv: &GjkOut, rv: &GjkOut) {
    assert_f32_bits_eq(&format!("{ctx} / dist"), cv.dist, rv.dist);
    assert_bits_eq(&format!("{ctx} / outA"), &cv.a, &rv.a);
    assert_bits_eq(&format!("{ctx} / outB"), &cv.b, &rv.b);
    assert_eq_ctx(&format!("{ctx} / iterations"), cv.iters, rv.iters);
    assert_bits_eq(&format!("{ctx} / cache"), &cv.cache, &rv.cache);
    assert_eq_ctx(&format!("{ctx} / wrote_a"), cv.wrote_a, rv.wrote_a);
    assert_eq_ctx(&format!("{ctx} / wrote_b"), cv.wrote_b, rv.wrote_b);
    assert_eq_ctx(
        &format!("{ctx} / wrote_iters"),
        cv.wrote_iters,
        rv.wrote_iters,
    );
}

/// Build a `c2Simplex` from `count` vertices, leaving the rest zeroed.
pub fn simplex(count: c_int, div: f32, vs: &[c2sv]) -> c2Simplex {
    let mut s = c2Simplex::default();
    for (i, v) in vs.iter().take(4).enumerate() {
        s.verts[i] = *v;
    }
    s.div = div;
    s.count = count;
    s
}

/// Run `f` against both libraries with an identical fresh copy of `s` and
/// compare the full 152-byte simplex afterwards as well as the return value.
#[track_caller]
pub fn diff_simplex<R, F>(ctx: &str, s: &c2Simplex, f: F) -> (R, R)
where
    F: Fn(&Api, *mut c2Simplex) -> R,
    R: Copy,
{
    let (ca, ra) = both();
    let mut cs = *s;
    let mut rs = *s;
    let cr = f(ca, &mut cs);
    let rr = f(ra, &mut rs);
    assert_bits_eq(&format!("{ctx} / simplex state"), &cs, &rs);
    (cr, rr)
}
