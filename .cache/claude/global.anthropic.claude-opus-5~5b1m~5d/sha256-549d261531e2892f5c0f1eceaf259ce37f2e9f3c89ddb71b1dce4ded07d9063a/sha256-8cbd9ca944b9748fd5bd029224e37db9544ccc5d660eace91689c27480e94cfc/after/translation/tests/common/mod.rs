//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` with `libloading` and calls every
//! function through its exported symbol, so the `#[no_mangle]`/`extern "C"`
//! wrappers and the struct-passing ABI are part of what is under test.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI-identical type definitions (mirrors of c_src/include/lib.h + src/lib.c)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2Raycast {
    pub t: f32,
    pub n: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
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

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2Ray {
    pub p: c2v,
    pub d: c2v,
    pub t: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2m {
    pub x: c2v,
    pub y: c2v,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;
pub const C2_TYPE_POLY: c_int = 3;

// ---------------------------------------------------------------------------
// Function-pointer signatures
// ---------------------------------------------------------------------------

pub type FnVff = extern "C" fn(f32, f32) -> c2v;
pub type FnFvv = extern "C" fn(c2v, c2v) -> f32;
pub type FnFv = extern "C" fn(c2v) -> f32;
pub type FnVvv = extern "C" fn(c2v, c2v) -> c2v;
pub type FnVvf = extern "C" fn(c2v, f32) -> c2v;
pub type FnVv = extern "C" fn(c2v) -> c2v;
pub type FnVmv = extern "C" fn(c2m, c2v) -> c2v;
pub type FnVrv = extern "C" fn(c2r, c2v) -> c2v;
pub type FnVxv = extern "C" fn(c2x, c2v) -> c2v;
pub type FnR = extern "C" fn() -> c2r;
pub type FnX = extern "C" fn() -> c2x;
pub type FnIbb = extern "C" fn(c2AABB, c2AABB) -> c_int;
pub type FnIbv = extern "C" fn(c2AABB, c2v) -> c_int;
pub type FnIcv = extern "C" fn(c2Circle, c2v) -> c_int;
pub type FnRayCircle = unsafe extern "C" fn(c2Ray, c2Circle, *mut c2Raycast) -> c_int;
pub type FnRayAABB = unsafe extern "C" fn(c2Ray, c2AABB, *mut c2Raycast) -> c_int;
pub type FnRayCapsule = unsafe extern "C" fn(c2Ray, c2Capsule, *mut c2Raycast) -> c_int;
pub type FnRayPoly = unsafe extern "C" fn(c2Ray, *const c2Poly, *const c2x, *mut c2Raycast) -> c_int;
pub type FnCastRay =
    unsafe extern "C" fn(c2Ray, *const c_void, *const c2x, c_int, *mut c2Raycast) -> c_int;
pub type FnPolyRay = unsafe extern "C" fn(*mut c2Raycast, *mut c2Raycast) -> c_int;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Libs {
    pub c: Library,
    pub r: Library,
    pub c_path: PathBuf,
    pub r_path: PathBuf,
}

/// Locate the C `.so`. Its file name is derived from the *parent directory* name
/// by `c_src/CMakeLists.txt`, so it is discovered by scanning, not hard-coded.
fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.parent().expect("crate has a parent dir");
    let build = root.join("c_src").join("build");
    let mut hits: Vec<PathBuf> = Vec::new();
    let mut stack = vec![build.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().and_then(|s| s.to_str()) == Some("so") {
                hits.push(p);
            }
        }
    }
    assert!(
        !hits.is_empty(),
        "no C .so found under {}. Build it first:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    hits.sort();
    hits.remove(0)
}

/// Locate the Rust `cdylib`. Prefers `release` (what the task builds), falls
/// back to `debug`.
fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let name = "libpoly_ray_lib.so";
    for profile in ["release", "debug"] {
        let p = manifest.join("target").join(profile).join(name);
        if p.exists() {
            return p;
        }
    }
    panic!(
        "no Rust {name} found under {}/target/{{release,debug}}. Run `cargo build --release`.",
        manifest.display()
    );
}

fn open(p: &Path) -> Library {
    unsafe { Library::new(p) }.unwrap_or_else(|e| panic!("dlopen {} failed: {e}", p.display()))
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let r_path = find_rust_so();
        let c = open(&c_path);
        let r = open(&r_path);
        Libs {
            c,
            r,
            c_path,
            r_path,
        }
    })
}

impl Libs {
    /// Fetch the same symbol from both libraries as a raw function pointer.
    pub fn pair<T: Copy>(&self, name: &str) -> (T, T) {
        let mut bytes = name.as_bytes().to_vec();
        bytes.push(0);
        let cf: Symbol<T> = unsafe { self.c.get(&bytes) }
            .unwrap_or_else(|e| panic!("C .so is missing symbol `{name}`: {e}"));
        let rf: Symbol<T> = unsafe { self.r.get(&bytes) }
            .unwrap_or_else(|e| panic!("Rust .so is missing symbol `{name}`: {e}"));
        (*cf, *rf)
    }
}

/// Convenience: `sym!("c2Dot", FnFvv)` -> `(c_fn, rust_fn)`.
#[macro_export]
macro_rules! sym {
    ($name:literal, $ty:ty) => {
        $crate::common::libs().pair::<$ty>($name)
    };
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub fn fb(x: f32) -> u32 {
    x.to_bits()
}

pub fn vb(v: c2v) -> [u32; 2] {
    [v.x.to_bits(), v.y.to_bits()]
}

pub fn rb(r: c2r) -> [u32; 2] {
    [r.c.to_bits(), r.s.to_bits()]
}

pub fn xb(x: c2x) -> [u32; 4] {
    [
        x.p.x.to_bits(),
        x.p.y.to_bits(),
        x.r.c.to_bits(),
        x.r.s.to_bits(),
    ]
}

pub fn cb(c: c2Raycast) -> [u32; 3] {
    [c.t.to_bits(), c.n.x.to_bits(), c.n.y.to_bits()]
}

/// A recognisable non-zero pattern for out-params, so that "C left it untouched
/// but Rust wrote it" (or vice versa) is detected, not masked by zeros.
pub fn dirty() -> c2Raycast {
    c2Raycast {
        t: f32::from_bits(0xDEAD_BEEF),
        n: c2v {
            x: f32::from_bits(0xCAFE_BABE),
            y: f32::from_bits(0x0BAD_F00D),
        },
    }
}

/// Accumulates mismatches so one test run reports many failures at once.
pub struct Diff {
    pub label: &'static str,
    pub checked: u64,
    pub failures: Vec<String>,
}

impl Diff {
    pub fn new(label: &'static str) -> Self {
        Diff {
            label,
            checked: 0,
            failures: Vec::new(),
        }
    }

    pub fn eq<T: PartialEq + std::fmt::Debug>(&mut self, ctx: impl std::fmt::Debug, c: T, r: T) {
        self.checked += 1;
        if c != r {
            if self.failures.len() < 25 {
                self.failures
                    .push(format!("  input {ctx:?}\n    C  = {c:?}\n    RS = {r:?}"));
            }
        }
    }

    /// Compare an `int` return value together with the full out-param bits.
    pub fn ray(
        &mut self,
        ctx: impl std::fmt::Debug,
        cret: c_int,
        cout: c2Raycast,
        rret: c_int,
        rout: c2Raycast,
    ) {
        self.eq(&ctx, (cret, cb(cout)), (rret, cb(rout)));
    }

    pub fn finish(self) {
        assert!(self.checked > 0, "[{}] ran zero comparisons", self.label);
        if !self.failures.is_empty() {
            panic!(
                "[{}] {} mismatch(es) out of {} comparisons:\n{}",
                self.label,
                self.failures.len(),
                self.checked,
                self.failures.join("\n")
            );
        }
        eprintln!("[{}] OK ({} bit-exact comparisons)", self.label, self.checked);
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) + input generators
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x2545_F491_4F6C_DD1D;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { SEED } else { seed })
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
    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in [-mag, mag].
    pub fn range(&mut self, mag: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * mag
    }
    /// A "geometry-sized" coordinate: mostly small, occasionally large/tiny.
    pub fn coord(&mut self) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => self.range(1.0e-6),
            3 => self.range(1.0e6),
            4..=6 => (self.below(41) as f32) - 20.0, // integral, hits exact ties
            _ => self.range(20.0),
        }
    }
    /// Any float, including NaN / inf / denormals / huge magnitudes.
    pub fn any_f32(&mut self) -> f32 {
        match self.below(8) {
            0 => SPECIALS[self.below(SPECIALS.len() as u32) as usize],
            1 => f32::from_bits(self.next_u32()), // arbitrary pattern, may be NaN
            _ => self.coord(),
        }
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
    /// A ray with a mostly-unit direction (occasionally unnormalised or zero).
    pub fn ray(&mut self) -> c2Ray {
        let d = match self.below(8) {
            0 => c2v { x: 0.0, y: 0.0 },
            1 => self.v(),
            2 => {
                let k = self.below(4);
                c2v {
                    x: [1.0, -1.0, 0.0, 0.0][k as usize],
                    y: [0.0, 0.0, 1.0, -1.0][k as usize],
                }
            }
            _ => {
                let a = self.unit() * std::f32::consts::TAU;
                c2v {
                    x: a.cos(),
                    y: a.sin(),
                }
            }
        };
        let t = match self.below(8) {
            0 => 0.0,
            1 => -self.unit() * 10.0,
            2 => f32::INFINITY,
            3 => self.unit() * 1000.0,
            _ => self.unit() * 30.0,
        };
        c2Ray {
            p: self.v(),
            d,
            t,
        }
    }
    pub fn circle(&mut self) -> c2Circle {
        c2Circle {
            p: self.v(),
            r: match self.below(8) {
                0 => 0.0,
                1 => -self.unit() * 5.0,
                _ => self.unit() * 10.0,
            },
        }
    }
    pub fn aabb(&mut self) -> c2AABB {
        let a = self.v();
        let b = self.v();
        match self.below(8) {
            // inverted box (min > max) — C never validates
            0 => c2AABB { min: a, max: b },
            1 => c2AABB { min: a, max: a }, // zero extent
            _ => c2AABB {
                min: c2v {
                    x: a.x.min(b.x),
                    y: a.y.min(b.y),
                },
                max: c2v {
                    x: a.x.max(b.x),
                    y: a.y.max(b.y),
                },
            },
        }
    }
    pub fn capsule(&mut self) -> c2Capsule {
        let a = self.v();
        let b = match self.below(8) {
            0 => a, // degenerate: a == b
            _ => self.v(),
        };
        c2Capsule {
            a,
            b,
            r: match self.below(8) {
                0 => 0.0,
                1 => -self.unit() * 5.0,
                _ => self.unit() * 8.0,
            },
        }
    }
    pub fn rot(&mut self) -> c2r {
        match self.below(8) {
            0 => c2r { c: 1.0, s: 0.0 },
            1 => c2r { c: 0.0, s: 0.0 }, // degenerate, non-unit
            2 => c2r {
                c: self.range(5.0),
                s: self.range(5.0),
            }, // non-unit
            _ => {
                let a = self.unit() * std::f32::consts::TAU;
                c2r {
                    c: a.cos(),
                    s: a.sin(),
                }
            }
        }
    }
    pub fn x(&mut self) -> c2x {
        c2x {
            p: self.v(),
            r: self.rot(),
        }
    }
    /// A random polygon with a random `count` in `1..=8`. Verts and norms are
    /// deliberately *not* always geometrically consistent — the C never
    /// validates them, so inconsistent input is valid input.
    pub fn poly(&mut self) -> c2Poly {
        let n = 1 + self.below(8) as usize;
        if self.below(4) == 0 {
            let mut p = zero_poly();
            p.count = n as c_int;
            for i in 0..n {
                p.verts[i] = self.v();
                p.norms[i] = self.v();
            }
            p
        } else {
            let r = 0.5 + self.unit() * 8.0;
            let phase = self.unit() * std::f32::consts::TAU;
            ngon(n, r, phase, self.v())
        }
    }
}

pub const SPECIALS: [f32; 18] = [
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
    1.0e-45, // smallest denormal
    -1.0e-45,
    f32::MAX,
    f32::MIN,
    16_777_216.0, // 2^24, first integer with an unrepresentable successor
    -16_777_216.0,
];

pub fn zero_poly() -> c2Poly {
    let z = c2v { x: 0.0, y: 0.0 };
    c2Poly {
        count: 0,
        verts: [z; 8],
        norms: [z; 8],
    }
}

/// Build a convex CCW regular `n`-gon. `norms[i]` is the outward unit normal of
/// the edge `verts[i] -> verts[i+1]`, matching the convention the C's
/// `poly_ray` uses (`norms[i] = normalize(CCW90(v[i+1] - v[i]))`).
pub fn ngon(n: usize, radius: f32, phase: f32, center: c2v) -> c2Poly {
    assert!((1..=8).contains(&n));
    let mut p = zero_poly();
    p.count = n as c_int;
    for i in 0..n {
        let a = phase + std::f32::consts::TAU * (i as f32) / (n as f32);
        p.verts[i] = c2v {
            x: center.x + radius * a.cos(),
            y: center.y + radius * a.sin(),
        };
    }
    for i in 0..n {
        let j = (i + 1) % n;
        let (dx, dy) = (p.verts[j].x - p.verts[i].x, p.verts[j].y - p.verts[i].y);
        // CCW90(edge) = (edge.y, -edge.x), then normalize.
        let (nx, ny) = (dy, -dx);
        let len = (nx * nx + ny * ny).sqrt();
        p.norms[i] = if len > 0.0 {
            c2v {
                x: nx / len,
                y: ny / len,
            }
        } else {
            c2v { x: 0.0, y: 0.0 }
        };
    }
    p
}

/// The axis-aligned box polygon, in the exact CCW/normal convention of the C's
/// `poly_ray` (verts start bottom-right, norms are +x, +y, -x, -y).
pub fn box_poly(hw: f32, hh: f32) -> c2Poly {
    let mut p = zero_poly();
    p.count = 4;
    p.verts[0] = c2v { x: hw, y: -hh };
    p.verts[1] = c2v { x: hw, y: hh };
    p.verts[2] = c2v { x: -hw, y: hh };
    p.verts[3] = c2v { x: -hw, y: -hh };
    p.norms[0] = c2v { x: 1.0, y: 0.0 };
    p.norms[1] = c2v { x: 0.0, y: 1.0 };
    p.norms[2] = c2v { x: -1.0, y: 0.0 };
    p.norms[3] = c2v { x: 0.0, y: -1.0 };
    p
}

pub fn identity_x() -> c2x {
    c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 1.0, s: 0.0 },
    }
}

pub fn rot_x(theta: f32) -> c2r {
    c2r {
        c: theta.cos(),
        s: theta.sin(),
    }
}

// ---------------------------------------------------------------------------
// Thin differential drivers for the four raycasts + the dispatcher
// ---------------------------------------------------------------------------

pub fn diff_ray_circle(d: &mut Diff, ctx: impl std::fmt::Debug, a: c2Ray, b: c2Circle) {
    let (cf, rf) = sym!("c2RaytoCircle", FnRayCircle);
    let (mut co, mut ro) = (dirty(), dirty());
    let cret = unsafe { cf(a, b, &mut co) };
    let rret = unsafe { rf(a, b, &mut ro) };
    d.ray(ctx, cret, co, rret, ro);
}

pub fn diff_ray_aabb(d: &mut Diff, ctx: impl std::fmt::Debug, a: c2Ray, b: c2AABB) {
    let (cf, rf) = sym!("c2RaytoAABB", FnRayAABB);
    let (mut co, mut ro) = (dirty(), dirty());
    let cret = unsafe { cf(a, b, &mut co) };
    let rret = unsafe { rf(a, b, &mut ro) };
    d.ray(ctx, cret, co, rret, ro);
}

pub fn diff_ray_capsule(d: &mut Diff, ctx: impl std::fmt::Debug, a: c2Ray, b: c2Capsule) {
    let (cf, rf) = sym!("c2RaytoCapsule", FnRayCapsule);
    let (mut co, mut ro) = (dirty(), dirty());
    let cret = unsafe { cf(a, b, &mut co) };
    let rret = unsafe { rf(a, b, &mut ro) };
    d.ray(ctx, cret, co, rret, ro);
}

pub fn diff_ray_poly(
    d: &mut Diff,
    ctx: impl std::fmt::Debug,
    a: c2Ray,
    p: &c2Poly,
    bx: Option<&c2x>,
) {
    let (cf, rf) = sym!("c2RaytoPoly", FnRayPoly);
    let bxp: *const c2x = match bx {
        Some(v) => v,
        None => std::ptr::null(),
    };
    let (mut co, mut ro) = (dirty(), dirty());
    let cret = unsafe { cf(a, p, bxp, &mut co) };
    let rret = unsafe { rf(a, p, bxp, &mut ro) };
    d.ray(ctx, cret, co, rret, ro);
}

pub fn diff_cast_ray(
    d: &mut Diff,
    ctx: impl std::fmt::Debug,
    a: c2Ray,
    shape: *const c_void,
    bx: *const c2x,
    ty: c_int,
) {
    let (cf, rf) = sym!("c2CastRay", FnCastRay);
    let (mut co, mut ro) = (dirty(), dirty());
    let cret = unsafe { cf(a, shape, bx, ty, &mut co) };
    let rret = unsafe { rf(a, shape, bx, ty, &mut ro) };
    d.ray(ctx, cret, co, rret, ro);
}
