//! Shared differential-test harness.
//!
//! BOTH the C `.so` and the Rust `.so` are loaded with `libloading` and driven
//! exclusively through their exported C symbols -- no Rust function is ever
//! called directly, so the `#[no_mangle] extern "C"` wrappers and the SysV
//! struct-passing ABI are part of what is under test.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::os::raw::c_void;
use std::path::PathBuf;

/* ------------------------------------------------------------------ types */

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2Raycast {
    pub t: f32,
    pub n: c2v,
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
pub struct c2Ray {
    pub p: c2v,
    pub d: c2v,
    pub t: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct c2m {
    pub x: c2v,
    pub y: c2v,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

pub fn v(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

/// The largest representable `f32` strictly below `x` (works for both signs).
pub fn next_down(x: f32) -> f32 {
    if x.is_nan() || x == f32::NEG_INFINITY {
        return x;
    }
    if x == 0.0 {
        return -f32::from_bits(1);
    }
    let b = x.to_bits();
    f32::from_bits(if x > 0.0 { b - 1 } else { b + 1 })
}

/// The smallest representable `f32` strictly above `x`.
pub fn next_up(x: f32) -> f32 {
    if x.is_nan() || x == f32::INFINITY {
        return x;
    }
    if x == 0.0 {
        return f32::from_bits(1);
    }
    let b = x.to_bits();
    f32::from_bits(if x > 0.0 { b + 1 } else { b - 1 })
}

/* --------------------------------------------------- bit-exact comparison */

/// Every compared value is reduced to a `Vec<u32>` of raw bit patterns so that
/// NaN payloads, `-0.0` vs `+0.0` and every other IEEE-754 detail is compared
/// exactly rather than by numeric equality.
pub trait Bits {
    fn bits(&self) -> Vec<u32>;
}

impl Bits for f32 {
    fn bits(&self) -> Vec<u32> {
        vec![self.to_bits()]
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
}
impl Bits for c2Raycast {
    fn bits(&self) -> Vec<u32> {
        vec![self.t.to_bits(), self.n.x.to_bits(), self.n.y.to_bits()]
    }
}
impl<A: Bits, B: Bits> Bits for (A, B) {
    fn bits(&self) -> Vec<u32> {
        let mut r = self.0.bits();
        r.extend(self.1.bits());
        r
    }
}

/* --------------------------------------------------------- symbol tables */

macro_rules! decl_syms {
    ($( $name:ident : $ty:ty ),* $(,)?) => {
        pub struct Syms<'a> {
            $( pub $name: Symbol<'a, $ty>, )*
        }
        impl<'a> Syms<'a> {
            fn load(l: &'a Library) -> Self {
                unsafe {
                    Syms {
                        $( $name: l.get(concat!(stringify!($name), "\0").as_bytes())
                                   .unwrap_or_else(|e| panic!(
                                       "missing symbol {}: {e}", stringify!($name))), )*
                    }
                }
            }
        }
    };
}

decl_syms! {
    c2V:              unsafe extern "C" fn(f32, f32) -> c2v,
    c2Dot:            unsafe extern "C" fn(c2v, c2v) -> f32,
    c2Len:            unsafe extern "C" fn(c2v) -> f32,
    c2Add:            unsafe extern "C" fn(c2v, c2v) -> c2v,
    c2Sub:            unsafe extern "C" fn(c2v, c2v) -> c2v,
    c2Mulvs:          unsafe extern "C" fn(c2v, f32) -> c2v,
    c2Div:            unsafe extern "C" fn(c2v, f32) -> c2v,
    c2Norm:           unsafe extern "C" fn(c2v) -> c2v,
    c2Minv:           unsafe extern "C" fn(c2v, c2v) -> c2v,
    c2Maxv:           unsafe extern "C" fn(c2v, c2v) -> c2v,
    c2Skew:           unsafe extern "C" fn(c2v) -> c2v,
    c2Absv:           unsafe extern "C" fn(c2v) -> c2v,
    c2CCW90:          unsafe extern "C" fn(c2v) -> c2v,
    c2MulmvT:         unsafe extern "C" fn(c2m, c2v) -> c2v,
    c2AABBtoAABB:     unsafe extern "C" fn(c2AABB, c2AABB) -> c_int,
    c2AABBtoPoint:    unsafe extern "C" fn(c2AABB, c2v) -> c_int,
    c2CircleToPoint:  unsafe extern "C" fn(c2Circle, c2v) -> c_int,
    c2RaytoCircle:    unsafe extern "C" fn(c2Ray, c2Circle, *mut c2Raycast) -> c_int,
    c2RaytoAABB:      unsafe extern "C" fn(c2Ray, c2AABB, *mut c2Raycast) -> c_int,
    c2RaytoCapsule:   unsafe extern "C" fn(c2Ray, c2Capsule, *mut c2Raycast) -> c_int,
    c2CastRay:        unsafe extern "C" fn(c2Ray, *const c_void, c_int, *mut c2Raycast) -> c_int,
    spec_ray:         unsafe extern "C" fn(*mut c2Raycast, f32, f32, f32, f32, f32, f32, f32) -> c_int,
}

/// The `.so` sentinel written into `*out` before every call, so that "the
/// callee did not touch `out`" is itself an observable, compared output.
pub const SENTINEL: c2Raycast = c2Raycast {
    t: -12345.0,
    n: c2v {
        x: -23456.0,
        y: -34567.0,
    },
};

pub struct Pair {
    _c_lib: Box<Library>,
    _r_lib: Box<Library>,
    pub c: Syms<'static>,
    pub r: Syms<'static>,
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent dir")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let dir = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}\nbuild the C library first:\n  cd c_src && mkdir -p build \
                 && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                dir.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {found:?}",
        dir.display()
    );
    found.pop().unwrap()
}

fn find_rust_so() -> PathBuf {
    // `cargo test` does not itself emit the `cdylib` artifact (it only builds
    // the lib for the unit-test harness), so the `.so` must have been produced
    // by a prior `cargo build`.  An explicit override wins, then the profile
    // the test binary itself was built with, then the other profile.
    if let Ok(p) = std::env::var("RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "RUST_SO={} does not exist", p.display());
        return p;
    }
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let target_dir = profile_dir
        .parent()
        .expect("target/")
        .to_path_buf();
    // Prefer the *release* cdylib: that is the artifact the crate actually
    // ships (`crate-type = ["cdylib"]`, `panic = "abort"`), and it is the one
    // whose behaviour must match the C `.so`.  A debug cdylib additionally
    // carries `-C debug-assertions`, which turns a raw NULL-pointer write into
    // a Rust panic instead of the SIGSEGV the C library produces -- a property
    // of the build profile, not of the translation.
    let mut candidates = vec![target_dir.join("release"), profile_dir.clone()];
    for prof in ["release", "debug"] {
        let d = target_dir.join(prof);
        if !candidates.contains(&d) {
            candidates.push(d);
        }
    }
    for dir in &candidates {
        for name in ["libspec_ray_lib.so", "libtranslation.so"] {
            let p = dir.join(name);
            if p.exists() {
                return p;
            }
        }
    }
    panic!(
        "Rust cdylib not found in any of {candidates:?}\n\
         `cargo test` does not build cdylib artifacts -- run\n  \
         cargo build --offline && cargo build --offline --release\n\
         first (or set RUST_SO=/path/to/libspec_ray_lib.so)."
    );
}

impl Pair {
    pub fn load() -> Self {
        let c_lib = Box::new(unsafe { Library::new(find_c_so()) }.expect("load C .so"));
        let r_lib = Box::new(unsafe { Library::new(find_rust_so()) }.expect("load Rust .so"));
        // SAFETY: the libraries are kept alive for the whole lifetime of `Pair`
        // and are never unloaded before the symbols are dropped.
        let c = Syms::load(unsafe { &*(&*c_lib as *const Library) });
        let r = Syms::load(unsafe { &*(&*r_lib as *const Library) });
        Pair {
            _c_lib: c_lib,
            _r_lib: r_lib,
            c,
            r,
        }
    }
}

/// Global, lazily-loaded library pair (loading the two `.so`s once keeps the
/// randomized sweeps fast).
pub fn libs() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(Pair::load)
}

/* ------------------------------------------------------------- assertions */

#[track_caller]
pub fn same<T: Bits + std::fmt::Debug>(ctx: &str, cv: T, rv: T) {
    let (cb, rb) = (cv.bits(), rv.bits());
    assert_eq!(
        cb, rb,
        "\nDIVERGENCE [{ctx}]\n  C    = {cv:?}  bits={cb:08x?}\n  Rust = {rv:?}  bits={rb:08x?}\n"
    );
}

/// Multiplier for every randomized sweep, from `$DIFF_SCALE` (default 1).
/// `DIFF_SCALE=20 cargo test` runs a ~20x heavier property sweep.
pub fn scale() -> usize {
    std::env::var("DIFF_SCALE")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(1)
}

/// `base` iterations, scaled by `$DIFF_SCALE`.
pub fn iters(base: usize) -> usize {
    base.saturating_mul(scale())
}

/* ------------------------------------------------------------------- PRNG */

/// xorshift64* -- deterministic, seeded, reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x2545_F491_4F6C_DD1D } else { seed })
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
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Uniform `f32` in `[-range, range)`.
    pub fn uniform(&mut self, range: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
        (u * 2.0 - 1.0) * range
    }
    /// A "nice" finite coordinate: mostly moderate, sometimes on a grid.
    pub fn coord(&mut self) -> f32 {
        match self.below(8) {
            0 => self.below(21) as f32 - 10.0,          // small integer
            1 => (self.below(41) as f32 - 20.0) * 0.5,  // half-integer
            2 => self.uniform(1.0e3),
            3 => self.uniform(1.0e-3),
            _ => self.uniform(10.0),
        }
    }
    /// A radius-like non-negative value (occasionally 0).
    pub fn radius(&mut self) -> f32 {
        match self.below(10) {
            0 => 0.0,
            1 => self.below(6) as f32,
            2 => 1.0e-4,
            3 => 1.0e4,
            _ => self.uniform(10.0).abs(),
        }
    }
    /// Fully adversarial scalar covering every IEEE-754 value class,
    /// including raw random bit patterns (so NaN payloads are covered too).
    pub fn wild(&mut self) -> f32 {
        const SPECIALS: [f32; 22] = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            -0.5,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            -f32::NAN,
            f32::MIN_POSITIVE,
            -f32::MIN_POSITIVE,
            f32::MAX,
            f32::MIN,
            1.0e30,
            -1.0e30,
            1.0e-30,
            -1.0e-30,
            3.0,
            -3.0,
            f32::EPSILON,
            16_777_216.0,
        ];
        match self.below(4) {
            0 => SPECIALS[self.below(SPECIALS.len() as u32) as usize],
            1 => f32::from_bits(self.next_u32()), // any bit pattern at all
            2 => f32::from_bits(0x0000_0001 | (self.next_u32() & 0x807f_ffff)), // denormals
            _ => self.coord(),
        }
    }
    pub fn wild_v(&mut self) -> c2v {
        v(self.wild(), self.wild())
    }
    pub fn coord_v(&mut self) -> c2v {
        v(self.coord(), self.coord())
    }
}

/* ------------------------------------------------- differential call sites */

/// Call `c2RaytoCircle` in both libraries with a pre-seeded `*out` and compare
/// the return value **and** the full out-struct bit patterns.
#[track_caller]
pub fn diff_raytocircle(ctx: &str, ray: c2Ray, circle: c2Circle) {
    let l = libs();
    let mut co = SENTINEL;
    let mut ro = SENTINEL;
    let cr = unsafe { (l.c.c2RaytoCircle)(ray, circle, &mut co) };
    let rr = unsafe { (l.r.c2RaytoCircle)(ray, circle, &mut ro) };
    same(
        &format!("c2RaytoCircle {ctx} ray={ray:?} circle={circle:?}"),
        (cr, co),
        (rr, ro),
    );
}

#[track_caller]
pub fn diff_raytoaabb(ctx: &str, ray: c2Ray, bb: c2AABB) {
    let l = libs();
    let mut co = SENTINEL;
    let mut ro = SENTINEL;
    let cr = unsafe { (l.c.c2RaytoAABB)(ray, bb, &mut co) };
    let rr = unsafe { (l.r.c2RaytoAABB)(ray, bb, &mut ro) };
    same(
        &format!("c2RaytoAABB {ctx} ray={ray:?} bb={bb:?}"),
        (cr, co),
        (rr, ro),
    );
}

#[track_caller]
pub fn diff_raytocapsule(ctx: &str, ray: c2Ray, cap: c2Capsule) {
    let l = libs();
    let mut co = SENTINEL;
    let mut ro = SENTINEL;
    let cr = unsafe { (l.c.c2RaytoCapsule)(ray, cap, &mut co) };
    let rr = unsafe { (l.r.c2RaytoCapsule)(ray, cap, &mut ro) };
    same(
        &format!("c2RaytoCapsule {ctx} ray={ray:?} cap={cap:?}"),
        (cr, co),
        (rr, ro),
    );
}

#[track_caller]
pub fn diff_castray_circle(ctx: &str, ray: c2Ray, s: c2Circle) {
    let l = libs();
    let mut co = SENTINEL;
    let mut ro = SENTINEL;
    let p = &s as *const c2Circle as *const c_void;
    let cr = unsafe { (l.c.c2CastRay)(ray, p, C2_TYPE_CIRCLE, &mut co) };
    let rr = unsafe { (l.r.c2CastRay)(ray, p, C2_TYPE_CIRCLE, &mut ro) };
    same(
        &format!("c2CastRay/CIRCLE {ctx} ray={ray:?} s={s:?}"),
        (cr, co),
        (rr, ro),
    );
    // The dispatcher must be indistinguishable from the direct call.
    let mut do_ = SENTINEL;
    let dr = unsafe { (l.c.c2RaytoCircle)(ray, s, &mut do_) };
    same(
        &format!("c2CastRay/CIRCLE == c2RaytoCircle {ctx}"),
        (dr, do_),
        (rr, ro),
    );
}

#[track_caller]
pub fn diff_castray_aabb(ctx: &str, ray: c2Ray, s: c2AABB) {
    let l = libs();
    let mut co = SENTINEL;
    let mut ro = SENTINEL;
    let p = &s as *const c2AABB as *const c_void;
    let cr = unsafe { (l.c.c2CastRay)(ray, p, C2_TYPE_AABB, &mut co) };
    let rr = unsafe { (l.r.c2CastRay)(ray, p, C2_TYPE_AABB, &mut ro) };
    same(
        &format!("c2CastRay/AABB {ctx} ray={ray:?} s={s:?}"),
        (cr, co),
        (rr, ro),
    );
    let mut do_ = SENTINEL;
    let dr = unsafe { (l.c.c2RaytoAABB)(ray, s, &mut do_) };
    same(
        &format!("c2CastRay/AABB == c2RaytoAABB {ctx}"),
        (dr, do_),
        (rr, ro),
    );
}

#[track_caller]
pub fn diff_castray_capsule(ctx: &str, ray: c2Ray, s: c2Capsule) {
    let l = libs();
    let mut co = SENTINEL;
    let mut ro = SENTINEL;
    let p = &s as *const c2Capsule as *const c_void;
    let cr = unsafe { (l.c.c2CastRay)(ray, p, C2_TYPE_CAPSULE, &mut co) };
    let rr = unsafe { (l.r.c2CastRay)(ray, p, C2_TYPE_CAPSULE, &mut ro) };
    same(
        &format!("c2CastRay/CAPSULE {ctx} ray={ray:?} s={s:?}"),
        (cr, co),
        (rr, ro),
    );
    let mut do_ = SENTINEL;
    let dr = unsafe { (l.c.c2RaytoCapsule)(ray, s, &mut do_) };
    same(
        &format!("c2CastRay/CAPSULE == c2RaytoCapsule {ctx}"),
        (dr, do_),
        (rr, ro),
    );
}

#[allow(clippy::too_many_arguments)]
#[track_caller]
pub fn diff_spec_ray(ctx: &str, a: [f32; 7]) {
    let l = libs();
    let mut co = SENTINEL;
    let mut ro = SENTINEL;
    let cr = unsafe { (l.c.spec_ray)(&mut co, a[0], a[1], a[2], a[3], a[4], a[5], a[6]) };
    let rr = unsafe { (l.r.spec_ray)(&mut ro, a[0], a[1], a[2], a[3], a[4], a[5], a[6]) };
    same(
        &format!("spec_ray {ctx} args={a:?}"),
        (cr, co),
        (rr, ro),
    );
}
