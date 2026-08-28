//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and exposes their exported
//! symbols as plain `extern "C"` function pointers, so every call in every test
//! crosses a real FFI boundary (this is what exercises the Rust crate's
//! `#[no_mangle]` export wrappers, struct-return ABI, and argument classing).
//!
//! Nothing in these tests calls a Rust function from the crate directly.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Types — byte-for-byte copies of the C declarations (include/lib.h, src/lib.c)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Raycast {
    pub t: f32,
    pub n: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2Poly {
    pub count: c_int,
    pub verts: [c2v; 8],
    pub norms: [c2v; 8],
}

impl Default for c2Poly {
    fn default() -> Self {
        c2Poly {
            count: 0,
            verts: [c2v::default(); 8],
            norms: [c2v::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Ray {
    pub p: c2v,
    pub d: c2v,
    pub t: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2m {
    pub x: c2v,
    pub y: c2v,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;
pub const C2_TYPE_POLY: c_int = 3;

// ---------------------------------------------------------------------------
// Function-pointer types, one per exported C symbol
// ---------------------------------------------------------------------------

pub type FnVff = extern "C" fn(f32, f32) -> c2v;
pub type FnVvv = extern "C" fn(c2v, c2v) -> c2v;
pub type FnFvv = extern "C" fn(c2v, c2v) -> f32;
pub type FnFv = extern "C" fn(c2v) -> f32;
pub type FnVv = extern "C" fn(c2v) -> c2v;
pub type FnVvf = extern "C" fn(c2v, f32) -> c2v;
pub type FnVmv = extern "C" fn(c2m, c2v) -> c2v;
pub type FnVrv = extern "C" fn(c2r, c2v) -> c2v;
pub type FnVxv = extern "C" fn(c2x, c2v) -> c2v;
pub type FnR = extern "C" fn() -> c2r;
pub type FnX = extern "C" fn() -> c2x;
pub type FnIaabbaabb = extern "C" fn(c2AABB, c2AABB) -> c_int;
pub type FnIaabbv = extern "C" fn(c2AABB, c2v) -> c_int;
pub type FnIcirclev = extern "C" fn(c2Circle, c2v) -> c_int;
pub type FnRayCircle = unsafe extern "C" fn(c2Ray, c2Circle, *mut c2Raycast) -> c_int;
pub type FnRayAABB = unsafe extern "C" fn(c2Ray, c2AABB, *mut c2Raycast) -> c_int;
pub type FnRayCapsule = unsafe extern "C" fn(c2Ray, c2Capsule, *mut c2Raycast) -> c_int;
pub type FnRayPoly = unsafe extern "C" fn(c2Ray, *const c2Poly, *const c2x, *mut c2Raycast) -> c_int;
pub type FnCastRay =
    unsafe extern "C" fn(c2Ray, *const c_void, *const c2x, c_int, *mut c2Raycast) -> c_int;
pub type FnPolyRay = unsafe extern "C" fn(*mut c2Raycast, *mut c2Raycast) -> c_int;

macro_rules! declare_api {
    ( $( $name:ident : $ty:ty ),* $(,)? ) => {
        /// All 28 exported symbols of one shared object, resolved via `dlsym`.
        pub struct Api {
            /// Human-readable tag ("C" or "RUST") used in assertion messages.
            pub tag: &'static str,
            $( pub $name: $ty, )*
        }

        impl Api {
            unsafe fn load(tag: &'static str, lib: &'static Library) -> Api {
                Api {
                    tag,
                    $( $name: *lib
                        .get::<$ty>(concat!(stringify!($name), "\0").as_bytes())
                        .unwrap_or_else(|e| panic!(
                            "{}: missing symbol `{}`: {e}", tag, stringify!($name))), )*
                }
            }

            /// The list of symbol names this harness requires.
            pub const SYMBOLS: &'static [&'static str] = &[ $( stringify!($name), )* ];
        }
    };
}

declare_api! {
    c2V: FnVff,
    c2Dot: FnFvv,
    c2Len: FnFv,
    c2Add: FnVvv,
    c2Sub: FnVvv,
    c2Mulvs: FnVvf,
    c2Div: FnVvf,
    c2Norm: FnVv,
    c2Minv: FnVvv,
    c2Maxv: FnVvv,
    c2Skew: FnVv,
    c2Absv: FnVv,
    c2CCW90: FnVv,
    c2MulmvT: FnVmv,
    c2RotIdentity: FnR,
    c2xIdentity: FnX,
    c2Mulrv: FnVrv,
    c2MulrvT: FnVrv,
    c2MulxvT: FnVxv,
    c2AABBtoAABB: FnIaabbaabb,
    c2AABBtoPoint: FnIaabbv,
    c2CircleToPoint: FnIcirclev,
    c2RaytoCircle: FnRayCircle,
    c2RaytoAABB: FnRayAABB,
    c2RaytoCapsule: FnRayCapsule,
    c2RaytoPoly: FnRayPoly,
    c2CastRay: FnCastRay,
    poly_ray: FnPolyRay,
}

// ---------------------------------------------------------------------------
// Locating and loading the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn first_so_in(dir: &Path, must_contain: Option<&str>) -> Option<PathBuf> {
    let mut hits: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .filter(|p| match must_contain {
            None => true,
            Some(sub) => p.file_name().unwrap().to_string_lossy().contains(sub),
        })
        .collect();
    hits.sort();
    hits.into_iter().next()
}

/// The C `.so`. Built by `cmake` into `c_src/build/`; the library name is
/// derived from the parent directory name by `c_src/CMakeLists.txt`, so glob
/// for it rather than hard-coding.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C2_C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().parent().unwrap().join("c_src/build");
    first_so_in(&build, None).unwrap_or_else(|| {
        panic!(
            "no C .so found in {}.\nBuild it first:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// The Rust `cdylib` under test.
///
/// IMPORTANT: `cargo test` does **not** build a `crate-type = ["cdylib"]` lib
/// target, because an integration test cannot link against a cdylib. Left to
/// itself, this harness would happily `dlopen` a `.so` left over from some
/// earlier `cargo build` and report a green run for stale code. So build it
/// here, explicitly, into a *separate* `--target-dir` (a separate dir means a
/// separate cargo lock file, so this nested invocation cannot deadlock against
/// the outer `cargo test` that is holding `target/`'s lock).
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C2_RUST_SO") {
        return PathBuf::from(p);
    }

    let manifest = manifest_dir();
    let target_dir = manifest.join("target/so-under-test");
    let profile = std::env::var("C2_SO_PROFILE").unwrap_or_else(|_| "release".to_string());

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = Command::new(cargo);
    cmd.arg("build")
        .arg("--lib")
        .arg("--manifest-path")
        .arg(manifest.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target_dir);
    if profile == "release" {
        cmd.arg("--release");
    }
    // Forward the feature selection the outer `cargo test` was run with, so the
    // `.so` under test is built with the same cfg as the tests expect.
    if let Ok(feats) = std::env::var("C2_FEATURES") {
        if feats == "--no-default-features" {
            cmd.arg("--no-default-features");
        } else if !feats.is_empty() {
            cmd.arg("--no-default-features").arg("--features").arg(feats);
        }
    }
    // Don't let the outer test build's RUSTFLAGS/incremental settings leak in.
    cmd.env_remove("RUSTC_WRAPPER");

    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn `cargo build --lib`: {e}"));
    assert!(
        out.status.success(),
        "`cargo build --lib` for the cdylib under test failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let dir = target_dir.join(&profile);
    first_so_in(&dir, Some("poly_ray_lib")).unwrap_or_else(|| {
        panic!(
            "libpoly_ray_lib.so not produced in {} even though the build succeeded",
            dir.display()
        )
    })
}

pub struct Pair {
    pub c: Api,
    pub rs: Api,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

/// Load (once, lazily) both shared objects and resolve all 28 symbols in each.
pub fn libs() -> &'static Pair {
    PAIR.get_or_init(|| unsafe {
        // `c_src/CMakeLists.txt` never links `m`, so the C `.so` has an
        // UNDEFINED `sqrtf` and depends on the loading process to supply it.
        // A debug test binary happens to keep libm's symbols reachable; an
        // optimised one does not, and `dlopen` then fails with
        // "undefined symbol: sqrtf". Pull libm in with RTLD_GLOBAL first so the
        // C library resolves identically under every profile.
        // NOTE: the handle must be LEAKED. Dropping it calls `dlclose`, which
        // would unload libm again and put us right back where we started.
        {
            use libloading::os::unix as u;
            let mut ok = false;
            for cand in ["libm.so.6", "libm.so"] {
                if let Ok(lib) = u::Library::open(Some(cand), u::RTLD_NOW | u::RTLD_GLOBAL) {
                    std::mem::forget(lib);
                    ok = true;
                    break;
                }
            }
            assert!(
                ok,
                "could not dlopen libm with RTLD_GLOBAL; the C .so's undefined \
                 `sqrtf` would fail to bind"
            );
        }

        let cpath = c_so_path();
        let rpath = rust_so_path();
        // Leaked so the resolved function pointers are valid for the whole test
        // process; the libraries must outlive every symbol taken from them.
        let clib: &'static Library = Box::leak(Box::new(
            Library::new(&cpath).unwrap_or_else(|e| panic!("dlopen {}: {e}", cpath.display())),
        ));
        let rlib: &'static Library = Box::leak(Box::new(
            Library::new(&rpath).unwrap_or_else(|e| panic!("dlopen {}: {e}", rpath.display())),
        ));
        Pair {
            c: Api::load("C", clib),
            rs: Api::load("RUST", rlib),
        }
    })
}

pub fn c_so_path_pub() -> PathBuf {
    c_so_path()
}
pub fn rust_so_path_pub() -> PathBuf {
    rust_so_path()
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Bit pattern of an `f32`, so `+0.0`/`-0.0` and distinct NaN payloads differ.
#[inline]
pub fn fb(x: f32) -> u32 {
    x.to_bits()
}

#[inline]
pub fn vb(v: c2v) -> (u32, u32) {
    (fb(v.x), fb(v.y))
}

#[inline]
pub fn rb(r: c2r) -> (u32, u32) {
    (fb(r.c), fb(r.s))
}

#[inline]
pub fn xb(x: c2x) -> (u32, u32, u32, u32) {
    (fb(x.p.x), fb(x.p.y), fb(x.r.c), fb(x.r.s))
}

/// Render an `f32` unambiguously (value + raw bits) for failure messages.
pub fn show(x: f32) -> String {
    format!("{:?}[{:#010x}]", x, x.to_bits())
}

pub fn showv(v: c2v) -> String {
    format!("({}, {})", show(v.x), show(v.y))
}

/// Compare two values and report a divergence.
///
/// NOTE: `$ctx` is evaluated ONLY on failure. Several tests call this millions of
/// times with a `format!(...)` context; evaluating that eagerly would make the
/// allocation, not the FFI call, dominate the runtime.
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

// ---------------------------------------------------------------------------
// Out-parameter buffer
// ---------------------------------------------------------------------------

/// A 32-byte, 8-aligned staging area for a 12-byte `c2Raycast` out-parameter.
///
/// Pre-filled with a poison pattern before every call so that
///   * "the C left `*out` untouched" is *verifiable* rather than assumed, and
///   * a write past the end of `c2Raycast` would show up as a byte diff.
#[repr(C, align(8))]
#[derive(Clone, Copy)]
pub struct OutBuf(pub [u8; 32]);

pub const POISON: u8 = 0xA5;

impl OutBuf {
    pub fn poisoned() -> OutBuf {
        OutBuf([POISON; 32])
    }
    pub fn as_ptr(&mut self) -> *mut c2Raycast {
        self.0.as_mut_ptr() as *mut c2Raycast
    }
    pub fn bytes(&self) -> [u8; 32] {
        self.0
    }
    pub fn is_pristine(&self) -> bool {
        self.0.iter().all(|&b| b == POISON)
    }
    pub fn cast(&self) -> c2Raycast {
        unsafe { (self.0.as_ptr() as *const c2Raycast).read_unaligned() }
    }
}

/// Result of running one raycast on one library: return code + full out buffer.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RayResult {
    pub ret: c_int,
    pub out: [u8; 32],
}

impl std::fmt::Debug for RayResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let rc = unsafe { (self.out.as_ptr() as *const c2Raycast).read_unaligned() };
        let pristine = self.out.iter().all(|&b| b == POISON);
        write!(
            f,
            "ret={} out={{t={}, n={}}}{} raw={:02x?}",
            self.ret,
            show(rc.t),
            showv(rc.n),
            if pristine { " (UNTOUCHED)" } else { "" },
            &self.out[..16]
        )
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) + float generators
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

/// Fixed seed so every run is reproducible.
pub const SEED: u64 = 0x5EED_C2C2_5EED_C2C2;

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `[0, n)`.
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Uniform in `[0, 1)`.
    #[inline]
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in `[-mag, mag]`.
    #[inline]
    pub fn sym(&mut self, mag: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * mag
    }
    /// Small integer-ish value, which makes exact ties (`==`, `<=`) likely and
    /// therefore hits the C's boundary branches far more often than a pure
    /// uniform draw would.
    #[inline]
    pub fn gridded(&mut self, half_range: i32) -> f32 {
        let n = self.below((half_range as u32) * 2 + 1) as i32 - half_range;
        let quarter = (self.below(4) as f32) * 0.25;
        n as f32 + quarter
    }
    pub fn vec_sym(&mut self, mag: f32) -> c2v {
        c2v {
            x: self.sym(mag),
            y: self.sym(mag),
        }
    }
    pub fn vec_grid(&mut self, half_range: i32) -> c2v {
        c2v {
            x: self.gridded(half_range),
            y: self.gridded(half_range),
        }
    }
    /// Unit-length direction.
    pub fn dir(&mut self) -> c2v {
        let a = self.unit() * std::f32::consts::TAU;
        c2v {
            x: a.cos(),
            y: a.sin(),
        }
    }
    /// A float drawn from a mixture of "ordinary", "special-class" and
    /// "arbitrary bit pattern" distributions.
    pub fn any_f32(&mut self) -> f32 {
        match self.below(10) {
            0..=4 => self.sym(1.0e3),
            5 => self.gridded(8),
            6 => *pick(SPECIAL_F32, self),
            7 => f32::from_bits(self.next_u32()), // may be NaN/inf/subnormal
            8 => self.sym(1.0) * f32::from_bits(0x0080_0000), // near-subnormal
            _ => self.sym(1.0e30),
        }
    }
    pub fn any_vec(&mut self) -> c2v {
        c2v {
            x: self.any_f32(),
            y: self.any_f32(),
        }
    }
    pub fn any_rot(&mut self) -> c2r {
        match self.below(6) {
            0 => c2r { c: 1.0, s: 0.0 },
            1 => {
                let a = self.unit() * std::f32::consts::TAU;
                c2r {
                    c: a.cos(),
                    s: a.sin(),
                }
            }
            2 => c2r { c: 0.0, s: 0.0 },
            3 => c2r {
                c: self.sym(2.0),
                s: self.sym(2.0),
            },
            4 => c2r {
                c: self.any_f32(),
                s: self.any_f32(),
            },
            _ => {
                let a = self.gridded(4) * std::f32::consts::FRAC_PI_4;
                c2r {
                    c: a.cos(),
                    s: a.sin(),
                }
            }
        }
    }
    pub fn any_x(&mut self) -> c2x {
        let p = match self.below(4) {
            0 => c2v { x: 0.0, y: 0.0 },
            1 => self.vec_grid(6),
            2 => self.vec_sym(50.0),
            _ => self.any_vec(),
        };
        c2x {
            p,
            r: self.any_rot(),
        }
    }
}

pub fn pick<'a, T>(slice: &'a [T], rng: &mut Rng) -> &'a T {
    &slice[rng.below(slice.len() as u32) as usize]
}

/// Interesting float values: signed zeros, subnormals, ±inf, both NaN signs,
/// exact powers of two, the smallest/largest normals, and small integers.
pub const SPECIAL_F32: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    -2.0,
    3.0,
    -3.0,
    4.0,
    -4.0,
    0.875,
    -0.875,
    11.5,
    -11.5,
    f32::EPSILON,
    -f32::EPSILON,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
];

/// Non-finite / sign-sensitive values, including several distinct NaN encodings
/// (quiet and signalling, both signs, default and custom payloads). Signalling
/// NaNs matter because x86 arithmetic *quiets* them, which is an observable
/// bit-level transformation the Rust must reproduce.
pub fn special_wide() -> Vec<f32> {
    let mut v: Vec<f32> = SPECIAL_F32.to_vec();
    v.push(f32::from_bits(0x7FC0_0000)); // +qNaN (default)
    v.push(f32::from_bits(0xFFC0_0000)); // -qNaN (x86 "real indefinite")
    v.push(f32::from_bits(0x7FC0_1234)); // +qNaN, non-default payload
    v.push(f32::from_bits(0x7F80_0001)); // +sNaN
    v.push(f32::from_bits(0xFF80_0001)); // -sNaN
    v.push(f32::from_bits(0x7FBF_FFFF)); // +sNaN, max payload
    v.push(f32::from_bits(0x0000_0001)); // smallest positive subnormal
    v.push(f32::from_bits(0x8000_0001)); // smallest negative subnormal
    v.push(f32::from_bits(0x007F_FFFF)); // largest subnormal
    v
}

/// Just the NaN encodings from [`special_wide`], for targeted probes.
pub const NANS: &[u32] = &[
    0x7FC0_0000,
    0xFFC0_0000,
    0x7FC0_1234,
    0xFFC0_1234,
    0x7F80_0001,
    0xFF80_0001,
    0x7FBF_FFFF,
];

// ---------------------------------------------------------------------------
// Shape generators used by several test files
// ---------------------------------------------------------------------------

/// A convex, correctly-wound polygon with `count` vertices (a regular n-gon,
/// scaled/rotated/translated), together with matching outward `norms`.
pub fn convex_ngon(rng: &mut Rng, count: i32) -> c2Poly {
    let mut p = c2Poly::default();
    let n = count.clamp(1, 8);
    let radius = 0.5 + rng.unit() * 8.0;
    let phase = rng.unit() * std::f32::consts::TAU;
    let cx = rng.sym(6.0);
    let cy = rng.sym(6.0);
    for i in 0..n {
        let a = phase + std::f32::consts::TAU * (i as f32) / (n as f32);
        p.verts[i as usize] = c2v {
            x: cx + radius * a.cos(),
            y: cy + radius * a.sin(),
        };
    }
    for i in 0..n {
        let j = (i + 1) % n;
        let (a, b) = (p.verts[i as usize], p.verts[j as usize]);
        let ex = b.x - a.x;
        let ey = b.y - a.y;
        // Outward normal of a CCW polygon is (edge.y, -edge.x), normalised.
        let len = (ex * ex + ey * ey).sqrt();
        p.norms[i as usize] = if len > 0.0 {
            c2v {
                x: ey / len,
                y: -ex / len,
            }
        } else {
            c2v { x: 1.0, y: 0.0 }
        };
    }
    p.count = count;
    p
}

/// The exact polygon `poly_ray` hard-codes (an 0.875 × 11.5 axis-aligned box).
pub fn poly_ray_box() -> c2Poly {
    let mut p = c2Poly::default();
    p.verts[0] = c2v { x: 0.875, y: -11.5 };
    p.verts[1] = c2v { x: 0.875, y: 11.5 };
    p.verts[2] = c2v { x: -0.875, y: 11.5 };
    p.verts[3] = c2v {
        x: -0.875,
        y: -11.5,
    };
    p.norms[0] = c2v { x: 1.0, y: 0.0 };
    p.norms[1] = c2v { x: 0.0, y: 1.0 };
    p.norms[2] = c2v { x: -1.0, y: 0.0 };
    p.norms[3] = c2v { x: 0.0, y: -1.0 };
    p.count = 4;
    p
}

/// An axis-aligned box polygon of random extents (CCW, outward normals).
pub fn box_poly(rng: &mut Rng) -> c2Poly {
    let mut p = c2Poly::default();
    let hx = 0.25 + rng.unit() * 6.0;
    let hy = 0.25 + rng.unit() * 6.0;
    let cx = rng.sym(5.0);
    let cy = rng.sym(5.0);
    p.verts[0] = c2v {
        x: cx + hx,
        y: cy - hy,
    };
    p.verts[1] = c2v {
        x: cx + hx,
        y: cy + hy,
    };
    p.verts[2] = c2v {
        x: cx - hx,
        y: cy + hy,
    };
    p.verts[3] = c2v {
        x: cx - hx,
        y: cy - hy,
    };
    p.norms[0] = c2v { x: 1.0, y: 0.0 };
    p.norms[1] = c2v { x: 0.0, y: 1.0 };
    p.norms[2] = c2v { x: -1.0, y: 0.0 };
    p.norms[3] = c2v { x: 0.0, y: -1.0 };
    p.count = 4;
    p
}

/// Fully arbitrary polygon contents (non-convex, inconsistent normals,
/// non-finite components) — the C never validates any of this.
pub fn wild_poly(rng: &mut Rng, count: i32) -> c2Poly {
    let mut p = c2Poly::default();
    for i in 0..8 {
        p.verts[i] = match rng.below(4) {
            0 => rng.vec_grid(6),
            1 => rng.vec_sym(20.0),
            2 => rng.any_vec(),
            _ => c2v {
                x: *pick(SPECIAL_F32, rng),
                y: *pick(SPECIAL_F32, rng),
            },
        };
        p.norms[i] = match rng.below(4) {
            0 => rng.dir(),
            1 => rng.vec_grid(2),
            2 => rng.any_vec(),
            _ => c2v {
                x: *pick(SPECIAL_F32, rng),
                y: *pick(SPECIAL_F32, rng),
            },
        };
    }
    p.count = count;
    p
}

/// Ray "shapes" the code special-cases: axis-aligned, unit, non-unit, zero
/// direction; `t` of 0, positive, negative, infinite.
pub fn any_ray(rng: &mut Rng) -> c2Ray {
    let p = match rng.below(4) {
        0 => rng.vec_grid(14),
        1 => rng.vec_sym(20.0),
        2 => rng.any_vec(),
        _ => c2v {
            x: rng.sym(6.0),
            y: rng.sym(14.0),
        },
    };
    let d = match rng.below(8) {
        0 => c2v { x: 1.0, y: 0.0 },
        1 => c2v { x: -1.0, y: 0.0 },
        2 => c2v { x: 0.0, y: 1.0 },
        3 => c2v { x: 0.0, y: -1.0 },
        4 => c2v { x: 0.0, y: 0.0 },
        5 => rng.dir(),
        6 => rng.vec_sym(3.0),
        _ => rng.any_vec(),
    };
    let t = match rng.below(8) {
        0 => 0.0,
        1 => 4.0,
        2 => rng.unit() * 40.0,
        3 => -(rng.unit() * 10.0),
        4 => f32::INFINITY,
        5 => 1.0,
        6 => rng.gridded(20),
        _ => rng.any_f32(),
    };
    c2Ray { p, d, t }
}

/// A "sane" ray: unit direction, finite origin, positive length. Used for the
/// rows that need the *hit* paths to actually be reached often.
pub fn sane_ray(rng: &mut Rng) -> c2Ray {
    c2Ray {
        p: rng.vec_sym(18.0),
        d: rng.dir(),
        t: 0.5 + rng.unit() * 40.0,
    }
}

pub fn any_aabb(rng: &mut Rng) -> c2AABB {
    match rng.below(6) {
        0 => {
            // proper
            let a = rng.vec_grid(10);
            let b = c2v {
                x: a.x + rng.unit() * 8.0,
                y: a.y + rng.unit() * 8.0,
            };
            c2AABB { min: a, max: b }
        }
        1 => {
            let a = rng.vec_grid(10);
            c2AABB { min: a, max: a } // degenerate
        }
        2 => {
            let a = rng.vec_grid(10);
            let b = c2v {
                x: a.x - rng.unit() * 8.0,
                y: a.y - rng.unit() * 8.0,
            };
            c2AABB { min: a, max: b } // inverted
        }
        3 => {
            let a = rng.vec_grid(10);
            c2AABB {
                min: a,
                max: c2v {
                    x: a.x,
                    y: a.y + rng.unit() * 5.0,
                },
            } // zero width
        }
        4 => c2AABB {
            min: rng.any_vec(),
            max: rng.any_vec(),
        },
        _ => {
            let a = rng.vec_sym(30.0);
            let b = c2v {
                x: a.x + rng.unit() * 30.0,
                y: a.y + rng.unit() * 30.0,
            };
            c2AABB { min: a, max: b }
        }
    }
}

pub fn any_circle(rng: &mut Rng) -> c2Circle {
    let p = match rng.below(3) {
        0 => rng.vec_grid(10),
        1 => rng.vec_sym(20.0),
        _ => rng.any_vec(),
    };
    let r = match rng.below(7) {
        0 => 0.0,
        1 => -(rng.unit() * 5.0),
        2 => rng.unit() * 10.0,
        3 => 1.0,
        4 => f32::INFINITY,
        5 => rng.gridded(6),
        _ => rng.any_f32(),
    };
    c2Circle { p, r }
}

pub fn any_capsule(rng: &mut Rng) -> c2Capsule {
    let a = match rng.below(3) {
        0 => rng.vec_grid(10),
        1 => rng.vec_sym(20.0),
        _ => rng.any_vec(),
    };
    let b = match rng.below(7) {
        0 => a,                                          // degenerate a == b
        1 => c2v { x: a.x + 5.0, y: a.y },               // horizontal
        2 => c2v { x: a.x, y: a.y + 5.0 },               // vertical
        3 => c2v { x: a.x, y: a.y - 5.0 },               // vertical, b below a
        4 => rng.vec_grid(10),                           // oblique
        5 => rng.vec_sym(20.0),
        _ => rng.any_vec(),
    };
    let r = match rng.below(7) {
        0 => 0.0,
        1 => -(rng.unit() * 4.0),
        2 => rng.unit() * 6.0,
        3 => 1.0,
        4 => 0.5,
        5 => rng.gridded(4),
        _ => rng.any_f32(),
    };
    c2Capsule { a, b, r }
}

// ---------------------------------------------------------------------------
// One-shot runners: invoke a raycast on ONE library with a poisoned out-buffer
// and capture (return value, full 32-byte out buffer).
// ---------------------------------------------------------------------------

pub fn run_circle(api: &Api, a: c2Ray, b: c2Circle) -> RayResult {
    let mut buf = OutBuf::poisoned();
    let ret = unsafe { (api.c2RaytoCircle)(a, b, buf.as_ptr()) };
    RayResult {
        ret,
        out: buf.bytes(),
    }
}

pub fn run_aabb(api: &Api, a: c2Ray, b: c2AABB) -> RayResult {
    let mut buf = OutBuf::poisoned();
    let ret = unsafe { (api.c2RaytoAABB)(a, b, buf.as_ptr()) };
    RayResult {
        ret,
        out: buf.bytes(),
    }
}

pub fn run_capsule(api: &Api, a: c2Ray, b: c2Capsule) -> RayResult {
    let mut buf = OutBuf::poisoned();
    let ret = unsafe { (api.c2RaytoCapsule)(a, b, buf.as_ptr()) };
    RayResult {
        ret,
        out: buf.bytes(),
    }
}

/// `c2RaytoPoly` over an explicit backing buffer, so the exact same bytes
/// (including any padding the C might read past `norms[8]`) are visible to both
/// libraries. `bx` may be `None` to pass `NULL`.
pub fn run_poly_raw(api: &Api, a: c2Ray, poly_bytes: &PolyBuf, bx: Option<&c2x>) -> RayResult {
    let mut buf = OutBuf::poisoned();
    let bxp = match bx {
        Some(x) => x as *const c2x,
        None => std::ptr::null(),
    };
    let ret = unsafe { (api.c2RaytoPoly)(a, poly_bytes.as_ptr(), bxp, buf.as_ptr()) };
    RayResult {
        ret,
        out: buf.bytes(),
    }
}

pub fn run_poly(api: &Api, a: c2Ray, p: &c2Poly, bx: Option<&c2x>) -> RayResult {
    run_poly_raw(api, a, &PolyBuf::from_poly(p), bx)
}

/// `c2CastRay` over an explicit shape byte buffer, so both libraries
/// reinterpret byte-identical memory for whichever `typeB` is selected.
pub fn run_cast(
    api: &Api,
    a: c2Ray,
    shape: &ShapeBuf,
    bx: Option<&c2x>,
    type_b: c_int,
) -> RayResult {
    let mut buf = OutBuf::poisoned();
    let bxp = match bx {
        Some(x) => x as *const c2x,
        None => std::ptr::null(),
    };
    let ret = unsafe { (api.c2CastRay)(a, shape.as_ptr(), bxp, type_b, buf.as_ptr()) };
    RayResult {
        ret,
        out: buf.bytes(),
    }
}

pub fn run_poly_ray(api: &Api) -> (c_int, [u8; 32], [u8; 32]) {
    let mut b1 = OutBuf::poisoned();
    let mut b2 = OutBuf::poisoned();
    let ret = unsafe { (api.poly_ray)(b1.as_ptr(), b2.as_ptr()) };
    (ret, b1.bytes(), b2.bytes())
}

// ---------------------------------------------------------------------------
// Backing buffers, so out-of-bounds reads see identical bytes in both libraries
// ---------------------------------------------------------------------------

/// A 512-byte, 16-aligned buffer holding a `c2Poly` at offset 0 plus
/// deterministic padding.
///
/// `c2Poly` is 132 bytes (`int count; c2v verts[8]; c2v norms[8]`). A `count`
/// greater than 8 makes the C read `verts[i]`/`norms[i]` past the declared
/// arrays — `norms[15]` lands at offset 188..196, well past the struct. Both
/// libraries index the *same* buffer, so the padding bytes are identical and
/// the comparison is still meaningful.
#[repr(C, align(16))]
pub struct PolyBuf(pub [u8; 512]);

impl PolyBuf {
    pub fn from_poly(p: &c2Poly) -> PolyBuf {
        // Deterministic, non-zero padding derived from a fixed pattern so that
        // out-of-bounds reads produce interesting (often non-finite) floats.
        let mut b = [0u8; 512];
        for (i, slot) in b.iter_mut().enumerate() {
            *slot = (i as u8).wrapping_mul(37).wrapping_add(11);
        }
        unsafe {
            std::ptr::copy_nonoverlapping(
                p as *const c2Poly as *const u8,
                b.as_mut_ptr(),
                std::mem::size_of::<c2Poly>(),
            );
        }
        PolyBuf(b)
    }
    /// Same as [`PolyBuf::from_poly`] but with caller-chosen padding, so the
    /// out-of-range vertex/normal slots can be filled with known floats.
    pub fn from_poly_with_tail(p: &c2Poly, tail: &[f32]) -> PolyBuf {
        let mut buf = PolyBuf::from_poly(p);
        let base = std::mem::size_of::<c2Poly>();
        for (i, &f) in tail.iter().enumerate() {
            let off = base + i * 4;
            if off + 4 <= buf.0.len() {
                buf.0[off..off + 4].copy_from_slice(&f.to_bits().to_le_bytes());
            }
        }
        buf
    }
    pub fn as_ptr(&self) -> *const c2Poly {
        self.0.as_ptr() as *const c2Poly
    }
    pub fn set_count(&mut self, n: c_int) {
        self.0[0..4].copy_from_slice(&n.to_le_bytes());
    }
}

/// A 64-byte, 16-aligned buffer used as the `const void *B` argument of
/// `c2CastRay`, so a `c2Circle` (12 B), `c2AABB` (16 B) or `c2Capsule` (20 B)
/// can be reinterpreted from identical bytes.
#[repr(C, align(16))]
pub struct ShapeBuf(pub [u8; 64]);

impl ShapeBuf {
    pub fn zeroed() -> ShapeBuf {
        ShapeBuf([0u8; 64])
    }
    pub fn from_bytes(src: &[u8]) -> ShapeBuf {
        let mut b = [0u8; 64];
        b[..src.len()].copy_from_slice(src);
        ShapeBuf(b)
    }
    pub fn from_circle(c: &c2Circle) -> ShapeBuf {
        ShapeBuf::from_bytes(unsafe {
            std::slice::from_raw_parts(c as *const c2Circle as *const u8, 12)
        })
    }
    pub fn from_aabb(a: &c2AABB) -> ShapeBuf {
        ShapeBuf::from_bytes(unsafe {
            std::slice::from_raw_parts(a as *const c2AABB as *const u8, 16)
        })
    }
    pub fn from_capsule(c: &c2Capsule) -> ShapeBuf {
        ShapeBuf::from_bytes(unsafe {
            std::slice::from_raw_parts(c as *const c2Capsule as *const u8, 20)
        })
    }
    pub fn as_ptr(&self) -> *const c_void {
        self.0.as_ptr() as *const c_void
    }
}
