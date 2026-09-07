//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading`; every call
//! goes through `dlsym`, so the `#[no_mangle]` export wrappers are exercised
//! exactly as an external C consumer would exercise them. Nothing in the crate
//! under test is ever called directly.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Types — layout-identical to c_src/src/lib.c
// ---------------------------------------------------------------------------

pub const C2_TYPE_CAPSULE: c_int = 0;
pub const C2_TYPE_CIRCLE: c_int = 1;
pub const C2_TYPE_AABB: c_int = 2;
pub const C2_TYPE_POLY: c_int = 3;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

pub fn v(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct c2Manifold {
    pub count: c_int,
    pub depths: [f32; 2],
    pub contact_points: [c2v; 2],
    pub n: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct c2h {
    pub n: c2v,
    pub d: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
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
#[derive(Clone, Copy, Default, Debug)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

/// C: `struct c2Simplex { c2sv a, b, c, d; float div; int count; }`
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

// ---------------------------------------------------------------------------
// Function pointer types
// ---------------------------------------------------------------------------

pub type FnVff = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnVvf = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnVvv = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnVvvv = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnFvv = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnVv = unsafe extern "C" fn(c2v) -> c2v;
pub type FnFv = unsafe extern "C" fn(c2v) -> f32;
pub type FnFhv = unsafe extern "C" fn(c2h, c2v) -> f32;
pub type FnHpi = unsafe extern "C" fn(*const c2Poly, c_int) -> c2h;
pub type FnR = unsafe extern "C" fn() -> c2r;
pub type FnX = unsafe extern "C" fn() -> c2x;
pub type FnBBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
pub type FnFsimplex = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type FnVsimplex = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type FnSimplex = unsafe extern "C" fn(*mut c2Simplex);
pub type FnVrv = unsafe extern "C" fn(c2r, c2v) -> c2v;
pub type FnVxv = unsafe extern "C" fn(c2x, c2v) -> c2v;
pub type FnIntersect = unsafe extern "C" fn(c2v, c2v, f32, f32) -> c2v;
pub type FnSupport = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
pub type FnWitness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
pub type FnNorms = unsafe extern "C" fn(*mut c2v, *mut c2v, c_int);
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
pub type FnCircleCircle = unsafe extern "C" fn(c2Circle, c2Circle, *mut c2Manifold);
pub type FnCircleAABB = unsafe extern "C" fn(c2Circle, c2AABB, *mut c2Manifold);
pub type FnCircleCapsule = unsafe extern "C" fn(c2Circle, c2Capsule, *mut c2Manifold);
pub type FnAABBAABB = unsafe extern "C" fn(c2AABB, c2AABB, *mut c2Manifold);
pub type FnAABBCapsule = unsafe extern "C" fn(c2AABB, c2Capsule, *mut c2Manifold);
pub type FnCapsuleCapsule = unsafe extern "C" fn(c2Capsule, c2Capsule, *mut c2Manifold);
pub type FnCapsulePoly =
    unsafe extern "C" fn(c2Capsule, *const c2Poly, *const c2x, *mut c2Manifold);
pub type FnCollide =
    unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int, *mut c2Manifold);
pub type FnPtrFromParts =
    unsafe extern "C" fn(c_int, f32, f32, f32, f32, f32) -> *mut c_void;
pub type FnOmni = unsafe extern "C" fn(
    *mut c2Manifold,
    c_int,
    f32,
    f32,
    f32,
    f32,
    f32,
    c_int,
    f32,
    f32,
    f32,
    f32,
    f32,
);

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

impl Libs {
    /// Resolve one symbol out of both libraries. Panics with a useful message
    /// if either library lacks it (that is itself a parity failure).
    pub fn pair<T>(&'static self, name: &str) -> (Symbol<'static, T>, Symbol<'static, T>) {
        let mut owned = name.as_bytes().to_vec();
        owned.push(0);
        let cs: Symbol<'static, T> = unsafe {
            self.c
                .get(&owned)
                .unwrap_or_else(|e| panic!("C .so is missing `{name}`: {e}"))
        };
        let rs: Symbol<'static, T> = unsafe {
            self.rust
                .get(&owned)
                .unwrap_or_else(|e| panic!("Rust .so is missing `{name}`: {e}"))
        };
        (cs, rs)
    }
}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn find_c_so() -> std::path::PathBuf {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace parent")
        .join("c_src/build");
    let mut found: Vec<std::path::PathBuf> = std::fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", root.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {:?}",
        root.display(),
        found
    );
    found.pop().unwrap()
}

fn find_rust_so() -> std::path::PathBuf {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libomni_manifold_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libomni_manifold_lib.so not found under {}; run `cargo build --release` first",
        base.display()
    );
}

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let cpath = find_c_so();
        let rpath = find_rust_so();
        let c = unsafe { Library::new(&cpath) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", cpath.display()));
        let rust = unsafe { Library::new(&rpath) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", rpath.display()));
        Libs { c, rust }
    })
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub fn feq(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

pub fn veq(a: c2v, b: c2v) -> bool {
    feq(a.x, b.x) && feq(a.y, b.y)
}

pub fn heq(a: c2h, b: c2h) -> bool {
    veq(a.n, b.n) && feq(a.d, b.d)
}

pub fn fs(x: f32) -> String {
    format!("{x:e}(0x{:08x})", x.to_bits())
}

pub fn vs(a: c2v) -> String {
    format!("({}, {})", fs(a.x), fs(a.y))
}

pub fn ms(m: &c2Manifold) -> String {
    format!(
        "count={} depths=[{}, {}] cp=[{}, {}] n={}",
        m.count,
        fs(m.depths[0]),
        fs(m.depths[1]),
        vs(m.contact_points[0]),
        vs(m.contact_points[1]),
        vs(m.n)
    )
}

/// Compares two manifolds. `count` and `n` are always compared; the `depths` /
/// `contact_points` slots are compared for `0..count` and, additionally, the
/// full arrays are compared when both libraries left them at the same
/// initialization (they are seeded identically by the caller, so any divergence
/// in an unused slot is still a real divergence and is reported).
pub fn meq(a: &c2Manifold, b: &c2Manifold) -> bool {
    if a.count != b.count || !veq(a.n, b.n) {
        return false;
    }
    for i in 0..2 {
        if !feq(a.depths[i], b.depths[i]) {
            return false;
        }
        if !veq(a.contact_points[i], b.contact_points[i]) {
            return false;
        }
    }
    true
}

/// Like `feq`, but treats "both NaN" as equal regardless of payload.
///
/// x86 `addss`/`mulss` return the **destination** operand (quieted) when an
/// operand is NaN. gcc `-O0` (the C build) and LLVM `-O3` (the Rust build) make
/// different register-allocation choices for the final add in `c2Dot`, so when
/// a caller feeds in a NaN the resulting NaN can carry a different sign
/// bit. Everything that is not NaN is still compared bit-for-bit.
pub fn feq_nan(a: f32, b: f32) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    a.to_bits() == b.to_bits()
}

pub fn veq_nan(a: c2v, b: c2v) -> bool {
    feq_nan(a.x, b.x) && feq_nan(a.y, b.y)
}

pub fn meq_nan(a: &c2Manifold, b: &c2Manifold) -> bool {
    if a.count != b.count || !veq_nan(a.n, b.n) {
        return false;
    }
    (0..2).all(|i| feq_nan(a.depths[i], b.depths[i]) && veq_nan(a.contact_points[i], b.contact_points[i]))
}

pub fn seeded_manifold(seed: f32) -> c2Manifold {    c2Manifold {
        count: -12345,
        depths: [seed, seed],
        contact_points: [v(seed, seed), v(seed, seed)],
        n: v(seed, seed),
    }
}

// ---------------------------------------------------------------------------
// Stack canonicalization
// ---------------------------------------------------------------------------

/// Zeroes the stack region that the library's frames will occupy.
///
/// This is required for any path that reaches `c2GJK` with `C2_TYPE_POLY`
/// (i.e. `c2CapsuletoPolyManifold`, and therefore `c2AABBtoCapsuleManifold`,
/// `c2Collide(AABB, CAPSULE)` and `omni_manifold(AABB, CAPSULE)`): the C
/// `c2MakeProxy` has **no** `C2_TYPE_POLY` case, so `c2GJK`'s `c2Proxy pB`
/// local is never written and the C code then reads it. Whatever the stack
/// happens to hold at that offset is what the C library uses — with a dirty
/// stack the C library returns garbage and can even segfault (verified: it
/// crashes in `c2Support` when the leftover `pB.count` is large).
///
/// With the region zeroed, the C library was verified to return exactly what
/// the Rust translation's explicit zero-initialization produces. That is the
/// only reproducible precondition, so the differential tests establish it
/// before **every** call into either library.
#[inline(never)]
pub fn scrub_stack() {
    let mut buf = [0u8; 8192];
    for b in buf.iter_mut() {
        *b = 0;
    }
    std::hint::black_box(&mut buf);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed, reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

/// Optional global seed offset, from the `DIFF_SEED` environment variable.
///
/// Every test uses a fixed seed so failures are reproducible. Setting
/// `DIFF_SEED=<n>` perturbs all of them coherently, which lets the same suite be
/// re-run as an independent randomized sweep without editing any test.
fn seed_offset() -> u64 {
    static OFF: OnceLock<u64> = OnceLock::new();
    *OFF.get_or_init(|| {
        std::env::var("DIFF_SEED")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0)
    })
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng((seed ^ seed_offset().wrapping_mul(0x9E37_79B9_7F4A_7C15)) | 1)
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

    /// Uniform in `[-range, range)`.
    pub fn f(&mut self, range: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        (u * 2.0 - 1.0) * range
    }

    /// Uniform in `[0, range)`.
    pub fn fpos(&mut self, range: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        u * range
    }

    /// Quantized to a coarse grid so exact ties / touching cases occur often.
    pub fn grid(&mut self, step: f32, half_span: i32) -> f32 {
        let n = self.below((half_span * 2 + 1) as u32) as i32 - half_span;
        n as f32 * step
    }

    /// A random f32 built from raw bits, filtered to the "interesting" classes.
    /// Excludes NaN — NaN *payload* selection is covered separately by
    /// `wild_nan` / the `nan_payloads` tests, because it depends on which
    /// operand a commutative SSE op uses as its destination register.
    pub fn wild(&mut self) -> f32 {
        loop {
            let x = self.wild_nan();
            if !x.is_nan() {
                return x;
            }
        }
    }

    /// Like `wild`, but NaNs (of both signs) are part of the population.
    pub fn wild_nan(&mut self) -> f32 {
        const SPECIAL: [u32; 14] = [
            0x0000_0000, // +0
            0x8000_0000, // -0
            0x0000_0001, // smallest subnormal
            0x8000_0001, // -smallest subnormal
            0x007f_ffff, // largest subnormal
            0x0080_0000, // smallest normal
            0x3f80_0000, // 1.0
            0xbf80_0000, // -1.0
            0x7f7f_ffff, // FLT_MAX
            0xff7f_ffff, // -FLT_MAX
            0x7f80_0000, // +inf
            0xff80_0000, // -inf
            0x7fc0_0000, // qNaN
            0xffc0_0000, // -qNaN
        ];
        let r = self.next_u32();
        if r % 3 == 0 {
            f32::from_bits(SPECIAL[(r as usize >> 8) % SPECIAL.len()])
        } else if r % 3 == 1 {
            f32::from_bits(self.next_u32())
        } else {
            self.f(1000.0)
        }
    }

    pub fn vec(&mut self, range: f32) -> c2v {
        v(self.f(range), self.f(range))
    }

    pub fn wild_vec(&mut self) -> c2v {
        v(self.wild(), self.wild())
    }

    pub fn wild_nan_vec(&mut self) -> c2v {
        v(self.wild_nan(), self.wild_nan())
    }

    pub fn rot(&mut self) -> c2r {
        let a = self.f(std::f32::consts::PI);
        c2r {
            c: a.cos(),
            s: a.sin(),
        }
    }

    pub fn xform(&mut self, range: f32) -> c2x {
        c2x {
            p: self.vec(range),
            r: self.rot(),
        }
    }
}
