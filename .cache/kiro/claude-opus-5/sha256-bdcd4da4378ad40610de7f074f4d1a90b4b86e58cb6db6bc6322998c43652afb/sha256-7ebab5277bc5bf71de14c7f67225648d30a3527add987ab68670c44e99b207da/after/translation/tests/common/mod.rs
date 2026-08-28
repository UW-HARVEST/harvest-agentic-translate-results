//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and exposes matched symbol pairs so every call crosses the FFI boundary.
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// C-compatible type definitions (mirrors of c_src/src/lib.c)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Raycast {
    pub t: f32,
    pub n: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
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
#[derive(Copy, Clone, Debug)]
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
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Ray {
    pub p: c2v,
    pub d: c2v,
    pub t: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2m {
    pub x: c2v,
    pub y: c2v,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;
pub const C2_TYPE_POLY: c_int = 3;

// ---------------------------------------------------------------------------
// Function pointer signatures
// ---------------------------------------------------------------------------

pub type FnV = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnVV_f = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnV_f = unsafe extern "C" fn(c2v) -> f32;
pub type FnVV_V = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnVf_V = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnV_V = unsafe extern "C" fn(c2v) -> c2v;
pub type FnMV_V = unsafe extern "C" fn(c2m, c2v) -> c2v;
pub type FnRV_V = unsafe extern "C" fn(c2r, c2v) -> c2v;
pub type FnXV_V = unsafe extern "C" fn(c2x, c2v) -> c2v;
pub type Fn_R = unsafe extern "C" fn() -> c2r;
pub type Fn_X = unsafe extern "C" fn() -> c2x;
pub type FnAABBAABB_i = unsafe extern "C" fn(c2AABB, c2AABB) -> c_int;
pub type FnAABBV_i = unsafe extern "C" fn(c2AABB, c2v) -> c_int;
pub type FnCircleV_i = unsafe extern "C" fn(c2Circle, c2v) -> c_int;
pub type FnRayCircle_i = unsafe extern "C" fn(c2Ray, c2Circle, *mut c2Raycast) -> c_int;
pub type FnRayAABB_i = unsafe extern "C" fn(c2Ray, c2AABB, *mut c2Raycast) -> c_int;
pub type FnRayCapsule_i = unsafe extern "C" fn(c2Ray, c2Capsule, *mut c2Raycast) -> c_int;
pub type FnRayPoly_i =
    unsafe extern "C" fn(c2Ray, *const c2Poly, *const c2x, *mut c2Raycast) -> c_int;
pub type FnCastRay_i =
    unsafe extern "C" fn(c2Ray, *const c_void, *const c2x, c_int, *mut c2Raycast) -> c_int;
pub type FnPolyRay_i = unsafe extern "C" fn(*mut c2Raycast, *mut c2Raycast) -> c_int;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build_dir = manifest_dir().parent().unwrap().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build_dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.pop().unwrap_or_else(|| {
        panic!(
            "no C .so found in {:?}; build it with cmake first",
            build_dir
        )
    })
}

/// Absolute path of the crate's cdylib to test.
///
/// `cargo test` builds the lib target as an rlib for the test binaries but does
/// **not** refresh the `cdylib` artifact, so naively picking up
/// `target/debug/libpoly_ray_lib.so` can silently test a stale library (an
/// injected bug in src/lib.rs then goes undetected). Two safeguards:
///
///  * If `RUST_SO_PATH` is set (how `verify_all.sh` drives this), that exact
///    file is used and only checked for freshness.
///  * Otherwise the cdylib is rebuilt here via a nested `cargo build` into a
///    dedicated `--target-dir`, so it cannot conflict with the lock held by the
///    outer `cargo test`.
fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        let p = PathBuf::from(p);
        assert_fresh(&p);
        return p;
    }

    let harness_target = manifest_dir().join("target").join("harness");
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let mut args = vec!["build", "--target-dir"];
    let tdir = harness_target.to_string_lossy().to_string();
    args.push(&tdir);
    if profile == "release" {
        args.push("--release");
    }
    let out = std::process::Command::new(env!("CARGO"))
        .args(&args)
        .current_dir(manifest_dir())
        .output()
        .expect("failed to run nested `cargo build` for the cdylib");
    assert!(
        out.status.success(),
        "nested `cargo build` failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let p = harness_target.join(profile).join("libpoly_ray_lib.so");
    assert!(
        p.exists(),
        "nested build produced no cdylib at {p:?}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_fresh(&p);
    p
}

/// Refuse to test a cdylib older than the sources it was built from.
fn assert_fresh(so: &PathBuf) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("cannot stat {so:?}: {e}"));
    for src in ["src/lib.rs", "Cargo.toml"] {
        let p = manifest_dir().join(src);
        if let Ok(t) = std::fs::metadata(&p).and_then(|m| m.modified()) {
            assert!(
                so_mtime >= t,
                "STALE cdylib: {so:?} is older than {p:?}.\n\
                 Run `cargo build` (or ./verify_all.sh) before `cargo test`, \
                 or set RUST_SO_PATH to a freshly built library."
            );
        }
    }
}

pub struct Libs {
    pub c: Library,
    pub rs: Library,
}

impl Libs {
    pub fn sym<T: Copy>(&self, name: &str) -> (T, T) {
        let cname = std::ffi::CString::new(name).unwrap();
        unsafe {
            let a: Symbol<T> = self
                .c
                .get(cname.as_bytes_with_nul())
                .unwrap_or_else(|e| panic!("C .so missing symbol `{name}`: {e}"));
            let b: Symbol<T> = self
                .rs
                .get(cname.as_bytes_with_nul())
                .unwrap_or_else(|e| panic!("Rust .so missing symbol `{name}`: {e}"));
            (*a, *b)
        }
    }
}

static LIBS: OnceLock<Libs> = OnceLock::new();
static PATHS: OnceLock<(PathBuf, PathBuf)> = OnceLock::new();

/// `(c_so, rust_so)` -- resolved once, shared by every test in the binary.
pub fn so_paths() -> &'static (PathBuf, PathBuf) {
    PATHS.get_or_init(|| (find_c_so(), find_rust_so()))
}

/// The C `.so` has an undefined, unversioned reference to `sqrtf` because
/// `c_src/CMakeLists.txt` never links libm. Whether that resolves at `dlopen`
/// time depends on what the *test binary* happens to pull in, which is fragile.
/// Load libm into the global scope up front so resolution is deterministic.
fn preload_libm() {
    use libloading::os::unix::{Library as UnixLibrary, RTLD_GLOBAL, RTLD_NOW};
    static LIBM: OnceLock<Option<UnixLibrary>> = OnceLock::new();
    LIBM.get_or_init(|| {
        for name in ["libm.so.6", "libm.so"] {
            if let Ok(l) = unsafe { UnixLibrary::open(Some(name), RTLD_NOW | RTLD_GLOBAL) } {
                return Some(l);
            }
        }
        // On glibc >= 2.34 the math symbols live in libc, which is already in
        // the global scope, so failing here is not necessarily fatal.
        None
    });
}

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| unsafe {
        preload_libm();
        let (cpath, rpath) = so_paths();
        let c = Library::new(cpath).unwrap_or_else(|e| panic!("load {cpath:?}: {e}"));
        let rs = Library::new(rpath).unwrap_or_else(|e| panic!("load {rpath:?}: {e}"));
        // Force eager resolution of `sqrtf` so a missing-libm environment fails
        // loudly here instead of aborting the process mid-test.
        let probe: Symbol<unsafe extern "C" fn(c2v) -> f32> = c
            .get(b"c2Len\0")
            .expect("C .so missing c2Len");
        let v = probe(c2v { x: 3.0, y: 4.0 });
        assert_eq!(v, 5.0, "C .so sanity check failed (c2Len(3,4) != 5)");
        Libs { c, rs }
    })
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
//
// Results are compared bit-for-bit, with one documented exception: the *payload*
// of a NaN result. That payload is not a property of the C source -- compiling
// c_src unchanged at -O1 vs -O2 already flips it (e.g. `c2Dot(+nan, -nan)`
// yields ffc00000 at -O0 and 7fc00000 at -O1+), because x86 SSE returns the
// destination operand's NaN and operand assignment is a codegen choice. So NaN
// results are required to be NaN on both sides, while every other value --
// including the 0.0 / -0.0 distinction, infinities and subnormals -- must match
// exactly. NaN *inputs* are still exercised throughout.
// ---------------------------------------------------------------------------

const NAN_CANON: u32 = 0x7fc0_0000;

pub trait Bits {
    /// Raw bits, for diagnostics.
    fn bits(&self) -> Vec<u32>;
    /// Bits with NaN payloads folded to a single value, for comparison.
    fn canon(&self) -> Vec<u32> {
        self.bits()
    }
}

impl Bits for f32 {
    fn bits(&self) -> Vec<u32> {
        vec![self.to_bits()]
    }
    fn canon(&self) -> Vec<u32> {
        vec![if self.is_nan() {
            NAN_CANON
        } else {
            self.to_bits()
        }]
    }
}

impl Bits for c_int {
    fn bits(&self) -> Vec<u32> {
        vec![*self as u32]
    }
}

impl Bits for c2v {
    fn bits(&self) -> Vec<u32> {
        vec![self.x.to_bits(), self.y.to_bits()]
    }
    fn canon(&self) -> Vec<u32> {
        let mut v = self.x.canon();
        v.extend(self.y.canon());
        v
    }
}

impl Bits for c2r {
    fn bits(&self) -> Vec<u32> {
        vec![self.c.to_bits(), self.s.to_bits()]
    }
    fn canon(&self) -> Vec<u32> {
        let mut v = self.c.canon();
        v.extend(self.s.canon());
        v
    }
}

impl Bits for c2x {
    fn bits(&self) -> Vec<u32> {
        let mut v = self.p.bits();
        v.extend(self.r.bits());
        v
    }
    fn canon(&self) -> Vec<u32> {
        let mut v = self.p.canon();
        v.extend(self.r.canon());
        v
    }
}

impl Bits for c2Raycast {
    fn bits(&self) -> Vec<u32> {
        let mut v = vec![self.t.to_bits()];
        v.extend(self.n.bits());
        v
    }
    fn canon(&self) -> Vec<u32> {
        let mut v = self.t.canon();
        v.extend(self.n.canon());
        v
    }
}

/// Assert two values agree bit-for-bit (NaN payloads excepted, see above).
#[track_caller]
pub fn assert_bits<T: Bits + std::fmt::Debug>(what: &str, ctx: &str, c: &T, r: &T) {
    if c.canon() != r.canon() {
        panic!(
            "{what} mismatch for {ctx}\n  C  = {c:?} bits {:08x?}\n  RS = {r:?} bits {:08x?}",
            c.bits(),
            r.bits()
        );
    }
}

// ---------------------------------------------------------------------------
// Deterministic input generation
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }
    pub fn next_u32(&mut self) -> u32 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545F4914F6CDD1D) >> 32) as u32
    }
    /// Uniform in [-range, range], quantised to f32.
    pub fn f32_range(&mut self, range: f32) -> f32 {
        let u = self.next_u32();
        let unit = (u as f64) / (u32::MAX as f64); // [0,1]
        ((unit * 2.0 - 1.0) as f32) * range
    }
    /// Small integers and simple fractions -- good at hitting exact-equality
    /// branches (`den == 0`, `d != 0`, ties in the `>=` chains).
    pub fn f32_coarse(&mut self) -> f32 {
        let u = self.next_u32();
        let n = ((u % 17) as i32) - 8; // -8..8
        let scale = match (u >> 8) % 4 {
            0 => 1.0,
            1 => 0.5,
            2 => 0.25,
            _ => 2.0,
        };
        n as f32 * scale
    }
    pub fn vec_range(&mut self, range: f32) -> c2v {
        c2v {
            x: self.f32_range(range),
            y: self.f32_range(range),
        }
    }
    pub fn vec_coarse(&mut self) -> c2v {
        c2v {
            x: self.f32_coarse(),
            y: self.f32_coarse(),
        }
    }
}

/// Interesting scalar edge cases.
pub fn special_f32() -> Vec<f32> {
    vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(1), // smallest subnormal
        -f32::from_bits(1),
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        1e-30,
        1e30,
        3.869416,
        -3.869416,
        13.0693407,
        11.5,
        0.875,
    ]
}

/// Interesting vectors built from `special_f32` plus a few normals.
pub fn special_vecs() -> Vec<c2v> {
    let s = special_f32();
    let mut out = Vec::new();
    for &x in &s {
        for &y in &s {
            out.push(c2v { x, y });
        }
    }
    out
}
