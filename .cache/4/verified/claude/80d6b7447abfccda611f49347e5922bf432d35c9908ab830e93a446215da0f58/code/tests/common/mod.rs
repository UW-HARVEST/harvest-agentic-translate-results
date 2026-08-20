//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both shared objects are loaded with `libloading` and every call goes through
//! `dlsym`, so the Rust side is exercised exactly as an external C consumer would
//! exercise it (including the `#[no_mangle] extern "C"` wrappers and the SysV
//! struct-passing ABI). Rust functions are never called directly.
#![allow(non_snake_case, non_camel_case_types, dead_code)]
// The `Default` impls for `c2Poly` / `c2Proxy` are written out rather than derived so
// that the zero-initialisation the C library's uninitialised locals are compared
// against is explicit here too.
#![allow(clippy::derivable_impls)]

use libloading::{Library, Symbol};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// repr(C) type mirrors -- deliberately re-declared here rather than imported,
// so the tests see only the ABI, never the Rust source.
// ---------------------------------------------------------------------------

pub type C2_TYPE = u32;
pub const C2_TYPE_CAPSULE: C2_TYPE = 0;
pub const C2_TYPE_CIRCLE: C2_TYPE = 1;
pub const C2_TYPE_AABB: C2_TYPE = 2;
pub const C2_TYPE_POLY: C2_TYPE = 3;

/// The three shape types the C `switch`es actually handle.
pub const VALID_TYPES: [C2_TYPE; 3] = [C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB];
/// All four declared enumerators.
pub const ALL_TYPES: [C2_TYPE; 4] =
    [C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_POLY];
/// Values with no valid variant -- a C enum accepts any `int` across FFI.
pub const BAD_TYPES: [C2_TYPE; 7] = [4, 5, 255, 256, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff];

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}
pub const fn v(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Manifold {
    pub count: i32,
    pub depths: [f32; 2],
    pub contact_points: [c2v; 2],
    pub n: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2h {
    pub n: c2v,
    pub d: f32,
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
pub fn x_identity() -> c2x {
    c2x { p: v(0.0, 0.0), r: c2r { c: 1.0, s: 0.0 } }
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
    pub count: i32,
    pub verts: [c2v; 8],
    pub norms: [c2v; 8],
}
impl Default for c2Poly {
    fn default() -> Self {
        c2Poly { count: 0, verts: [c2v::default(); 8], norms: [c2v::default(); 8] }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: i32,
    pub iA: [i32; 3],
    pub iB: [i32; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: i32,
    pub verts: [c2v; 8],
}
impl Default for c2Proxy {
    fn default() -> Self {
        c2Proxy { radius: 0.0, count: 0, verts: [c2v::default(); 8] }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: i32,
    pub iB: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Simplex {
    pub a: c2sv,
    pub b: c2sv,
    pub c: c2sv,
    pub d: c2sv,
    pub div: f32,
    pub count: i32,
}

pub const FLT_MAX: f32 = 3.402_823_5e38;
pub const FLT_EPSILON: f32 = 1.192_092_9e-7;

/// Layout assertions -- if these fail the whole differential comparison is
/// meaningless, so check them once up front.
pub fn assert_layouts() {
    use std::mem::size_of;
    assert_eq!(size_of::<c2v>(), 8);
    assert_eq!(size_of::<c2h>(), 12);
    assert_eq!(size_of::<c2r>(), 8);
    assert_eq!(size_of::<c2x>(), 16);
    assert_eq!(size_of::<c2Circle>(), 12);
    assert_eq!(size_of::<c2AABB>(), 16);
    assert_eq!(size_of::<c2Capsule>(), 20);
    assert_eq!(size_of::<c2Poly>(), 132);
    assert_eq!(size_of::<c2GJKCache>(), 36);
    assert_eq!(size_of::<c2Manifold>(), 36);
    assert_eq!(size_of::<c2Proxy>(), 72);
    assert_eq!(size_of::<c2sv>(), 36);
    assert_eq!(size_of::<c2Simplex>(), 152);
}

// ---------------------------------------------------------------------------
// Byte-exact comparison
// ---------------------------------------------------------------------------

/// Raw object representation of a value, for byte-for-byte comparison
/// (so `-0.0` vs `+0.0` and differing NaN payloads are *not* equal).
pub fn raw<T>(t: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(t as *const T as *const u8, std::mem::size_of::<T>()) }
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect::<Vec<_>>().join("")
}

#[track_caller]
pub fn eq<T: std::fmt::Debug>(what: &str, ctx: &str, c: &T, r: &T) {
    let (cb, rb) = (raw(c), raw(r));
    if cb != rb {
        panic!(
            "{what} DIVERGED\n  ctx       : {ctx}\n  C bytes   : {}\n  Rust bytes: {}\n  C   : {c:?}\n  Rust: {r:?}",
            hex(cb),
            hex(rb)
        );
    }
}

#[track_caller]
pub fn eq_f32(what: &str, ctx: &str, c: f32, r: f32) {
    if c.to_bits() != r.to_bits() {
        panic!(
            "{what} DIVERGED\n  ctx : {ctx}\n  C   : {c:?} (0x{:08x})\n  Rust: {r:?} (0x{:08x})",
            c.to_bits(),
            r.to_bits()
        );
    }
}

#[track_caller]
pub fn eq_i32(what: &str, ctx: &str, c: i32, r: i32) {
    if c != r {
        panic!("{what} DIVERGED\n  ctx : {ctx}\n  C   : {c}\n  Rust: {r}");
    }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_rust_so() -> PathBuf {
    // Explicit override wins, so the same tests can be pointed at any build.
    if let Ok(p) = std::env::var("OMNI_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "OMNI_RUST_SO points at a missing file: {p:?}");
        return p;
    }
    let root = crate_root();
    // Prefer the profile this test binary was itself built with, so
    // `cargo test --release` exercises the optimised cdylib rather than a stale
    // debug one (optimisation is exactly what could defeat `src/fp.rs`).
    let (first, second) = if cfg!(debug_assertions) {
        ("target/debug", "target/release")
    } else {
        ("target/release", "target/debug")
    };
    let candidates = [
        root.join(first).join("libomni_manifold_lib.so"),
        root.join(second).join("libomni_manifold_lib.so"),
    ];
    for c in candidates.iter() {
        if c.exists() {
            return c.clone();
        }
    }
    panic!("Rust cdylib not found. Run `cargo build` first. Looked in: {candidates:?}");
}

fn find_c_so() -> PathBuf {
    let p = crate_root().join("c_src/build/libtranslated_rust.so");
    assert!(
        p.exists(),
        "C .so not found at {p:?}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

pub struct Libs {
    pub c: Library,
    pub rs: Library,
}

impl Libs {
    pub fn load() -> Libs {
        assert_layouts();
        unsafe {
            let c = Library::new(find_c_so()).expect("dlopen C .so");
            let rs = Library::new(find_rust_so()).expect("dlopen Rust .so");
            Libs { c, rs }
        }
    }

    /// `(c_fn, rust_fn)` for one exported symbol.
    pub fn get<T>(&self, name: &str) -> (Symbol<'_, T>, Symbol<'_, T>) {
        let mut n = name.as_bytes().to_vec();
        n.push(0);
        unsafe {
            let cs: Symbol<T> =
                self.c.get(&n).unwrap_or_else(|e| panic!("C .so missing symbol `{name}`: {e}"));
            let rs: Symbol<T> =
                self.rs.get(&n).unwrap_or_else(|e| panic!("Rust .so missing symbol `{name}`: {e}"));
            (cs, rs)
        }
    }
}

/// One process-wide pair of loaded libraries.
pub fn libs() -> &'static Libs {
    use std::sync::OnceLock;
    static L: OnceLock<Libs> = OnceLock::new();
    L.get_or_init(Libs::load)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) + float generators
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
    pub fn bool(&mut self) -> bool {
        self.next_u32() & 1 == 1
    }
    /// Uniform in [-mag, mag], a "well behaved" finite float.
    pub fn f_norm(&mut self, mag: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
        (u * 2.0 - 1.0) * mag
    }
    /// Non-negative, in [0, mag].
    pub fn f_pos(&mut self, mag: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        u * mag
    }
    /// Small integer-valued float in [-n, n] -- makes exact ties common.
    pub fn f_lattice(&mut self, n: i32) -> f32 {
        (self.below((2 * n + 1) as u32) as i32 - n) as f32
    }
    /// Half-integer lattice, so "exactly touching" configurations occur often.
    pub fn f_half_lattice(&mut self, n: i32) -> f32 {
        (self.below((4 * n + 1) as u32) as i32 - 2 * n) as f32 * 0.5
    }
    /// Completely arbitrary 32-bit pattern reinterpreted as f32
    /// (covers +-inf, +-0, denormals, quiet and signalling NaNs of both signs).
    pub fn f_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A value drawn from the pool of "interesting" floats.
    pub fn f_special(&mut self) -> f32 {
        f32::from_bits(SPECIAL_BITS[self.below(SPECIAL_BITS.len() as u32) as usize])
    }
    /// Mostly well-behaved, occasionally pathological.
    pub fn f_mixed(&mut self, mag: f32) -> f32 {
        match self.below(10) {
            0 | 1 => self.f_special(),
            2 => self.f_bits(),
            3 => self.f_lattice(4),
            _ => self.f_norm(mag),
        }
    }

    pub fn vec_norm(&mut self, mag: f32) -> c2v {
        v(self.f_norm(mag), self.f_norm(mag))
    }
    pub fn vec_bits(&mut self) -> c2v {
        v(self.f_bits(), self.f_bits())
    }
    pub fn vec_mixed(&mut self, mag: f32) -> c2v {
        v(self.f_mixed(mag), self.f_mixed(mag))
    }
    pub fn vec_lattice(&mut self, n: i32) -> c2v {
        v(self.f_lattice(n), self.f_lattice(n))
    }
    pub fn vec_special(&mut self) -> c2v {
        v(self.f_special(), self.f_special())
    }
    /// A random `c2x`: sometimes identity, sometimes translation/rotation only,
    /// sometimes a non-unit or zero rotation.
    pub fn xform(&mut self, mag: f32) -> c2x {
        match self.below(6) {
            0 => x_identity(),
            1 => c2x { p: self.vec_norm(mag), r: c2r { c: 1.0, s: 0.0 } }, // translation only
            2 => {
                let t = self.f_pos(std::f32::consts::TAU);
                c2x { p: v(0.0, 0.0), r: c2r { c: t.cos(), s: t.sin() } } // rotation only
            }
            3 => c2x { p: self.vec_norm(mag), r: c2r { c: 0.0, s: 0.0 } }, // zero rotation
            4 => c2x { p: self.vec_norm(mag), r: c2r { c: 2.0, s: 3.0 } }, // non-unit
            _ => {
                let t = self.f_pos(std::f32::consts::TAU);
                c2x { p: self.vec_norm(mag), r: c2r { c: t.cos(), s: t.sin() } }
            }
        }
    }
}

pub const SPECIAL_BITS: [u32; 26] = [
    0x0000_0000, // +0.0
    0x8000_0000, // -0.0
    0x0000_0001, // smallest +denormal
    0x8000_0001, // smallest -denormal
    0x007f_ffff, // largest +denormal
    0x0080_0000, // smallest +normal
    0x3f80_0000, // 1.0
    0xbf80_0000, // -1.0
    0x3f00_0000, // 0.5
    0x4000_0000, // 2.0
    0x7f7f_ffff, // FLT_MAX
    0xff7f_ffff, // -FLT_MAX
    0x7f80_0000, // +inf
    0xff80_0000, // -inf
    0x7fc0_0000, // +qNaN default
    0xffc0_0000, // -qNaN default
    0x7fc0_1234, // +qNaN payload
    0xffc0_5678, // -qNaN payload
    0x7f80_0001, // +sNaN
    0xff80_0001, // -sNaN
    0x7fbf_ffff, // +sNaN max payload
    0x3400_0000, // FLT_EPSILON
    0x3580_0000, // ~1e-6
    0x322b_cc77, // 1e-8
    0x4b18_9680, // 1e7
    0x0800_0000, // tiny normal, squares to zero
];

// ---------------------------------------------------------------------------
// Poison patterns, so "field left untouched" is observable
// ---------------------------------------------------------------------------

fn poison<T: Default>(seed: u8, mul: u8, or: u8) -> T {
    let mut t = T::default();
    let bytes = unsafe {
        std::slice::from_raw_parts_mut(&mut t as *mut T as *mut u8, std::mem::size_of::<T>())
    };
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = seed.wrapping_add(i as u8).wrapping_mul(mul) | or;
    }
    t
}

pub fn poison_manifold(seed: u8) -> c2Manifold {
    poison(seed, 37, 0x11)
}
pub fn poison_proxy(seed: u8) -> c2Proxy {
    poison(seed, 101, 0x03)
}
pub fn poison_v(seed: u8) -> c2v {
    poison(seed, 53, 0x07)
}
pub fn poison_h(seed: u8) -> c2h {
    poison(seed, 71, 0x05)
}

// ---------------------------------------------------------------------------
// Shape builders
// ---------------------------------------------------------------------------

/// Convex CCW polygon with `count` vertices on a circle of radius `r`.
/// Normals are left zeroed; call [`fill_norms`].
pub fn convex_poly(rng: &mut Rng, count: i32, r: f32, center: c2v) -> c2Poly {
    let mut p = c2Poly::default();
    p.count = count;
    let phase = rng.f_pos(std::f32::consts::TAU);
    let n = count.clamp(1, 8) as usize;
    for i in 0..n {
        let t = phase + (i as f32) * std::f32::consts::TAU / (n as f32);
        p.verts[i] = v(center.x + r * t.cos(), center.y + r * t.sin());
    }
    p
}

/// Same but clockwise, so `c2Norms` produces inward normals.
pub fn concave_wound_poly(rng: &mut Rng, count: i32, r: f32, center: c2v) -> c2Poly {
    let mut p = convex_poly(rng, count, r, center);
    let n = count.clamp(1, 8) as usize;
    p.verts[..n].reverse();
    p
}

/// Fill `norms` using the C library's own `c2Norms` so both sides start identical.
pub fn fill_norms(p: &mut c2Poly) {
    type F = unsafe extern "C" fn(*mut c2v, *mut c2v, i32);
    let (cf, _) = libs().get::<F>("c2Norms");
    unsafe { cf(p.verts.as_mut_ptr(), p.norms.as_mut_ptr(), p.count) };
}

// ---------------------------------------------------------------------------
// Function-pointer type aliases for every exported symbol
// ---------------------------------------------------------------------------

use std::ffi::c_void;

pub type FnV_ff = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnV_v = unsafe extern "C" fn(c2v) -> c2v;
pub type FnV_vv = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnV_vf = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnV_vvv = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnV_vvff = unsafe extern "C" fn(c2v, c2v, f32, f32) -> c2v;
pub type FnF_v = unsafe extern "C" fn(c2v) -> f32;
pub type FnF_vv = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnF_hv = unsafe extern "C" fn(c2h, c2v) -> f32;
pub type FnH_polyi = unsafe extern "C" fn(*const c2Poly, i32) -> c2h;
pub type FnR_void = unsafe extern "C" fn() -> c2r;
pub type FnX_void = unsafe extern "C" fn() -> c2x;
pub type FnV_rv = unsafe extern "C" fn(c2r, c2v) -> c2v;
pub type FnV_xv = unsafe extern "C" fn(c2x, c2v) -> c2v;
pub type FnBBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const c_void, C2_TYPE, *mut c2Proxy);
pub type FnSupport = unsafe extern "C" fn(*const c2v, i32, c2v) -> i32;
pub type FnNorms = unsafe extern "C" fn(*mut c2v, *mut c2v, i32);
pub type FnSimplexF = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type FnSimplexV = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type FnSimplexVoid = unsafe extern "C" fn(*mut c2Simplex);
pub type FnWitness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
pub type FnGJK = unsafe extern "C" fn(
    *const c_void,
    C2_TYPE,
    *const c2x,
    *const c_void,
    C2_TYPE,
    *const c2x,
    *mut c2v,
    *mut c2v,
    i32,
    *mut i32,
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
    unsafe extern "C" fn(*const c_void, C2_TYPE, *const c_void, C2_TYPE, *mut c2Manifold);
pub type FnPtrFromParts =
    unsafe extern "C" fn(C2_TYPE, f32, f32, f32, f32, f32) -> *mut c_void;
pub type FnOmni = unsafe extern "C" fn(
    *mut c2Manifold,
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
);

// ---------------------------------------------------------------------------
// Making the C library's uninitialised `c2Proxy` read deterministic
// ---------------------------------------------------------------------------

/// Zero 8 KiB of stack immediately below the current frame.
///
/// 8 KiB is ~6x the deepest C call chain: `omni_manifold` -> `c2Collide` ->
/// `c2AABBtoCapsuleManifold` (0xb8) -> `c2CapsuletoPolyManifold` (0x138) ->
/// `c2GJK` (0x288, the largest frame in the library) -> leaf helpers, about
/// 1.3 KiB in total.
///
/// `c2GJK` declares `c2Proxy pA, pB;` without an initialiser and `c2MakeProxy` has no
/// `C2_TYPE_POLY` case, so on every polygon path the C library reads whatever the
/// stack happens to hold (see `tests/probe_uninit.rs`: normally a stack address, and
/// in a fresh minimal process a garbage `count` that segfaults).
///
/// Calling this immediately before a C entry point that reaches the polygon path
/// forces that region to all-zero, which is exactly the model `src/gjk.rs`
/// implements: a POLY operand behaves as a single point at the origin with radius 0.
/// That makes the C side a *deterministic function of its inputs* again and lets the
/// polygon paths -- and with them `c2Clip`, `c2SidePlanes`, `c2SidePlanesFromPoly`,
/// `c2KeepDeep` and `c2Incident`, which are unreachable any other way -- be compared
/// byte-for-byte. Verified in
/// `tests/probe_uninit.rs::zero_stack_makes_c_agree_with_rust`.
///
/// Call it before *both* the C and the Rust invocation so the two are symmetric.
///
/// **It must be the LAST statement before the FFI call.** Anything invoked in
/// between -- even a tiny helper like `poison_v` -- gets a stack frame in the very
/// region that was just zeroed and dirties it again. Every wrapper in this suite
/// therefore has the shape: prepare the locals, `zero_stack()`, then the `unsafe`
/// call and nothing else.
#[inline(never)]
pub fn zero_stack() {
    let mut buf = [0u64; 1024];
    for i in 0..buf.len() {
        unsafe { std::ptr::write_volatile(buf.as_mut_ptr().add(i), 0) };
    }
    std::hint::black_box(buf.as_ptr());
}

/// Run `f` a few times to force lazy PLT resolution inside the shared objects, so
/// that `_dl_runtime_resolve` does not dirty the stack during measured calls.
pub fn warmup(mut f: impl FnMut()) {
    for _ in 0..8 {
        f();
    }
}
