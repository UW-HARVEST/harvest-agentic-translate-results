//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` with `libloading` and exposes each
//! exported symbol as a raw `extern "C"` function pointer. No Rust function is
//! ever called directly — every call goes through the dynamic-library export,
//! so the `#[no_mangle]` wrappers and the ABI are under test too.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// C-layout types (mirrors of the definitions in c_src/src/lib.c)
// ---------------------------------------------------------------------------

pub type C2_TYPE = c_int;
pub const C2_TYPE_CAPSULE: C2_TYPE = 0;
pub const C2_TYPE_CIRCLE: C2_TYPE = 1;
pub const C2_TYPE_AABB: C2_TYPE = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
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
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

/// `typedef struct { float radius; int count; c2v verts[8]; } c2Proxy;`
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

/// `typedef struct { c2v sA; c2v sB; c2v p; float u; int iA; int iB; } c2sv;`
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

/// `typedef struct { c2sv a, b, c, d; float div; int count; } c2Simplex;`
///
/// The four `c2sv` members are contiguous (36 bytes each, align 4), so an
/// array is layout-identical to the four named fields.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

// ---------------------------------------------------------------------------
// Function-pointer table
// ---------------------------------------------------------------------------

pub struct Api {
    pub tag: &'static str,

    // level 0
    pub c2V: extern "C" fn(f32, f32) -> c2v,
    pub c2Mulvs: extern "C" fn(c2v, f32) -> c2v,
    pub c2Maxv: extern "C" fn(c2v, c2v) -> c2v,
    pub c2Minv: extern "C" fn(c2v, c2v) -> c2v,
    pub c2Clampv: extern "C" fn(c2v, c2v, c2v) -> c2v,
    pub c2Sub: extern "C" fn(c2v, c2v) -> c2v,
    pub c2Add: extern "C" fn(c2v, c2v) -> c2v,
    pub c2Dot: extern "C" fn(c2v, c2v) -> f32,
    pub c2Det2: extern "C" fn(c2v, c2v) -> f32,
    pub c2Len: extern "C" fn(c2v) -> f32,
    pub c2Neg: extern "C" fn(c2v) -> c2v,
    pub c2Skew: extern "C" fn(c2v) -> c2v,
    pub c2CCW90: extern "C" fn(c2v) -> c2v,
    pub c2Div: extern "C" fn(c2v, f32) -> c2v,
    pub c2Norm: extern "C" fn(c2v) -> c2v,
    pub c2Mulrv: extern "C" fn(c2r, c2v) -> c2v,
    pub c2MulrvT: extern "C" fn(c2r, c2v) -> c2v,
    pub c2Mulxv: extern "C" fn(c2x, c2v) -> c2v,
    pub c2RotIdentity: extern "C" fn() -> c2r,
    pub c2xIdentity: extern "C" fn() -> c2x,

    // level 1
    pub c2BBVerts: unsafe extern "C" fn(*mut c2v, *mut c2AABB),
    pub c2MakeProxy: unsafe extern "C" fn(*const c_void, C2_TYPE, *mut c2Proxy),

    // level 2
    pub c2GJKSimplexMetric: unsafe extern "C" fn(*mut c2Simplex) -> f32,
    pub c22: unsafe extern "C" fn(*mut c2Simplex),
    pub c23: unsafe extern "C" fn(*mut c2Simplex),
    pub c2D: unsafe extern "C" fn(*mut c2Simplex) -> c2v,
    pub c2L: unsafe extern "C" fn(*mut c2Simplex) -> c2v,
    pub c2Witness: unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v),
    pub c2Support: unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int,

    // level 3
    pub c2GJK: unsafe extern "C" fn(
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
    ) -> f32,

    // level 4
    pub c2AABBtoAABB: extern "C" fn(c2AABB, c2AABB) -> c_int,
    pub c2AABBtoCapsule: extern "C" fn(c2AABB, c2Capsule) -> c_int,
    pub c2CapsuletoCapsule: extern "C" fn(c2Capsule, c2Capsule) -> c_int,
    pub c2CircletoCircle: extern "C" fn(c2Circle, c2Circle) -> c_int,
    pub c2CircletoAABB: extern "C" fn(c2Circle, c2AABB) -> c_int,
    pub c2CircletoCapsule: extern "C" fn(c2Circle, c2Capsule) -> c_int,

    // level 5/6
    pub c2Collided: unsafe extern "C" fn(*const c_void, C2_TYPE, *const c_void, C2_TYPE) -> c_int,
    pub ptr_from_parts: unsafe extern "C" fn(C2_TYPE, f32, f32, f32, f32, f32) -> *mut c_void,
    pub omni_collide: unsafe extern "C" fn(
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
    ) -> c_int,
}

unsafe fn sym<T: Copy>(lib: &libloading::Library, name: &[u8]) -> T {
    let s: libloading::Symbol<T> = unsafe {
        lib.get(name)
            .unwrap_or_else(|e| panic!("symbol {} not found: {e}", String::from_utf8_lossy(name)))
    };
    unsafe { *s.into_raw() }
}

impl Api {
    fn load(tag: &'static str, path: &Path) -> Api {
        // Leaked on purpose: the raw function pointers below must stay valid
        // for the whole process lifetime.
        let lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
            libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()))
        }));
        unsafe {
            Api {
                tag,
                c2V: sym(lib, b"c2V\0"),
                c2Mulvs: sym(lib, b"c2Mulvs\0"),
                c2Maxv: sym(lib, b"c2Maxv\0"),
                c2Minv: sym(lib, b"c2Minv\0"),
                c2Clampv: sym(lib, b"c2Clampv\0"),
                c2Sub: sym(lib, b"c2Sub\0"),
                c2Add: sym(lib, b"c2Add\0"),
                c2Dot: sym(lib, b"c2Dot\0"),
                c2Det2: sym(lib, b"c2Det2\0"),
                c2Len: sym(lib, b"c2Len\0"),
                c2Neg: sym(lib, b"c2Neg\0"),
                c2Skew: sym(lib, b"c2Skew\0"),
                c2CCW90: sym(lib, b"c2CCW90\0"),
                c2Div: sym(lib, b"c2Div\0"),
                c2Norm: sym(lib, b"c2Norm\0"),
                c2Mulrv: sym(lib, b"c2Mulrv\0"),
                c2MulrvT: sym(lib, b"c2MulrvT\0"),
                c2Mulxv: sym(lib, b"c2Mulxv\0"),
                c2RotIdentity: sym(lib, b"c2RotIdentity\0"),
                c2xIdentity: sym(lib, b"c2xIdentity\0"),
                c2BBVerts: sym(lib, b"c2BBVerts\0"),
                c2MakeProxy: sym(lib, b"c2MakeProxy\0"),
                c2GJKSimplexMetric: sym(lib, b"c2GJKSimplexMetric\0"),
                c22: sym(lib, b"c22\0"),
                c23: sym(lib, b"c23\0"),
                c2D: sym(lib, b"c2D\0"),
                c2L: sym(lib, b"c2L\0"),
                c2Witness: sym(lib, b"c2Witness\0"),
                c2Support: sym(lib, b"c2Support\0"),
                c2GJK: sym(lib, b"c2GJK\0"),
                c2AABBtoAABB: sym(lib, b"c2AABBtoAABB\0"),
                c2AABBtoCapsule: sym(lib, b"c2AABBtoCapsule\0"),
                c2CapsuletoCapsule: sym(lib, b"c2CapsuletoCapsule\0"),
                c2CircletoCircle: sym(lib, b"c2CircletoCircle\0"),
                c2CircletoAABB: sym(lib, b"c2CircletoAABB\0"),
                c2CircletoCapsule: sym(lib, b"c2CircletoCapsule\0"),
                c2Collided: sym(lib, b"c2Collided\0"),
                ptr_from_parts: sym(lib, b"ptr_from_parts\0"),
                omni_collide: sym(lib, b"omni_collide\0"),
            }
        }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}); build the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    let root = workspace_root().join("translation/target");
    // Prefer the profile the test binary itself was built with, then fall back.
    let mut tried = Vec::new();
    for profile in ["release", "debug"] {
        let p = root.join(profile).join("libomni_collide_lib.so");
        if p.exists() {
            return p;
        }
        tried.push(p);
    }
    panic!(
        "Rust cdylib not found; run `cargo build --release` in translation/. Tried: {:?}",
        tried
    );
}

static C_API: OnceLock<Api> = OnceLock::new();
static RUST_API: OnceLock<Api> = OnceLock::new();

pub fn c_api() -> &'static Api {
    C_API.get_or_init(|| Api::load("C", &find_c_so()))
}

pub fn rust_api() -> &'static Api {
    RUST_API.get_or_init(|| Api::load("Rust", &find_rust_so()))
}

/// Both implementations, ready for differential calls.
pub fn apis() -> (&'static Api, &'static Api) {
    (c_api(), rust_api())
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Differential float comparison.
///
/// Bit-exact — so `+0.0 != -0.0`, `+inf != -inf`, and NaN never equals a
/// number — with exactly one tolerance: **two NaNs count as equal even if
/// their payload/sign bits differ**. See [`NAN_PAYLOAD_NOTE`].
#[track_caller]
pub fn eq_f32(ctx: &str, c: f32, r: f32) {
    if c.is_nan() && r.is_nan() {
        return;
    }
    if c.to_bits() != r.to_bits() {
        panic!(
            "{ctx}: f32 mismatch\n  C    = {c:?} (bits 0x{:08x})\n  Rust = {r:?} (bits 0x{:08x})",
            c.to_bits(),
            r.to_bits()
        );
    }
}

/// Fully bit-exact, NaN payloads included. Used where no NaN can arise.
#[track_caller]
pub fn eq_f32_bits(ctx: &str, c: f32, r: f32) {
    if c.to_bits() != r.to_bits() {
        panic!(
            "{ctx}: f32 bit mismatch\n  C    = {c:?} (0x{:08x})\n  Rust = {r:?} (0x{:08x})",
            c.to_bits(),
            r.to_bits()
        );
    }
}

/// The single documented tolerance in this test suite.
///
/// # The divergence
/// gcc at `-O0` and LLVM pick **opposite SSE operand orders** for the same C
/// expression. For `a.x += b.x` gcc emits `addss %xmm1,%xmm0` with `b.x` in
/// `xmm0`, i.e. it computes `b + a` so **`b` is `src1`**; LLVM emits
/// `addps %xmm1,%xmm0`, i.e. `a + b` so **`a` is `src1`**. x86 SSE returns
/// `src1` whenever *both* operands are NaN, so the two builds propagate a
/// different input NaN. The same applies to the internally generated NaNs:
/// `0.0 * inf` raises invalid-operation and yields the x86 "QNaN indefinite"
/// `0xffc00000`, which then meets a propagated `0x7fc00000` in the following
/// add inside `c2Dot` / `c2Det2` / `c2Mulrv`.
///
/// # Why it is not fixed in the Rust
/// LLVM canonicalizes commutative `fadd`/`fmul` operand order. Writing
/// `b.x + a.x` instead of `a.x + b.x` produces *byte-identical* machine code —
/// verified by compiling both spellings and observing that LLVM merged them to
/// the same address (`nm` shows two names, one address). Matching gcc `-O0`
/// would require per-expression inline assembly throughout, and would then
/// stop matching a C build at any other optimization level.
///
/// # Why it is unobservable
/// A NaN payload never affects a comparison: every `<` / `>` / `<=` in
/// `c_src/src/lib.c` is false when either side is NaN, whatever the payload.
/// So no branch, no `int` return, and no non-NaN float can depend on it.
/// `tests/nan_payload.rs` asserts exactly that: with arbitrary NaN payloads and
/// signaling NaNs on the input, every integer result is bit-identical and every
/// float result is either bit-identical or NaN on both sides.
pub const NAN_PAYLOAD_NOTE: &str = "two NaNs compare equal regardless of payload; see NAN_PAYLOAD_NOTE";

#[track_caller]
pub fn eq_v(ctx: &str, c: c2v, r: c2v) {
    eq_f32(&format!("{ctx}.x"), c.x, r.x);
    eq_f32(&format!("{ctx}.y"), c.y, r.y);
}

#[track_caller]
pub fn eq_r(ctx: &str, c: c2r, r: c2r) {
    eq_f32(&format!("{ctx}.c"), c.c, r.c);
    eq_f32(&format!("{ctx}.s"), c.s, r.s);
}

#[track_caller]
pub fn eq_x(ctx: &str, c: c2x, r: c2x) {
    eq_v(&format!("{ctx}.p"), c.p, r.p);
    eq_r(&format!("{ctx}.r"), c.r, r.r);
}

#[track_caller]
pub fn eq_int(ctx: &str, c: c_int, r: c_int) {
    if c != r {
        panic!("{ctx}: int mismatch\n  C = {c}\n  Rust = {r}");
    }
}

/// Raw-bytes comparison — the strictest possible check for a POD struct.
#[track_caller]
pub fn eq_bytes<T>(ctx: &str, c: &T, r: &T) {
    let n = std::mem::size_of::<T>();
    let cb = unsafe { std::slice::from_raw_parts(c as *const T as *const u8, n) };
    let rb = unsafe { std::slice::from_raw_parts(r as *const T as *const u8, n) };
    if cb != rb {
        panic!(
            "{ctx}: {n}-byte struct mismatch\n  C    = {:02x?}\n  Rust = {:02x?}\n  \
             first differing byte at offset {}",
            cb,
            rb,
            cb.iter().zip(rb).position(|(a, b)| a != b).unwrap()
        );
    }
}

#[track_caller]
pub fn eq_simplex(ctx: &str, c: &c2Simplex, r: &c2Simplex) {
    for i in 0..4 {
        let cv = &c.verts[i];
        let rv = &r.verts[i];
        eq_v(&format!("{ctx}.verts[{i}].sA"), cv.sA, rv.sA);
        eq_v(&format!("{ctx}.verts[{i}].sB"), cv.sB, rv.sB);
        eq_v(&format!("{ctx}.verts[{i}].p"), cv.p, rv.p);
        eq_f32(&format!("{ctx}.verts[{i}].u"), cv.u, rv.u);
        eq_int(&format!("{ctx}.verts[{i}].iA"), cv.iA, rv.iA);
        eq_int(&format!("{ctx}.verts[{i}].iB"), cv.iB, rv.iB);
    }
    eq_f32(&format!("{ctx}.div"), c.div, r.div);
    eq_int(&format!("{ctx}.count"), c.count, r.count);
}

#[track_caller]
pub fn eq_cache(ctx: &str, c: &c2GJKCache, r: &c2GJKCache) {
    eq_f32(&format!("{ctx}.metric"), c.metric, r.metric);
    eq_int(&format!("{ctx}.count"), c.count, r.count);
    for i in 0..3 {
        eq_int(&format!("{ctx}.iA[{i}]"), c.iA[i], r.iA[i]);
        eq_int(&format!("{ctx}.iB[{i}]"), c.iB[i], r.iB[i]);
    }
    eq_f32(&format!("{ctx}.div"), c.div, r.div);
}

#[track_caller]
pub fn eq_proxy(ctx: &str, c: &c2Proxy, r: &c2Proxy) {
    eq_f32(&format!("{ctx}.radius"), c.radius, r.radius);
    eq_int(&format!("{ctx}.count"), c.count, r.count);
    for i in 0..8 {
        eq_v(&format!("{ctx}.verts[{i}]"), c.verts[i], r.verts[i]);
    }
}

/// The canonical quiet NaN (`0x7fc00000`) that real IEEE-754 operations
/// produce (`0.0/0.0`, `inf - inf`, `sqrt(-1)`, …).
pub const CANON_NAN: f32 = f32::from_bits(0x7fc0_0000);

/// Collapse any NaN to the canonical payload.
///
/// # Why
/// gcc at `-O0` and LLVM choose *opposite* SSE operand orders for the same C
/// expression: gcc emits `addss %xmm1,%xmm0` for `a.x += b.x` (so `b` is
/// `src1`), while LLVM emits `addps %xmm1,%xmm0` (so `a` is `src1`). x86 SSE
/// returns `src1` when **both** operands are NaN, so the two builds propagate
/// a *different input NaN* — a difference in payload/sign bits only. LLVM
/// canonicalizes commutative `fadd`/`fmul` operand order and even merges
/// `b + a` with `a + b` into one function (verified: both compile to the same
/// address), so this is not fixable from Rust source without inline asm, and it
/// changes with the C compiler's optimization level too.
///
/// The difference is unobservable in every value the library actually decides
/// on: a NaN makes every `<`/`>` in the source false regardless of payload, so
/// all `int` results and all non-NaN float results are bit-identical. That
/// claim is asserted directly by `tests/nan_payload.rs`; the other suites feed
/// canonical NaNs so they can keep a strict bit-exact comparison.
pub fn canon_nan(v: f32) -> f32 {
    if v.is_nan() { CANON_NAN } else { v }
}

/// Bit-exact, except that any two NaNs count as equal. Used only by
/// `tests/nan_payload.rs`.
#[track_caller]
pub fn eq_f32_nan_ok(ctx: &str, c: f32, r: f32) {
    if c.is_nan() && r.is_nan() {
        return;
    }
    eq_f32(ctx, c, r);
}

#[track_caller]
pub fn eq_v_nan_ok(ctx: &str, c: c2v, r: c2v) {
    eq_f32_nan_ok(&format!("{ctx}.x"), c.x, r.x);
    eq_f32_nan_ok(&format!("{ctx}.y"), c.y, r.y);
}

// ---------------------------------------------------------------------------
// Deterministic RNG (PCG32) — fixed seed, reproducible across runs
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
        r.state = r.state.wrapping_add(0x853c_49e6_748f_ea9b ^ seed);
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

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in `[-lim, lim)`.
    pub fn range(&mut self, lim: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * lim
    }

    /// A "well behaved" coordinate: moderate magnitude, sometimes exactly 0.
    pub fn coord(&mut self) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => self.range(0.001),
            3 => self.range(1000.0),
            _ => self.range(10.0),
        }
    }

    /// A radius: mostly small positive, sometimes 0 or negative (the C never
    /// validates the sign).
    pub fn radius(&mut self) -> f32 {
        match self.below(12) {
            0 => 0.0,
            1 => -self.unit() * 5.0,
            2 => self.unit() * 100.0,
            _ => self.unit() * 5.0,
        }
    }

    /// Full float-class spread: normals, zeros, subnormals, huge, inf, NaN.
    /// NaNs are canonicalized — see [`canon_nan`].
    pub fn wild(&mut self) -> f32 {
        canon_nan(self.wild_any_nan())
    }

    /// Like [`Rng::wild`] but keeps arbitrary NaN payloads (incl. signaling
    /// NaNs and negative NaNs). Only `tests/nan_payload.rs` uses this.
    pub fn wild_any_nan(&mut self) -> f32 {
        match self.below(20) {
            0 => f32::NAN,
            1 => -f32::NAN,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => 0.0,
            5 => -0.0,
            6 => f32::from_bits(1), // smallest subnormal
            7 => -f32::from_bits(1),
            8 => f32::MIN_POSITIVE,
            9 => f32::MAX,
            10 => f32::MIN,
            11 => 1e30,
            12 => -1e30,
            13 => f32::EPSILON,
            14 => -f32::EPSILON,
            15 => f32::from_bits(self.next_u32()), // any bit pattern at all
            _ => self.range(100.0),
        }
    }

    pub fn wild_v(&mut self) -> c2v {
        c2v {
            x: self.wild(),
            y: self.wild(),
        }
    }

    pub fn wild_v_any_nan(&mut self) -> c2v {
        c2v {
            x: self.wild_any_nan(),
            y: self.wild_any_nan(),
        }
    }

    pub fn v(&mut self) -> c2v {
        c2v {
            x: self.coord(),
            y: self.coord(),
        }
    }

    /// A `c2r`: unit rotations, plus the non-unit / zero / NaN cases the C
    /// happily accepts because it never normalizes.
    pub fn rot(&mut self) -> c2r {
        match self.below(10) {
            0 => c2r { c: 1.0, s: 0.0 },
            1 => c2r { c: 0.0, s: 0.0 },
            2 => c2r {
                c: self.range(3.0),
                s: self.range(3.0),
            },
            3 => c2r {
                c: f32::NAN,
                s: self.range(1.0),
            },
            _ => {
                let a = self.unit() * std::f32::consts::TAU;
                c2r {
                    c: a.cos(),
                    s: a.sin(),
                }
            }
        }
    }

    pub fn xform(&mut self) -> c2x {
        c2x {
            p: self.v(),
            r: self.rot(),
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
        match self.below(8) {
            // degenerate: min == max
            0 => c2AABB { min: a, max: a },
            // inverted: min > max (the C never validates)
            1 => c2AABB {
                min: c2v {
                    x: a.x.max(b.x),
                    y: a.y.max(b.y),
                },
                max: c2v {
                    x: a.x.min(b.x),
                    y: a.y.min(b.y),
                },
            },
            // fully unconstrained
            2 => c2AABB { min: a, max: b },
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
        match self.below(8) {
            // degenerate point capsule: a == b  =>  n == (0,0)
            0 => c2Capsule {
                a,
                b: a,
                r: self.radius(),
            },
            _ => c2Capsule {
                a,
                b: self.v(),
                r: self.radius(),
            },
        }
    }

    pub fn simplex_vert(&mut self) -> c2sv {
        c2sv {
            sA: self.v(),
            sB: self.v(),
            p: self.v(),
            u: self.coord(),
            iA: self.below(8) as c_int,
            iB: self.below(8) as c_int,
        }
    }
}

pub const SEED: u64 = 0x00c0_ffee_1234_5678;

// ---------------------------------------------------------------------------
// Shape-as-bytes helper: lets us hand the same raw buffer to both libraries
// (including for out-of-range type tags, where the C reads nothing at all).
// ---------------------------------------------------------------------------

/// A 32-byte zero-padded buffer, big enough for any of the three shapes, so
/// out-of-range type tags never read indeterminate memory on either side.
#[repr(C, align(8))]
pub struct ShapeBuf(pub [u8; 32]);

impl ShapeBuf {
    pub fn zeroed() -> ShapeBuf {
        ShapeBuf([0u8; 32])
    }
    pub fn from_circle(c: c2Circle) -> ShapeBuf {
        let mut b = ShapeBuf::zeroed();
        unsafe { std::ptr::write(b.0.as_mut_ptr() as *mut c2Circle, c) };
        b
    }
    pub fn from_aabb(a: c2AABB) -> ShapeBuf {
        let mut b = ShapeBuf::zeroed();
        unsafe { std::ptr::write(b.0.as_mut_ptr() as *mut c2AABB, a) };
        b
    }
    pub fn from_capsule(c: c2Capsule) -> ShapeBuf {
        let mut b = ShapeBuf::zeroed();
        unsafe { std::ptr::write(b.0.as_mut_ptr() as *mut c2Capsule, c) };
        b
    }
    pub fn as_ptr(&self) -> *const c_void {
        self.0.as_ptr() as *const c_void
    }
}

/// The three valid `C2_TYPE` values, in declaration order.
pub const VALID_TYPES: [C2_TYPE; 3] = [C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB];

/// Out-of-range `C2_TYPE` values. A C enum accepts any `int`, so these are
/// real inputs the library must handle identically.
pub const BAD_TYPES: [C2_TYPE; 8] = [3, 4, 100, -1, -2, i32::MAX, i32::MIN, 0x7fff_0000];

pub fn type_name(t: C2_TYPE) -> &'static str {
    match t {
        C2_TYPE_CAPSULE => "CAPSULE",
        C2_TYPE_CIRCLE => "CIRCLE",
        C2_TYPE_AABB => "AABB",
        _ => "INVALID",
    }
}

/// Build a random shape of the given type as a byte buffer.
pub fn rand_shape(rng: &mut Rng, t: C2_TYPE) -> ShapeBuf {
    match t {
        C2_TYPE_CIRCLE => ShapeBuf::from_circle(rng.circle()),
        C2_TYPE_AABB => ShapeBuf::from_aabb(rng.aabb()),
        C2_TYPE_CAPSULE => ShapeBuf::from_capsule(rng.capsule()),
        _ => ShapeBuf::zeroed(),
    }
}
