//! Shared differential-test harness.
//!
//! BOTH libraries are loaded with `libloading` and every call goes through the
//! dynamic symbol table — the Rust crate is never linked directly, so the
//! `#[no_mangle] extern "C"` wrappers and the SysV struct-passing ABI are part
//! of what is under test.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// ABI-identical mirrors of the C types (from c_src/include/lib.h + src/lib.c)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2Raycast {
    pub t: f32,
    pub n: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2AABB {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2Ray {
    pub p: C2v,
    pub d: C2v,
    pub t: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2m {
    pub x: C2v,
    pub y: C2v,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

// ---------------------------------------------------------------------------
// Function pointer types
// ---------------------------------------------------------------------------

pub type FnV = extern "C" fn(f32, f32) -> C2v;
pub type FnVVf = extern "C" fn(C2v, C2v) -> f32;
pub type FnVf = extern "C" fn(C2v) -> f32;
pub type FnVVV = extern "C" fn(C2v, C2v) -> C2v;
pub type FnVsV = extern "C" fn(C2v, f32) -> C2v;
pub type FnVV = extern "C" fn(C2v) -> C2v;
pub type FnMvV = extern "C" fn(C2m, C2v) -> C2v;
pub type FnAABB2 = extern "C" fn(C2AABB, C2AABB) -> c_int;
pub type FnAABBPt = extern "C" fn(C2AABB, C2v) -> c_int;
pub type FnCirPt = extern "C" fn(C2Circle, C2v) -> c_int;
pub type FnRayCircle = unsafe extern "C" fn(C2Ray, C2Circle, *mut C2Raycast) -> c_int;
pub type FnRayAABB = unsafe extern "C" fn(C2Ray, C2AABB, *mut C2Raycast) -> c_int;
pub type FnRayCapsule = unsafe extern "C" fn(C2Ray, C2Capsule, *mut C2Raycast) -> c_int;
pub type FnCastRay = unsafe extern "C" fn(C2Ray, *const c_void, c_int, *mut C2Raycast) -> c_int;
#[rustfmt::skip]
pub type FnGenRay = unsafe extern "C" fn(
    *mut C2Raycast, *mut C2Raycast, *mut C2Raycast,
    f32, f32, f32, f32, f32, f32, f32,
    f32, f32, f32, f32, f32,
    f32, f32, f32, f32,
) -> c_int;

// ---------------------------------------------------------------------------
// One loaded implementation
// ---------------------------------------------------------------------------

pub struct Impl {
    _lib: Library,
    pub name: &'static str,
    pub c2V: FnV,
    pub c2Dot: FnVVf,
    pub c2Len: FnVf,
    pub c2Add: FnVVV,
    pub c2Sub: FnVVV,
    pub c2Mulvs: FnVsV,
    pub c2Div: FnVsV,
    pub c2Norm: FnVV,
    pub c2Minv: FnVVV,
    pub c2Maxv: FnVVV,
    pub c2Skew: FnVV,
    pub c2Absv: FnVV,
    pub c2CCW90: FnVV,
    pub c2MulmvT: FnMvV,
    pub c2AABBtoAABB: FnAABB2,
    pub c2AABBtoPoint: FnAABBPt,
    pub c2CircleToPoint: FnCirPt,
    pub c2RaytoCircle: FnRayCircle,
    pub c2RaytoAABB: FnRayAABB,
    pub c2RaytoCapsule: FnRayCapsule,
    pub c2CastRay: FnCastRay,
    pub gen_ray: FnGenRay,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &str) -> T {
    let s: Symbol<T> = lib
        .get(name.as_bytes())
        .unwrap_or_else(|e| panic!("symbol `{name}` not found: {e}"));
    *s
}

impl Impl {
    pub unsafe fn load(path: &PathBuf, name: &'static str) -> Impl {
        let lib = Library::new(path)
            .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display()));
        Impl {
            name,
            c2V: sym(&lib, "c2V"),
            c2Dot: sym(&lib, "c2Dot"),
            c2Len: sym(&lib, "c2Len"),
            c2Add: sym(&lib, "c2Add"),
            c2Sub: sym(&lib, "c2Sub"),
            c2Mulvs: sym(&lib, "c2Mulvs"),
            c2Div: sym(&lib, "c2Div"),
            c2Norm: sym(&lib, "c2Norm"),
            c2Minv: sym(&lib, "c2Minv"),
            c2Maxv: sym(&lib, "c2Maxv"),
            c2Skew: sym(&lib, "c2Skew"),
            c2Absv: sym(&lib, "c2Absv"),
            c2CCW90: sym(&lib, "c2CCW90"),
            c2MulmvT: sym(&lib, "c2MulmvT"),
            c2AABBtoAABB: sym(&lib, "c2AABBtoAABB"),
            c2AABBtoPoint: sym(&lib, "c2AABBtoPoint"),
            c2CircleToPoint: sym(&lib, "c2CircleToPoint"),
            c2RaytoCircle: sym(&lib, "c2RaytoCircle"),
            c2RaytoAABB: sym(&lib, "c2RaytoAABB"),
            c2RaytoCapsule: sym(&lib, "c2RaytoCapsule"),
            c2CastRay: sym(&lib, "c2CastRay"),
            gen_ray: sym(&lib, "gen_ray"),
            _lib: lib,
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // translation/  ->  parent is the working directory holding c_src/
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
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
    found.pop().unwrap_or_else(|| {
        panic!(
            "no .so in {} — build the C library first (cmake .. && cmake --build .)",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    // current_exe is target/<profile>/deps/<test bin>; the cdylib sits in
    // target/<profile>/, so we test the SAME profile the harness was built with.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let candidate = profile_dir.join("libgen_ray_lib.so");
    if candidate.exists() {
        return candidate;
    }
    for prof in ["release", "debug"] {
        let c = workspace_root()
            .join("translation/target")
            .join(prof)
            .join("libgen_ray_lib.so");
        if c.exists() {
            return c;
        }
    }
    panic!("libgen_ray_lib.so not found; run `cargo build` first");
}

/// The pair under test.
pub struct Pair {
    pub c: Impl,
    pub r: Impl,
}

pub fn libs() -> &'static Pair {
    use std::sync::OnceLock;
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| unsafe {
        Pair {
            c: Impl::load(&find_c_so(), "C"),
            r: Impl::load(&find_rust_so(), "Rust"),
        }
    })
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub fn fb(x: f32) -> u32 {
    x.to_bits()
}

pub fn vb(v: C2v) -> (u32, u32) {
    (v.x.to_bits(), v.y.to_bits())
}

pub fn rcb(r: C2Raycast) -> (u32, u32, u32) {
    (r.t.to_bits(), r.n.x.to_bits(), r.n.y.to_bits())
}

/// Sentinel used to pre-fill `*out` so untouched bytes are compared as well.
pub const SENTINEL: C2Raycast = C2Raycast {
    t: f32::from_bits(0xDEAD_BEEF),
    n: C2v {
        x: f32::from_bits(0xCAFE_BABE),
        y: f32::from_bits(0x1234_5678),
    },
};

#[macro_export]
macro_rules! diff_eq {
    ($ctx:expr, $cv:expr, $rv:expr) => {{
        let cv = $cv;
        let rv = $rv;
        if cv != rv {
            panic!(
                "DIVERGENCE [{}]\n  C    = {:?}\n  Rust = {:?}",
                $ctx, cv, rv
            );
        }
    }};
}

// ---------------------------------------------------------------------------
// Deterministic RNG (PCG-XSH-RR 64/32) — fixed seed per test for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng {
    state: u64,
    inc: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        let mut r = Rng {
            state: 0,
            inc: (seed << 1) | 1,
        };
        r.next_u32();
        r.state = r.state.wrapping_add(seed);
        r.next_u32();
        r
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old
            .wrapping_mul(6364136223846793005)
            .wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    /// Uniform in [0,1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in [-a, a).
    pub fn sym(&mut self, a: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * a
    }

    /// Any f32 bit pattern, including NaNs, infinities and denormals.
    pub fn any_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// Random sign, random exponent within `10^±exp_range`, random mantissa.
    pub fn wide(&mut self, exp_range: i32) -> f32 {
        let e = (self.next_u32() as i32 % (2 * exp_range + 1)) - exp_range;
        let m = self.unit() * 9.0 + 1.0;
        let s = if self.next_u32() & 1 == 0 { 1.0 } else { -1.0 };
        s * m * 10f32.powi(e)
    }

    /// A value from the "interesting classes" set, or a plain random one.
    pub fn special(&mut self, scale: f32) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::NAN,
            5 => -f32::NAN,
            6 => f32::from_bits(0x7FC0_1234), // NaN, non-default payload
            7 => f32::from_bits(0xFFC0_1234), // negative NaN, payload
            8 => f32::MIN_POSITIVE,
            9 => -f32::MIN_POSITIVE,
            10 => f32::from_bits(1), // smallest denormal
            11 => f32::MAX,
            12 => f32::MIN,
            13 => 1.0,
            14 => -1.0,
            _ => self.sym(scale),
        }
    }

    pub fn v(&mut self, scale: f32) -> C2v {
        C2v {
            x: self.sym(scale),
            y: self.sym(scale),
        }
    }

    pub fn v_special(&mut self, scale: f32) -> C2v {
        C2v {
            x: self.special(scale),
            y: self.special(scale),
        }
    }

    pub fn unit_dir(&mut self) -> C2v {
        let a = self.unit() * std::f32::consts::TAU;
        C2v {
            x: a.cos(),
            y: a.sin(),
        }
    }
}

/// The canonical "interesting" scalar set, used for exhaustive small crosses.
pub const SPECIALS: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    3.0,
    1e-30,
    -1e-30,
    1e30,
    -1e30,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
];

pub fn specials_nan_payloads() -> Vec<f32> {
    let mut v = SPECIALS.to_vec();
    v.push(-f32::NAN);
    v.push(f32::from_bits(0x7FC0_1234));
    v.push(f32::from_bits(0xFFC0_1234));
    v.push(f32::from_bits(0x7F80_0001)); // signalling NaN
    v.push(f32::from_bits(1));
    v.push(f32::from_bits(0x8000_0001));
    v
}
