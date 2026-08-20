//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries through `libloading`:
//!   * the C reference: `c_src/build/libtranslated_rust.so`
//!   * the Rust translation: `target/<profile>/libgen_ray_lib.so`
//!
//! Rust functions are NEVER called directly - every call goes through the
//! `.so`'s exported `#[no_mangle]` symbol, exactly like an external C consumer.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI types (mirrors of c_src/include/lib.h + the private structs in lib.c)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Raycast {
    pub t: f32,
    pub n: c2v,
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
pub struct c2Ray {
    pub p: c2v,
    pub d: c2v,
    pub t: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2m {
    pub x: c2v,
    pub y: c2v,
}

pub const C2_TYPE_CIRCLE: i32 = 0;
pub const C2_TYPE_AABB: i32 = 1;
pub const C2_TYPE_CAPSULE: i32 = 2;

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub fn bits(x: f32) -> u32 {
    x.to_bits()
}

pub fn fmt_f(x: f32) -> String {
    format!("{:?}[{:#010x}]", x, x.to_bits())
}

pub fn fmt_v(v: c2v) -> String {
    format!("({}, {})", fmt_f(v.x), fmt_f(v.y))
}

pub fn fmt_cast(c: c2Raycast) -> String {
    format!("{{t: {}, n: {}}}", fmt_f(c.t), fmt_v(c.n))
}

pub fn f_eq(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

pub fn v_eq(a: c2v, b: c2v) -> bool {
    f_eq(a.x, b.x) && f_eq(a.y, b.y)
}

pub fn cast_eq(a: c2Raycast, b: c2Raycast) -> bool {
    f_eq(a.t, b.t) && v_eq(a.n, b.n)
}

// ---------------------------------------------------------------------------
// The loaded API surface (all 22 exported symbols)
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: &'static str,
    pub c2V: extern "C" fn(f32, f32) -> c2v,
    pub c2Dot: extern "C" fn(c2v, c2v) -> f32,
    pub c2Len: extern "C" fn(c2v) -> f32,
    pub c2Add: extern "C" fn(c2v, c2v) -> c2v,
    pub c2Sub: extern "C" fn(c2v, c2v) -> c2v,
    pub c2Mulvs: extern "C" fn(c2v, f32) -> c2v,
    pub c2Div: extern "C" fn(c2v, f32) -> c2v,
    pub c2Norm: extern "C" fn(c2v) -> c2v,
    pub c2Minv: extern "C" fn(c2v, c2v) -> c2v,
    pub c2Maxv: extern "C" fn(c2v, c2v) -> c2v,
    pub c2Skew: extern "C" fn(c2v) -> c2v,
    pub c2Absv: extern "C" fn(c2v) -> c2v,
    pub c2CCW90: extern "C" fn(c2v) -> c2v,
    pub c2MulmvT: extern "C" fn(c2m, c2v) -> c2v,
    pub c2AABBtoAABB: extern "C" fn(c2AABB, c2AABB) -> i32,
    pub c2AABBtoPoint: extern "C" fn(c2AABB, c2v) -> i32,
    pub c2CircleToPoint: extern "C" fn(c2Circle, c2v) -> i32,
    pub c2RaytoCircle: unsafe extern "C" fn(c2Ray, c2Circle, *mut c2Raycast) -> i32,
    pub c2RaytoAABB: unsafe extern "C" fn(c2Ray, c2AABB, *mut c2Raycast) -> i32,
    pub c2RaytoCapsule: unsafe extern "C" fn(c2Ray, c2Capsule, *mut c2Raycast) -> i32,
    pub c2CastRay: unsafe extern "C" fn(c2Ray, *const c_void, i32, *mut c2Raycast) -> i32,
    pub gen_ray: unsafe extern "C" fn(
        *mut c2Raycast,
        *mut c2Raycast,
        *mut c2Raycast,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
    ) -> i32,
}

unsafe fn sym<T: Copy>(lib: &'static Library, name: &[u8]) -> T {
    unsafe {
        let s: Symbol<T> = lib
            .get(name)
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
        *s
    }
}

unsafe fn load(name: &'static str, path: &PathBuf) -> Api {
    unsafe {
        let lib: &'static Library = Box::leak(Box::new(
            Library::new(path).unwrap_or_else(|e| panic!("cannot load {}: {e}", path.display())),
        ));
        Api {
            name,
            c2V: sym(lib, b"c2V\0"),
            c2Dot: sym(lib, b"c2Dot\0"),
            c2Len: sym(lib, b"c2Len\0"),
            c2Add: sym(lib, b"c2Add\0"),
            c2Sub: sym(lib, b"c2Sub\0"),
            c2Mulvs: sym(lib, b"c2Mulvs\0"),
            c2Div: sym(lib, b"c2Div\0"),
            c2Norm: sym(lib, b"c2Norm\0"),
            c2Minv: sym(lib, b"c2Minv\0"),
            c2Maxv: sym(lib, b"c2Maxv\0"),
            c2Skew: sym(lib, b"c2Skew\0"),
            c2Absv: sym(lib, b"c2Absv\0"),
            c2CCW90: sym(lib, b"c2CCW90\0"),
            c2MulmvT: sym(lib, b"c2MulmvT\0"),
            c2AABBtoAABB: sym(lib, b"c2AABBtoAABB\0"),
            c2AABBtoPoint: sym(lib, b"c2AABBtoPoint\0"),
            c2CircleToPoint: sym(lib, b"c2CircleToPoint\0"),
            c2RaytoCircle: sym(lib, b"c2RaytoCircle\0"),
            c2RaytoAABB: sym(lib, b"c2RaytoAABB\0"),
            c2RaytoCapsule: sym(lib, b"c2RaytoCapsule\0"),
            c2CastRay: sym(lib, b"c2CastRay\0"),
            gen_ray: sym(lib, b"gen_ray\0"),
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn mtime(p: &PathBuf) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

/// Is `so` present and at least as new as `src`?
fn fresh(so: &PathBuf, src: &PathBuf) -> bool {
    match (mtime(so), mtime(src)) {
        (Some(a), Some(b)) => a >= b,
        _ => false,
    }
}

/// Scratch directory for fallback builds (never inside `c_src/`, which must not
/// be modified).
fn scratch() -> PathBuf {
    let d = manifest_dir().join("target/diff-artifacts");
    let _ = std::fs::create_dir_all(&d);
    d
}

/// The C reference `.so`.
///
/// Preferred: the artifact produced by the documented CMake invocation
/// (`c_src/build/libtranslated_rust.so`).  If it is missing, build an equivalent
/// one with `cc` using the same flags CMake's default (no `CMAKE_BUILD_TYPE`)
/// configuration uses - no optimization flags at all - so that a bare
/// `cargo test` works in a fresh checkout.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let cmake_out = manifest_dir().join("c_src/build/libtranslated_rust.so");
    let src = manifest_dir().join("c_src/src/lib.c");
    if fresh(&cmake_out, &src) {
        return cmake_out;
    }
    let out = scratch().join("libtranslated_rust.so");
    if fresh(&out, &src) {
        return out;
    }
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let status = std::process::Command::new(&cc)
        .args(["-fPIC", "-shared", "-O0"])
        .arg(format!("-I{}", manifest_dir().join("c_src/include").display()))
        .arg(&src)
        .arg("-lm")
        .arg("-o")
        .arg(&out)
        .status();
    match status {
        Ok(s) if s.success() => out,
        other => panic!(
            "the C reference library is missing and could not be built.\n\
             Expected: {}\n\
             Build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
             (fallback `{cc}` invocation returned {other:?})",
            cmake_out.display()
        ),
    }
}

/// The Rust cdylib under test.
///
/// `cargo test` does NOT (re)build a `crate-type = ["cdylib"]` artifact, so the
/// artifact next to the test binary can be missing or stale.  Rather than
/// silently testing yesterday's `.so`, rebuild it from `src/lib.rs` with a
/// direct `rustc` call (the crate has no dependencies, so this needs no cargo
/// and cannot deadlock on cargo's build lock).
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let src = manifest_dir().join("src/lib.rs");
    // The integration-test executable lives in target/<profile>/deps/, so the
    // cdylib for the very same profile is one directory up.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .expect("target/<profile>");
    let cargo_out = profile_dir.join("libgen_ray_lib.so");
    if fresh(&cargo_out, &src) {
        return cargo_out;
    }
    let out = scratch().join("libgen_ray_lib.so");
    if fresh(&out, &src) {
        return out;
    }
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let status = std::process::Command::new(&rustc)
        .args([
            "--crate-type=cdylib",
            "--crate-name=gen_ray_lib",
            "--edition=2024",
            "-O",
            "-C",
            "debug-assertions=off",
            "-C",
            "overflow-checks=off",
        ])
        .arg(&src)
        .arg("-o")
        .arg(&out)
        .status();
    match status {
        Ok(s) if s.success() => {
            eprintln!(
                "note: {} was missing/stale, rebuilt it with `{rustc}`",
                cargo_out.display()
            );
            out
        }
        other => panic!(
            "the Rust cdylib is missing or stale ({}) and the fallback `{rustc}` \
             build failed ({other:?}).\n\
             Run `cargo build` (or ./run_diff_tests.sh) first.",
            cargo_out.display()
        ),
    }
}

static APIS: OnceLock<(Api, Api)> = OnceLock::new();

/// `(c_api, rust_api)`
pub fn apis() -> &'static (Api, Api) {
    APIS.get_or_init(|| unsafe {
        let rs = rust_so_path();
        let cs = c_so_path();
        eprintln!("C  .so: {}", cs.display());
        eprintln!("RS .so: {}", rs.display());
        (load("C", &cs), load("RUST", &rs))
    })
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xoshiro-ish; fixed seed for reproducibility)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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

    /// Uniform in [-scale, scale].
    pub fn uniform(&mut self, scale: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
        (u * 2.0 - 1.0) * scale
    }

    /// A "nice" finite float in a geometry-plausible range.
    pub fn nice(&mut self) -> f32 {
        match self.below(8) {
            0 => self.uniform(1.0),
            1 => self.uniform(10.0),
            2 => self.uniform(100.0),
            3 => self.uniform(1000.0),
            4 => (self.uniform(10.0) as i32) as f32, // small integers
            5 => self.uniform(1e6),
            6 => self.uniform(1e-4),
            _ => self.uniform(4.0),
        }
    }

    /// Non-negative "nice" float (radii, extents).
    pub fn nice_pos(&mut self) -> f32 {
        let v = self.nice();
        if v < 0.0 { -v } else { v }
    }

    /// A float drawn from the full set of interesting values, including
    /// signed zeros, infinities, NaNs, subnormals and raw bit patterns.
    pub fn hostile(&mut self) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::NAN,
            5 => -f32::NAN,
            6 => f32::from_bits(0x7f80_0001), // signalling NaN
            7 => f32::from_bits(0xff80_0001), // negative signalling NaN
            8 => f32::MIN_POSITIVE,
            9 => -f32::MIN_POSITIVE,
            10 => f32::from_bits(1), // smallest subnormal
            11 => f32::MAX,
            12 => f32::MIN,
            13 => f32::from_bits(self.next_u32()),
            14 => f32::from_bits(self.next_u32()),
            _ => self.nice(),
        }
    }

    pub fn vec_nice(&mut self) -> c2v {
        c2v {
            x: self.nice(),
            y: self.nice(),
        }
    }

    pub fn vec_hostile(&mut self) -> c2v {
        c2v {
            x: self.hostile(),
            y: self.hostile(),
        }
    }
}

/// A canonical "poison" raycast used to detect whether the callee wrote to
/// `out` at all (and to make sure both libraries write the same fields).
pub const POISON: c2Raycast = c2Raycast {
    t: -1.234_567_9e28,
    n: c2v {
        x: 9.876_543e-11,
        y: -5.555_5e17,
    },
};

pub struct Diff {
    pub fails: usize,
    pub checks: usize,
    pub first_msgs: Vec<String>,
}

impl Diff {
    pub fn new() -> Self {
        Diff {
            fails: 0,
            checks: 0,
            first_msgs: Vec::new(),
        }
    }

    pub fn check(&mut self, ok: bool, msg: impl FnOnce() -> String) {
        self.checks += 1;
        if !ok {
            self.fails += 1;
            if self.first_msgs.len() < 10 {
                self.first_msgs.push(msg());
            }
        }
    }

    pub fn finish(self, label: &str) {
        if self.fails != 0 {
            panic!(
                "{label}: {} of {} differential checks FAILED\n{}",
                self.fails,
                self.checks,
                self.first_msgs.join("\n")
            );
        }
        assert!(self.checks > 0, "{label}: no checks ran");
        eprintln!("{label}: {} differential checks OK", self.checks);
    }
}

// ---------------------------------------------------------------------------
// Call wrappers: run one raycast through a `.so` and capture (ret, *out).
//
// `out` is pre-filled with POISON so that "did not write" is distinguishable
// from "wrote something that happens to equal the default".
// ---------------------------------------------------------------------------

pub type RayResult = (i32, c2Raycast);

pub fn call_circle(api: &Api, ray: c2Ray, b: c2Circle) -> RayResult {
    let mut out = POISON;
    let r = unsafe { (api.c2RaytoCircle)(ray, b, &mut out) };
    (r, out)
}

pub fn call_aabb(api: &Api, ray: c2Ray, b: c2AABB) -> RayResult {
    let mut out = POISON;
    let r = unsafe { (api.c2RaytoAABB)(ray, b, &mut out) };
    (r, out)
}

pub fn call_capsule(api: &Api, ray: c2Ray, b: c2Capsule) -> RayResult {
    let mut out = POISON;
    let r = unsafe { (api.c2RaytoCapsule)(ray, b, &mut out) };
    (r, out)
}

/// `c2CastRay` with a raw 20-byte shape buffer, so the same bytes can be
/// reinterpreted under any `typeB`.
pub fn call_castray(api: &Api, ray: c2Ray, buf: &[u8; 20], ty: i32) -> RayResult {
    let mut out = POISON;
    let r = unsafe {
        (api.c2CastRay)(
            ray,
            buf.as_ptr() as *const c_void,
            ty,
            &mut out,
        )
    };
    (r, out)
}

pub struct GenRayResult {
    pub ret: i32,
    pub cast1: c2Raycast,
    pub cast2: c2Raycast,
    pub cast3: c2Raycast,
}

/// The 16 float parameters of `gen_ray`, in declaration order.
pub type GenRayArgs = [f32; 16];

pub fn call_gen_ray(api: &Api, a: &GenRayArgs) -> GenRayResult {
    let mut c1 = POISON;
    let mut c2 = POISON;
    let mut c3 = POISON;
    let ret = unsafe {
        (api.gen_ray)(
            &mut c1, &mut c2, &mut c3, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8], a[9],
            a[10], a[11], a[12], a[13], a[14], a[15],
        )
    };
    GenRayResult {
        ret,
        cast1: c1,
        cast2: c2,
        cast3: c3,
    }
}

/// `gen_ray` with all three out-pointers aliased to the same `c2Raycast`.
pub fn call_gen_ray_aliased(api: &Api, a: &GenRayArgs) -> (i32, c2Raycast) {
    let mut c1 = POISON;
    let p: *mut c2Raycast = &mut c1;
    let ret = unsafe {
        (api.gen_ray)(
            p, p, p, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8], a[9], a[10], a[11],
            a[12], a[13], a[14], a[15],
        )
    };
    (ret, c1)
}

impl Diff {
    /// Compare a `(ret, *out)` pair from the two libraries.
    pub fn ray(&mut self, label: &str, ctx: impl Fn() -> String, c: RayResult, r: RayResult) {
        let ok = c.0 == r.0 && cast_eq(c.1, r.1);
        self.check(ok, || {
            format!(
                "{label} [{}]:\n    C   : ret={} out={}\n    RUST: ret={} out={}",
                ctx(),
                c.0,
                fmt_cast(c.1),
                r.0,
                fmt_cast(r.1)
            )
        });
    }

    pub fn ints(&mut self, label: &str, ctx: impl Fn() -> String, c: i32, r: i32) {
        self.check(c == r, || {
            format!("{label} [{}]: C ret={c} RUST ret={r}", ctx())
        });
    }

    pub fn gen_cmp(&mut self, label: &str, ctx: impl Fn() -> String, c: GenRayResult, r: GenRayResult) {
        let ok = c.ret == r.ret
            && cast_eq(c.cast1, r.cast1)
            && cast_eq(c.cast2, r.cast2)
            && cast_eq(c.cast3, r.cast3);
        self.check(ok, || {
            format!(
                "{label} [{}]:\n    C   : ret={} c1={} c2={} c3={}\n    RUST: ret={} c1={} c2={} c3={}",
                ctx(),
                c.ret,
                fmt_cast(c.cast1),
                fmt_cast(c.cast2),
                fmt_cast(c.cast3),
                r.ret,
                fmt_cast(r.cast1),
                fmt_cast(r.cast2),
                fmt_cast(r.cast3)
            )
        });
    }
}

// ---------------------------------------------------------------------------
// Shape / ray generators
// ---------------------------------------------------------------------------

impl Rng {
    pub fn ray_nice(&mut self) -> c2Ray {
        c2Ray {
            p: self.vec_nice(),
            d: self.vec_nice(),
            t: self.nice(),
        }
    }

    pub fn ray_hostile(&mut self) -> c2Ray {
        c2Ray {
            p: self.vec_hostile(),
            d: self.vec_hostile(),
            t: self.hostile(),
        }
    }

    pub fn circle_nice(&mut self) -> c2Circle {
        c2Circle {
            p: self.vec_nice(),
            r: self.nice(),
        }
    }

    pub fn circle_hostile(&mut self) -> c2Circle {
        c2Circle {
            p: self.vec_hostile(),
            r: self.hostile(),
        }
    }

    pub fn aabb_nice(&mut self) -> c2AABB {
        c2AABB {
            min: self.vec_nice(),
            max: self.vec_nice(),
        }
    }

    pub fn aabb_hostile(&mut self) -> c2AABB {
        c2AABB {
            min: self.vec_hostile(),
            max: self.vec_hostile(),
        }
    }

    pub fn capsule_nice(&mut self) -> c2Capsule {
        c2Capsule {
            a: self.vec_nice(),
            b: self.vec_nice(),
            r: self.nice(),
        }
    }

    pub fn capsule_hostile(&mut self) -> c2Capsule {
        c2Capsule {
            a: self.vec_hostile(),
            b: self.vec_hostile(),
            r: self.hostile(),
        }
    }

    /// A proper (min <= max) box around a random center.
    pub fn aabb_proper(&mut self) -> c2AABB {
        let cx = self.uniform(20.0);
        let cy = self.uniform(20.0);
        let hx = (self.uniform(10.0)).abs() + 1e-3;
        let hy = (self.uniform(10.0)).abs() + 1e-3;
        c2AABB {
            min: c2v {
                x: cx - hx,
                y: cy - hy,
            },
            max: c2v {
                x: cx + hx,
                y: cy + hy,
            },
        }
    }

    /// A unit-length direction.
    pub fn unit(&mut self) -> c2v {
        let ang = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32 * 6.283_185_5;
        c2v {
            x: ang.cos(),
            y: ang.sin(),
        }
    }
}

/// The 10 "interesting" IEEE values used for exhaustive cross products.
pub const SPECIALS: [f32; 10] = [
    0.0,
    -0.0,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    f32::MAX,
    f32::MIN,
    f32::MIN_POSITIVE,
    1.5,
];

/// Turn any shape into the 20-byte buffer `c2CastRay` reads through `void *`.
pub fn shape_bytes_circle(c: c2Circle) -> [u8; 20] {
    let mut b = [0u8; 20];
    b[0..4].copy_from_slice(&c.p.x.to_ne_bytes());
    b[4..8].copy_from_slice(&c.p.y.to_ne_bytes());
    b[8..12].copy_from_slice(&c.r.to_ne_bytes());
    b
}

pub fn shape_bytes_aabb(a: c2AABB) -> [u8; 20] {
    let mut b = [0u8; 20];
    b[0..4].copy_from_slice(&a.min.x.to_ne_bytes());
    b[4..8].copy_from_slice(&a.min.y.to_ne_bytes());
    b[8..12].copy_from_slice(&a.max.x.to_ne_bytes());
    b[12..16].copy_from_slice(&a.max.y.to_ne_bytes());
    b
}

pub fn shape_bytes_capsule(c: c2Capsule) -> [u8; 20] {
    let mut b = [0u8; 20];
    b[0..4].copy_from_slice(&c.a.x.to_ne_bytes());
    b[4..8].copy_from_slice(&c.a.y.to_ne_bytes());
    b[8..12].copy_from_slice(&c.b.x.to_ne_bytes());
    b[12..16].copy_from_slice(&c.b.y.to_ne_bytes());
    b[16..20].copy_from_slice(&c.r.to_ne_bytes());
    b
}
