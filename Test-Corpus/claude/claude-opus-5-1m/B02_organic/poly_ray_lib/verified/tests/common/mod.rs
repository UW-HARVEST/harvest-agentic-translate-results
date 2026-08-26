//! Shared differential-test harness.
//!
//! Both the C shared library and the Rust `cdylib` are loaded with
//! `libloading` and driven **only** through their exported C symbols, exactly
//! as an external consumer would.  No Rust function is ever called directly.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::c_void;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI-compatible mirrors of the C types (c_src/include/lib.h + c_src/src/lib.c)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct C2Raycast {
    pub t: f32,
    pub n: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct C2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct C2x {
    pub p: C2v,
    pub r: C2r,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct C2AABB {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2Poly {
    pub count: c_int,
    pub verts: [C2v; 8],
    pub norms: [C2v; 8],
}

impl Default for C2Poly {
    fn default() -> Self {
        C2Poly {
            count: 0,
            verts: [C2v::default(); 8],
            norms: [C2v::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct C2Ray {
    pub p: C2v,
    pub d: C2v,
    pub t: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct C2m {
    pub x: C2v,
    pub y: C2v,
}

pub const C2_TYPE_CIRCLE: u32 = 0;
pub const C2_TYPE_AABB: u32 = 1;
pub const C2_TYPE_CAPSULE: u32 = 2;
pub const C2_TYPE_POLY: u32 = 3;

pub fn v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Bit-for-bit float equality, with one *documented* relaxation.
///
/// Strict about everything that is architecturally determined:
/// `+0.0` vs `-0.0`, `+inf` vs `-inf`, every finite value, and *whether* a
/// result is NaN at all.
///
/// Relaxed about the **payload / sign bit of a NaN result**: when an operation
/// has two NaN operands, x86 SSE returns the NaN of the *destination* register,
/// so the surviving payload is decided purely by register allocation. GCC at
/// `-O0` and LLVM pick different orders even inside the same expression — e.g.
/// for `a.x*b.x + a.y*b.y` GCC emits `mulss %xmm0,%xmm1` (dst = `a.x`) for the
/// first product but `mulss %xmm2,%xmm0` (dst = `b.y`) for the second, then
/// `addss %xmm1,%xmm0` (dst = the *second* product). Neither IEEE-754 nor C nor
/// Rust specifies which payload wins, so requiring identical NaN bit patterns
/// would be asserting on the C compiler's register allocator rather than on the
/// translation. `both NaN` is therefore treated as equal.
pub fn feq(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

/// Fully strict bit comparison — used by the informational NaN-payload report.
pub fn feq_strict(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

pub fn fshow(a: f32) -> String {
    format!("{a:?} (0x{:08x})", a.to_bits())
}

pub fn veq(a: C2v, b: C2v) -> bool {
    feq(a.x, b.x) && feq(a.y, b.y)
}

pub fn vshow(a: C2v) -> String {
    format!("({}, {})", fshow(a.x), fshow(a.y))
}

pub fn rceq(a: C2Raycast, b: C2Raycast) -> bool {
    feq(a.t, b.t) && veq(a.n, b.n)
}

pub fn rcshow(a: C2Raycast) -> String {
    format!("{{ t: {}, n: {} }}", fshow(a.t), vshow(a.n))
}

/// A poisoned `c2Raycast` used to detect "the callee did not write to `*out`".
pub fn poison(seed: u32) -> C2Raycast {
    C2Raycast {
        t: f32::from_bits(0x7f80_0001 ^ seed),
        n: C2v {
            x: f32::from_bits(0xdead_beef ^ seed),
            y: f32::from_bits(0x0bad_f00d ^ seed),
        },
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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

    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    /// Uniform in `[-mag, mag]`, always finite.
    pub fn unit(&mut self, mag: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
        (u * 2.0 - 1.0) * mag
    }

    /// "Geometry friendly" float: mostly small finite values, with a sprinkle
    /// of exact integers/halves so that boundary comparisons are actually hit.
    pub fn geom(&mut self) -> f32 {
        match self.below(10) {
            0 => 0.0,
            1 => -0.0,
            2 => self.below(21) as f32 - 10.0,             // exact integers
            3 => (self.below(41) as f32 - 20.0) * 0.5,     // exact halves
            _ => self.unit(20.0),
        }
    }

    /// Full-spectrum float, including `±0.0`, subnormals, `±inf`, NaN,
    /// `±FLT_MAX` and completely random bit patterns.
    pub fn wild(&mut self) -> f32 {
        match self.below(20) {
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
            10 => f32::from_bits(0x0000_0001), // smallest subnormal
            11 => f32::from_bits(0x8000_0001),
            12 => 1.0,
            13 => -1.0,
            14 => f32::from_bits(self.next_u32()), // anything at all
            15 => f32::from_bits(self.next_u32()),
            _ => self.unit(1.0e6),
        }
    }

    pub fn geom_v(&mut self) -> C2v {
        C2v {
            x: self.geom(),
            y: self.geom(),
        }
    }

    pub fn wild_v(&mut self) -> C2v {
        C2v {
            x: self.wild(),
            y: self.wild(),
        }
    }
}

/// The 16 "special" float values used for exhaustive cross-products.
pub const SPECIALS: [f32; 16] = [
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    f32::MAX,
    f32::MIN,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    3.402_823_4e38,
    1.175_494_3e-38,
];

/// Subnormals & other odd bit patterns as raw bits.
pub const SPECIAL_BITS: [u32; 10] = [
    0x0000_0000,
    0x8000_0000,
    0x0000_0001,
    0x8000_0001,
    0x007f_ffff,
    0x7f7f_ffff,
    0x7f80_0000,
    0xff80_0000,
    0x7fc0_0000,
    0xffc0_0000,
];

// ---------------------------------------------------------------------------
// Function-pointer table loaded from a shared object
// ---------------------------------------------------------------------------

pub type FnV = unsafe extern "C" fn(f32, f32) -> C2v;
pub type FnVV_F = unsafe extern "C" fn(C2v, C2v) -> f32;
pub type FnV_F = unsafe extern "C" fn(C2v) -> f32;
pub type FnVV_V = unsafe extern "C" fn(C2v, C2v) -> C2v;
pub type FnVF_V = unsafe extern "C" fn(C2v, f32) -> C2v;
pub type FnV_V = unsafe extern "C" fn(C2v) -> C2v;
pub type FnMV_V = unsafe extern "C" fn(C2m, C2v) -> C2v;
pub type Fn_R = unsafe extern "C" fn() -> C2r;
pub type Fn_X = unsafe extern "C" fn() -> C2x;
pub type FnRV_V = unsafe extern "C" fn(C2r, C2v) -> C2v;
pub type FnXV_V = unsafe extern "C" fn(C2x, C2v) -> C2v;
pub type FnBB_I = unsafe extern "C" fn(C2AABB, C2AABB) -> c_int;
pub type FnBV_I = unsafe extern "C" fn(C2AABB, C2v) -> c_int;
pub type FnCV_I = unsafe extern "C" fn(C2Circle, C2v) -> c_int;
pub type FnRayCircle = unsafe extern "C" fn(C2Ray, C2Circle, *mut C2Raycast) -> c_int;
pub type FnRayAabb = unsafe extern "C" fn(C2Ray, C2AABB, *mut C2Raycast) -> c_int;
pub type FnRayCapsule = unsafe extern "C" fn(C2Ray, C2Capsule, *mut C2Raycast) -> c_int;
pub type FnRayPoly =
    unsafe extern "C" fn(C2Ray, *const C2Poly, *const C2x, *mut C2Raycast) -> c_int;
pub type FnCastRay =
    unsafe extern "C" fn(C2Ray, *const c_void, *const C2x, u32, *mut C2Raycast) -> c_int;
pub type FnPolyRay = unsafe extern "C" fn(*mut C2Raycast, *mut C2Raycast) -> c_int;

/// Every exported symbol of the library under test.
pub struct Api {
    pub name: &'static str,
    _lib: libloading::Library,

    pub c2V: FnV,
    pub c2Dot: FnVV_F,
    pub c2Len: FnV_F,
    pub c2Add: FnVV_V,
    pub c2Sub: FnVV_V,
    pub c2Mulvs: FnVF_V,
    pub c2Div: FnVF_V,
    pub c2Norm: FnV_V,
    pub c2Minv: FnVV_V,
    pub c2Maxv: FnVV_V,
    pub c2Skew: FnV_V,
    pub c2Absv: FnV_V,
    pub c2CCW90: FnV_V,
    pub c2MulmvT: FnMV_V,
    pub c2RotIdentity: Fn_R,
    pub c2xIdentity: Fn_X,
    pub c2Mulrv: FnRV_V,
    pub c2MulrvT: FnRV_V,
    pub c2MulxvT: FnXV_V,
    pub c2AABBtoAABB: FnBB_I,
    pub c2AABBtoPoint: FnBV_I,
    pub c2CircleToPoint: FnCV_I,
    pub c2RaytoCircle: FnRayCircle,
    pub c2RaytoAABB: FnRayAabb,
    pub c2RaytoCapsule: FnRayCapsule,
    pub c2RaytoPoly: FnRayPoly,
    pub c2CastRay: FnCastRay,
    pub poly_ray: FnPolyRay,
}

macro_rules! sym {
    ($lib:expr, $ty:ty, $name:literal) => {{
        let s: libloading::Symbol<$ty> = unsafe { $lib.get(concat!($name, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("missing symbol `{}`: {e}", $name));
        *s
    }};
}

impl Api {
    fn load(name: &'static str, path: &Path) -> Api {
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display()));
        let a = Api {
            name,
            c2V: sym!(lib, FnV, "c2V"),
            c2Dot: sym!(lib, FnVV_F, "c2Dot"),
            c2Len: sym!(lib, FnV_F, "c2Len"),
            c2Add: sym!(lib, FnVV_V, "c2Add"),
            c2Sub: sym!(lib, FnVV_V, "c2Sub"),
            c2Mulvs: sym!(lib, FnVF_V, "c2Mulvs"),
            c2Div: sym!(lib, FnVF_V, "c2Div"),
            c2Norm: sym!(lib, FnV_V, "c2Norm"),
            c2Minv: sym!(lib, FnVV_V, "c2Minv"),
            c2Maxv: sym!(lib, FnVV_V, "c2Maxv"),
            c2Skew: sym!(lib, FnV_V, "c2Skew"),
            c2Absv: sym!(lib, FnV_V, "c2Absv"),
            c2CCW90: sym!(lib, FnV_V, "c2CCW90"),
            c2MulmvT: sym!(lib, FnMV_V, "c2MulmvT"),
            c2RotIdentity: sym!(lib, Fn_R, "c2RotIdentity"),
            c2xIdentity: sym!(lib, Fn_X, "c2xIdentity"),
            c2Mulrv: sym!(lib, FnRV_V, "c2Mulrv"),
            c2MulrvT: sym!(lib, FnRV_V, "c2MulrvT"),
            c2MulxvT: sym!(lib, FnXV_V, "c2MulxvT"),
            c2AABBtoAABB: sym!(lib, FnBB_I, "c2AABBtoAABB"),
            c2AABBtoPoint: sym!(lib, FnBV_I, "c2AABBtoPoint"),
            c2CircleToPoint: sym!(lib, FnCV_I, "c2CircleToPoint"),
            c2RaytoCircle: sym!(lib, FnRayCircle, "c2RaytoCircle"),
            c2RaytoAABB: sym!(lib, FnRayAabb, "c2RaytoAABB"),
            c2RaytoCapsule: sym!(lib, FnRayCapsule, "c2RaytoCapsule"),
            c2RaytoPoly: sym!(lib, FnRayPoly, "c2RaytoPoly"),
            c2CastRay: sym!(lib, FnCastRay, "c2CastRay"),
            poly_ray: sym!(lib, FnPolyRay, "poly_ray"),
            _lib: lib,
        };
        a
    }
}

// ---------------------------------------------------------------------------
// Locating / building the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` directory of the currently running test binary.
fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-bin>
    exe.parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .expect("target profile dir")
}

fn c_so_path() -> PathBuf {
    // Escape hatch so the suite can be re-run against a C library built with
    // different CFLAGS (e.g. `-O2`) without touching `c_src/`.
    if let Ok(p) = std::env::var("DIFFTEST_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "DIFFTEST_C_SO={} is not a file", p.display());
        return p;
    }
    let base = manifest_dir().join("c_src/build");
    for cand in [
        base.join("libtranslated_rust.so"),
        base.join("libc_src.so"),
    ] {
        if cand.is_file() {
            return cand;
        }
    }
    // Any .so in the cmake build dir.
    if let Ok(rd) = std::fs::read_dir(&base) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                return p;
            }
        }
    }
    build_c_so()
}

fn build_c_so() -> PathBuf {
    use std::process::Command;
    let c_src = manifest_dir().join("c_src");
    let build = c_src.join("build");
    std::fs::create_dir_all(&build).expect("mkdir c_src/build");
    let ok = Command::new("cmake")
        .current_dir(&build)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        && Command::new("cmake")
            .current_dir(&build)
            .args(["--build", "."])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    assert!(ok, "failed to build the C shared library in {}", build.display());
    let mut found = None;
    for e in std::fs::read_dir(&build).expect("read c_src/build").flatten() {
        let p = e.path();
        if p.extension().map(|x| x == "so").unwrap_or(false) {
            found = Some(p);
        }
    }
    found.expect("no .so produced by cmake")
}

fn rust_so_path() -> PathBuf {
    let dir = target_profile_dir();
    let cand = dir.join("libpoly_ray_lib.so");
    if cand.is_file() {
        return cand;
    }
    build_rust_so()
}

/// `cargo test` does **not** build a `cdylib`-only lib target, so build it
/// explicitly into a side target directory (avoids the outer build lock).
fn build_rust_so() -> PathBuf {
    use std::process::Command;
    let root = manifest_dir();
    let side = root.join("target").join("difftest-cdylib");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let profile_dir = target_profile_dir();
    let release = profile_dir.file_name().map(|n| n == "release").unwrap_or(false);
    let mut cmd = Command::new(cargo);
    cmd.current_dir(&root)
        .env("CARGO_TARGET_DIR", &side)
        .arg("build")
        .arg("--lib");
    if release {
        cmd.arg("--release");
    }
    let st = cmd.status().expect("spawn cargo build --lib");
    assert!(st.success(), "cargo build --lib failed");
    let out = side
        .join(if release { "release" } else { "debug" })
        .join("libpoly_ray_lib.so");
    assert!(
        out.is_file(),
        "expected the Rust cdylib at {}",
        out.display()
    );
    out
}

/// The C `.so` imports `sqrtf`, which on glibc >= 2.34 lives *only* in
/// `libm.so.6`.  Rust links `-lm` with `--as-needed`, so a test binary that
/// happens not to reference any libm symbol itself does not pull `libm.so.6`
/// in, and `dlopen`ing the C library then fails with
/// `undefined symbol: sqrtf`.  Load libm explicitly with `RTLD_GLOBAL` first so
/// its symbols are visible to every subsequently loaded object.
static LIBM: OnceLock<Option<libloading::os::unix::Library>> = OnceLock::new();

fn ensure_libm() {
    LIBM.get_or_init(|| {
        use libloading::os::unix::{Library, RTLD_GLOBAL, RTLD_NOW};
        for name in ["libm.so.6", "libm.so"] {
            if let Ok(l) = unsafe { Library::open(Some(name), RTLD_NOW | RTLD_GLOBAL) } {
                return Some(l);
            }
        }
        // Fall back to promoting the already-loaded global scope; if `sqrtf`
        // still cannot be found, `Api::load` reports a clear error.
        None
    });
}

static SO_PATHS: OnceLock<(PathBuf, PathBuf)> = OnceLock::new();

/// `(path to the C .so, path to the Rust cdylib)`, building them if needed.
pub fn so_paths() -> &'static (PathBuf, PathBuf) {
    SO_PATHS.get_or_init(|| (c_so_path(), rust_so_path()))
}

static APIS: OnceLock<(Api, Api)> = OnceLock::new();

/// `(c_api, rust_api)` — both loaded through `dlopen`.
pub fn apis() -> &'static (Api, Api) {
    APIS.get_or_init(|| {
        ensure_libm();
        let (c, r) = so_paths();
        (Api::load("C", c), Api::load("RUST", r))
    })
}

pub fn c() -> &'static Api {
    &apis().0
}

pub fn rs() -> &'static Api {
    &apis().1
}
