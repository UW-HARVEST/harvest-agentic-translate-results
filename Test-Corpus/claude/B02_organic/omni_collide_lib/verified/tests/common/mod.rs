//! Shared infrastructure for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as *shared objects* through `libloading` and
//! called through real `extern "C"` function pointers, so the `#[no_mangle]`
//! export wrappers and the System V AMD64 struct-passing ABI are part of what
//! gets tested.  No Rust function is ever called directly.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use libloading::Library;
use std::os::raw::{c_int, c_uint, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// C types (must match c_src/src/lib.c byte for byte)
// ---------------------------------------------------------------------------

pub type C2_TYPE = c_uint;
pub const C2_TYPE_CAPSULE: C2_TYPE = 0;
pub const C2_TYPE_CIRCLE: C2_TYPE = 1;
pub const C2_TYPE_AABB: C2_TYPE = 2;

pub const ALL_TYPES: [C2_TYPE; 3] = [C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB];

/// Values with no valid enum variant, passed across the FFI boundary.
pub const BAD_TYPES: [C2_TYPE; 7] = [3, 4, 7, 255, 0x1000, 0x7FFF_FFFF, 0xFFFF_FFFF];

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
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
#[derive(Clone, Copy, Debug, Default)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

impl Default for c2Proxy {
    fn default() -> Self {
        c2Proxy {
            radius: 0.0,
            count: 0,
            verts: [c2v::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

// Layout must match gcc's.  Every field is 4-byte, so there is no padding
// anywhere and a raw byte compare is meaningful.
const _: () = {
    assert!(std::mem::size_of::<c2v>() == 8);
    assert!(std::mem::size_of::<c2r>() == 8);
    assert!(std::mem::size_of::<c2x>() == 16);
    assert!(std::mem::size_of::<c2Circle>() == 12);
    assert!(std::mem::size_of::<c2AABB>() == 16);
    assert!(std::mem::size_of::<c2Capsule>() == 20);
    assert!(std::mem::size_of::<c2GJKCache>() == 36);
    assert!(std::mem::size_of::<c2Proxy>() == 72);
    assert!(std::mem::size_of::<c2sv>() == 36);
    assert!(std::mem::size_of::<c2Simplex>() == 152);
};

pub const FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7;
pub const FLT_MAX: f32 = f32::MAX;

// ---------------------------------------------------------------------------
// Function pointer types
// ---------------------------------------------------------------------------

pub type FnV = extern "C" fn(f32, f32) -> c2v;
pub type FnVv = extern "C" fn(c2v) -> c2v;
pub type FnVvv = extern "C" fn(c2v, c2v) -> c2v;
pub type FnVvvv = extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnVvF = extern "C" fn(c2v, f32) -> c2v;
pub type FnFvv = extern "C" fn(c2v, c2v) -> f32;
pub type FnFv = extern "C" fn(c2v) -> f32;
pub type FnR = extern "C" fn() -> c2r;
pub type FnX = extern "C" fn() -> c2x;
pub type FnVrv = extern "C" fn(c2r, c2v) -> c2v;
pub type FnVxv = extern "C" fn(c2x, c2v) -> c2v;
pub type FnBBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const c_void, C2_TYPE, *mut c2Proxy);
pub type FnSimplexF = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type FnSimplexV = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type FnSimplexVoid = unsafe extern "C" fn(*mut c2Simplex);
pub type FnSupport = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
pub type FnWitness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
#[allow(clippy::type_complexity)]
pub type FnGJK = unsafe extern "C" fn(
    *const c_void,
    C2_TYPE,
    *const c2x,
    *const c_void,
    C2_TYPE,
    *const c2x,
    *mut c2v,
    *mut c2v,
    c_int,
    *mut c_int,
    *mut c2GJKCache,
) -> f32;
pub type FnAABBtoAABB = extern "C" fn(c2AABB, c2AABB) -> c_int;
pub type FnAABBtoCapsule = extern "C" fn(c2AABB, c2Capsule) -> c_int;
pub type FnCapsuletoCapsule = extern "C" fn(c2Capsule, c2Capsule) -> c_int;
pub type FnCircletoCircle = extern "C" fn(c2Circle, c2Circle) -> c_int;
pub type FnCircletoAABB = extern "C" fn(c2Circle, c2AABB) -> c_int;
pub type FnCircletoCapsule = extern "C" fn(c2Circle, c2Capsule) -> c_int;
pub type FnCollided =
    unsafe extern "C" fn(*const c_void, C2_TYPE, *const c_void, C2_TYPE) -> c_int;
pub type FnPtrFromParts =
    unsafe extern "C" fn(C2_TYPE, f32, f32, f32, f32, f32) -> *mut c_void;
#[allow(clippy::type_complexity)]
pub type FnOmniCollide = unsafe extern "C" fn(
    C2_TYPE,
    f32,
    f32,
    f32,
    f32,
    f32,
    C2_TYPE,
    f32,
    f32,
    f32,
    f32,
    f32,
) -> c_int;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Directory the running test binary lives under's parent, i.e.
/// `target/debug` or `target/release` — where cargo puts the cdylib.
fn artifact_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    // .../target/<profile>/deps/<test>-<hash>
    let deps = exe.parent()?;
    Some(deps.parent()?.to_path_buf())
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C2_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C2_RUST_SO") {
        return PathBuf::from(p);
    }
    if let Some(d) = artifact_dir() {
        let p = d.join("libomni_collide_lib.so");
        if p.exists() {
            return p;
        }
    }
    for profile in ["release", "debug"] {
        let p = manifest_dir()
            .join("target")
            .join(profile)
            .join("libomni_collide_lib.so");
        if p.exists() {
            return p;
        }
    }
    manifest_dir().join("target/release/libomni_collide_lib.so")
}

static LIBS: OnceLock<(&'static Library, &'static Library)> = OnceLock::new();

pub fn libs() -> (&'static Library, &'static Library) {
    *LIBS.get_or_init(|| {
        let cp = c_so_path();
        let rp = rust_so_path();
        // Guard against the catastrophic harness bug of comparing one library
        // against itself, which would make every test vacuously green.
        let cc = cp.canonicalize().unwrap_or_else(|e| {
            panic!("C .so {} is not readable: {e}", cp.display())
        });
        let rc = rp.canonicalize().unwrap_or_else(|e| {
            panic!("Rust .so {} is not readable: {e}", rp.display())
        });
        assert_ne!(
            cc, rc,
            "the C and Rust .so resolve to the SAME FILE ({}) -- the harness would \
             not be differential",
            cc.display()
        );
        // If the caller pinned a specific Rust .so, that must be the one used.
        if let Ok(want) = std::env::var("C2_RUST_SO") {
            let want = PathBuf::from(want).canonicalize().expect("C2_RUST_SO not readable");
            assert_eq!(want, rc, "C2_RUST_SO was ignored");
        }
        let c = unsafe { Library::new(&cc) }
            .unwrap_or_else(|e| panic!("cannot load C .so {}: {e}", cc.display()));
        let r = unsafe { Library::new(&rc) }
            .unwrap_or_else(|e| panic!("cannot load Rust .so {}: {e}", rc.display()));
        eprintln!("[diff] C   .so = {}", cc.display());
        eprintln!("[diff] Rust .so = {}", rc.display());
        (
            Box::leak(Box::new(c)) as &'static Library,
            Box::leak(Box::new(r)) as &'static Library,
        )
    })
}

/// Fetch the same symbol from both libraries as a pair of function pointers.
/// The libraries are leaked, so the returned pointers are valid forever.
#[macro_export]
macro_rules! fnpair {
    ($name:literal, $t:ty) => {{
        let (clib, rlib) = $crate::common::libs();
        unsafe {
            let cs: libloading::Symbol<$t> = clib
                .get(concat!($name, "\0").as_bytes())
                .unwrap_or_else(|e| panic!("C .so missing symbol {}: {e}", $name));
            let rs: libloading::Symbol<$t> = rlib
                .get(concat!($name, "\0").as_bytes())
                .unwrap_or_else(|e| panic!("Rust .so missing symbol {}: {e}", $name));
            (*cs, *rs)
        }
    }};
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub fn raw<T>(v: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v as *const T as *const u8, std::mem::size_of::<T>()) }
}

#[track_caller]
pub fn eq_raw<T: std::fmt::Debug>(ctx: &str, c: &T, r: &T) {
    if raw(c) != raw(r) {
        panic!(
            "DIVERGENCE {ctx}\n  C bytes = {:02x?}\n  R bytes = {:02x?}\n  C value = {c:?}\n  R value = {r:?}",
            raw(c),
            raw(r)
        );
    }
}

#[track_caller]
pub fn eq_f32(ctx: &str, c: f32, r: f32) {
    if c.to_bits() != r.to_bits() {
        panic!(
            "DIVERGENCE {ctx}\n  C = {c} (bits 0x{:08x})\n  R = {r} (bits 0x{:08x})",
            c.to_bits(),
            r.to_bits()
        );
    }
}

#[track_caller]
pub fn eq_int(ctx: &str, c: c_int, r: c_int) {
    assert_eq!(c, r, "DIVERGENCE {ctx}");
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64)
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    pub fn u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn u32(&mut self) -> u32 {
        (self.u64() >> 32) as u32
    }

    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u32) -> u32 {
        self.u32() % n
    }

    pub fn bool(&mut self) -> bool {
        self.u64() & 1 == 1
    }

    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let t = (self.u32() >> 8) as f32 / (1u32 << 24) as f32;
        lo + (hi - lo) * t
    }

    /// "Nice" coordinate: small magnitude, plenty of exact values.
    pub fn coord(&mut self) -> f32 {
        match self.below(6) {
            0 => self.range(-10.0, 10.0),
            1 => self.range(-1.0, 1.0),
            2 => (self.below(21) as f32) - 10.0,
            3 => self.range(-1000.0, 1000.0),
            4 => (self.below(9) as f32) * 0.5 - 2.0,
            _ => self.range(-100.0, 100.0),
        }
    }

    /// Non-negative radius, sometimes 0 / tiny / huge.
    pub fn radius(&mut self) -> f32 {
        match self.below(8) {
            0 => 0.0,
            1 => -0.0,
            2 => FLT_EPSILON * self.range(0.0, 2.0),
            3 => self.range(0.0, 1e-20),
            4 => self.range(0.0, 1e20),
            5 => (self.below(5) as f32) * 0.5,
            _ => self.range(0.0, 20.0),
        }
    }

    /// Any float at all, including specials.
    pub fn any_f32(&mut self) -> f32 {
        match self.below(10) {
            0 => SPECIALS[self.below(SPECIALS.len() as u32) as usize],
            1 => f32::from_bits(self.u32()),
            2 => self.range(-1.0, 1.0),
            3 => self.range(-1e30, 1e30),
            4 => (self.below(41) as f32) - 20.0,
            5 => self.range(-1e-30, 1e-30),
            _ => self.coord(),
        }
    }

    /// Random float that is guaranteed finite (no inf/NaN).
    pub fn finite_f32(&mut self) -> f32 {
        loop {
            let v = self.any_f32();
            if v.is_finite() {
                return v;
            }
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

    pub fn finite_v(&mut self) -> c2v {
        c2v {
            x: self.finite_f32(),
            y: self.finite_f32(),
        }
    }

    /// A `c2r`: sometimes a real rotation, sometimes arbitrary.
    pub fn r(&mut self) -> c2r {
        match self.below(6) {
            0 => c2r { c: 1.0, s: 0.0 },
            1 => {
                let th = self.range(-7.0, 7.0);
                c2r {
                    c: th.cos(),
                    s: th.sin(),
                }
            }
            2 => c2r {
                c: self.coord(),
                s: self.coord(),
            },
            3 => c2r { c: 0.0, s: 1.0 },
            4 => c2r { c: -1.0, s: 0.0 },
            _ => {
                let th = self.range(0.0, 6.283_185_5);
                c2r {
                    c: th.cos(),
                    s: th.sin(),
                }
            }
        }
    }

    pub fn x(&mut self) -> c2x {
        match self.below(5) {
            0 => c2x {
                p: c2v { x: 0.0, y: 0.0 },
                r: c2r { c: 1.0, s: 0.0 },
            },
            1 => c2x {
                p: self.v(),
                r: c2r { c: 1.0, s: 0.0 },
            },
            2 => c2x {
                p: c2v { x: 0.0, y: 0.0 },
                r: self.r(),
            },
            _ => c2x {
                p: self.v(),
                r: self.r(),
            },
        }
    }

    pub fn circle(&mut self) -> c2Circle {
        c2Circle {
            p: self.v(),
            r: self.radius(),
        }
    }

    pub fn aabb(&mut self) -> c2AABB {
        let a = self.v();
        let b = self.v();
        // Mostly well-formed, occasionally inverted / degenerate.
        match self.below(8) {
            0 => c2AABB { min: a, max: a },
            1 => c2AABB { min: b, max: a },
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
        let b = if self.below(8) == 0 { a } else { self.v() };
        c2Capsule {
            a,
            b,
            r: self.radius(),
        }
    }
}

pub const SPECIALS: [f32; 22] = [
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    1e-30,
    1e30,
    -1e30,
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    FLT_EPSILON,
    -FLT_EPSILON,
    1.0e8,
    -1.0e8,
];

/// Subnormal / oddly-payloaded values used for the boundary sweeps.
pub const ODDBALLS: [u32; 10] = [
    0x0000_0001, // smallest subnormal
    0x0080_0000, // smallest normal
    0x007F_FFFF, // largest subnormal
    0x7F7F_FFFF, // FLT_MAX
    0x7F80_0000, // +inf
    0x7FC0_0000, // default qNaN
    0x7FC0_1234, // qNaN, other payload
    0xFFC0_0000, // -qNaN
    0x7F80_0001, // sNaN
    0x3F80_0000, // 1.0
];

// ---------------------------------------------------------------------------
// Shape helpers
// ---------------------------------------------------------------------------

/// Build a "parts" tuple (5 floats) that `ptr_from_parts` turns into the shape.
pub fn circle_parts(c: &c2Circle) -> [f32; 5] {
    [c.p.x, c.p.y, c.r, 0.0, 0.0]
}
pub fn aabb_parts(b: &c2AABB) -> [f32; 5] {
    [b.min.x, b.min.y, b.max.x, b.max.y, 0.0]
}
pub fn capsule_parts(c: &c2Capsule) -> [f32; 5] {
    [c.a.x, c.a.y, c.b.x, c.b.y, c.r]
}

/// Random shape of the given type, returned as the 5 `omni_collide` floats.
pub fn random_parts(rng: &mut Rng, t: C2_TYPE) -> [f32; 5] {
    match t {
        C2_TYPE_CIRCLE => circle_parts(&rng.circle()),
        C2_TYPE_AABB => aabb_parts(&rng.aabb()),
        C2_TYPE_CAPSULE => capsule_parts(&rng.capsule()),
        _ => [
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
        ],
    }
}
